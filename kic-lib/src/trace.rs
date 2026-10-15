use serde::Serialize;
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, SyncSender, TrySendError};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use base64::Engine;

/// Maximum number of source payload bytes retained in a trace event.
pub const MAX_TRACE_PAYLOAD_BYTES: usize = 64 * 1024;
/// Maximum encoded payload size accepted by the trace queue.
pub const MAX_TRACE_DATA_BYTES: usize = ((MAX_TRACE_PAYLOAD_BYTES + 2) / 3) * 4;
/// Maximum number of events retained by a trace queue.
pub const MAX_TRACE_QUEUE_CAPACITY: usize = 1024;

/// Direction of a successful physical instrument I/O operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TraceDirection {
    Read,
    Write,
}

/// Transport used by a trace event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TraceProtocol {
    Lan,
    Tls,
    Visa,
}

/// Encoding used for the event payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TracePayloadEncoding {
    Utf8,
    Base64,
}

static SESSION_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Immutable identity for one CLI trace session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceSession {
    id: String,
}

impl TraceSession {
    /// Preserve a caller-provided ID, or generate a unique process-local fallback.
    #[must_use]
    pub fn new(caller_id: Option<&str>) -> Self {
        let id = caller_id
            .filter(|value| !value.is_empty())
            .map_or_else(generate_session_id, ToOwned::to_owned);
        Self { id }
    }

    /// Return the session ID used on trace events.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }
}

fn generate_session_id() -> String {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let sequence = SESSION_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!(
        "session-{}-{}-{}",
        elapsed.as_secs(),
        elapsed.subsec_nanos(),
        sequence
    )
}

/// Encoded payload data and the byte length of the original transport payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TracePayload {
    pub byte_length: usize,
    pub encoding: TracePayloadEncoding,
    pub data: String,
}

impl TracePayload {
    /// Encode bytes as UTF-8 when possible and Base64 otherwise.
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let bounded_bytes = &bytes[..bytes.len().min(MAX_TRACE_PAYLOAD_BYTES)];
        match std::str::from_utf8(bounded_bytes) {
            Ok(text) => Self {
                byte_length: bytes.len(),
                encoding: TracePayloadEncoding::Utf8,
                data: text.to_owned(),
            },
            Err(_) => Self {
                byte_length: bytes.len(),
                encoding: TracePayloadEncoding::Base64,
                data: base64::engine::general_purpose::STANDARD.encode(bounded_bytes),
            },
        }
    }
}

/// A session-correlated physical instrument I/O event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TraceEvent {
    pub timestamp: String,
    pub session_id: String,
    pub direction: TraceDirection,
    pub protocol: TraceProtocol,
    pub address: Option<String>,
    pub byte_length: usize,
    pub encoding: TracePayloadEncoding,
    pub data: String,
}

impl TraceEvent {
    /// Serialize this event as one JSON Lines record.
    pub fn to_json_line(&self) -> Result<String, serde_json::Error> {
        let mut line = serde_json::to_string(self)?;
        line.push('\n');
        Ok(line)
    }
}

type TraceHandler = Box<dyn Fn(TraceEvent) + Send + 'static>;

/// Bounded, asynchronous handoff for trace events.
pub struct TraceDispatcher {
    sender: Option<SyncSender<TraceEvent>>,
    worker: Option<JoinHandle<()>>,
    diagnostics: Arc<TraceDiagnostics>,
}

impl TraceDispatcher {
    /// Start a worker that receives at most `capacity` pending events.
    pub fn new(
        capacity: usize,
        handler: impl Fn(TraceEvent) + Send + 'static,
    ) -> std::io::Result<Self> {
        Self::new_with_diagnostics(capacity, handler, Arc::new(TraceDiagnostics::default()))
    }

    /// Start a dispatcher using caller-owned diagnostics shared with destinations.
    pub fn new_with_diagnostics(
        capacity: usize,
        handler: impl Fn(TraceEvent) + Send + 'static,
        diagnostics: Arc<TraceDiagnostics>,
    ) -> std::io::Result<Self> {
        assert!(
            capacity > 0,
            "trace queue capacity must be greater than zero"
        );
        let (sender, receiver) = mpsc::sync_channel(capacity.min(MAX_TRACE_QUEUE_CAPACITY));
        let worker = std::thread::Builder::new()
            .name("instrument-trace".to_owned())
            .spawn(move || run_trace_worker(receiver, Box::new(handler)))?;

        Ok(Self {
            sender: Some(sender),
            worker: Some(worker),
            diagnostics,
        })
    }

    /// Enqueue an event without waiting for the worker or its handler.
    pub fn try_enqueue(&self, event: TraceEvent) -> Result<(), TraceEvent> {
        if event.data.len() > MAX_TRACE_DATA_BYTES {
            self.diagnostics.record_dropped_event();
            return Err(event);
        }

        let Some(sender) = &self.sender else {
            return Err(event);
        };

        match sender.try_send(event) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(event) | TrySendError::Disconnected(event)) => {
                self.diagnostics.record_dropped_event();
                Err(event)
            }
        }
    }

    /// Return counters for non-fatal trace delivery diagnostics.
    #[must_use]
    pub fn diagnostics(&self) -> Arc<TraceDiagnostics> {
        Arc::clone(&self.diagnostics)
    }
}

impl Drop for TraceDispatcher {
    fn drop(&mut self) {
        self.sender.take();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn run_trace_worker(receiver: mpsc::Receiver<TraceEvent>, handler: TraceHandler) {
    while let Ok(event) = receiver.recv() {
        handler(event);
    }
}

/// Non-fatal counters for trace delivery failures and dropped events.
#[derive(Default)]
pub struct TraceDiagnostics {
    dropped_events: std::sync::atomic::AtomicUsize,
    destination_failures: std::sync::atomic::AtomicUsize,
    last_report: Mutex<Option<Instant>>,
}

impl TraceDiagnostics {
    /// Return the number of events rejected by a full or disconnected queue.
    #[must_use]
    pub fn dropped_event_count(&self) -> usize {
        self.dropped_events.load(Ordering::Relaxed)
    }

    /// Return the number of destination writes that failed.
    #[must_use]
    pub fn destination_failure_count(&self) -> usize {
        self.destination_failures.load(Ordering::Relaxed)
    }

    fn record_dropped_event(&self) {
        let count = self.dropped_events.fetch_add(1, Ordering::Relaxed) + 1;
        if self.should_report() {
            tracing::warn!(
                dropped = count,
                "trace event dropped because delivery queue is unavailable"
            );
        }
    }

    fn record_destination_failure(&self, error: &io::Error) {
        let count = self.destination_failures.fetch_add(1, Ordering::Relaxed) + 1;
        if self.should_report() {
            tracing::warn!(error = %error, failures = count, "trace destination disabled after write failure");
        }
    }

    fn should_report(&self) -> bool {
        let Ok(mut last_report) = self.last_report.lock() else {
            return false;
        };
        let now = Instant::now();
        if last_report.is_none_or(|last| now.duration_since(last) >= Duration::from_secs(1)) {
            *last_report = Some(now);
            true
        } else {
            false
        }
    }
}

enum TraceDestination {
    File(Mutex<Option<File>>),
    Socket(Mutex<Option<TcpStream>>),
}

/// JSON Lines destinations served by a trace worker.
pub struct TraceDestinations {
    destinations: Vec<TraceDestination>,
}

impl TraceDestinations {
    /// Create an empty collection of trace destinations.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            destinations: Vec::new(),
        }
    }

    /// Append JSON Lines records to a file destination.
    pub fn add_file(&mut self, path: impl AsRef<Path>) -> io::Result<()> {
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        self.destinations
            .push(TraceDestination::File(Mutex::new(Some(file))));
        Ok(())
    }

    /// Connect to a TCP trace consumer and add it as a JSON Lines destination.
    pub fn add_socket(&mut self, address: impl ToSocketAddrs) -> io::Result<()> {
        let stream = TcpStream::connect(address)?;
        self.destinations
            .push(TraceDestination::Socket(Mutex::new(Some(stream))));
        Ok(())
    }

    /// Write one JSON Lines record to every configured destination.
    pub fn write_event(&self, event: &TraceEvent) -> Vec<io::Result<()>> {
        let line = match event.to_json_line() {
            Ok(line) => line,
            Err(error) => return vec![Err(io::Error::other(error))],
        };

        self.destinations
            .iter()
            .map(|destination| match destination {
                TraceDestination::File(file) => write_line(file, line.as_bytes()),
                TraceDestination::Socket(socket) => write_line(socket, line.as_bytes()),
            })
            .collect()
    }

    /// Convert destinations into a dispatcher-compatible event handler.
    pub fn into_handler(self) -> impl Fn(TraceEvent) + Send + 'static {
        self.into_handler_with_diagnostics(Arc::new(TraceDiagnostics::default()))
    }

    /// Convert destinations into a handler that reports and isolates write failures.
    pub fn into_handler_with_diagnostics(
        self,
        diagnostics: Arc<TraceDiagnostics>,
    ) -> impl Fn(TraceEvent) + Send + 'static {
        move |event| {
            for result in self.write_event(&event) {
                if let Err(error) = result {
                    diagnostics.record_destination_failure(&error);
                }
            }
        }
    }
}

fn write_line<T: Write>(writer: &Mutex<Option<T>>, line: &[u8]) -> io::Result<()> {
    let mut writer = writer
        .lock()
        .map_err(|_| io::Error::other("trace destination mutex was poisoned"))?;
    let Some(writer_ref) = writer.as_mut() else {
        return Ok(());
    };
    if let Err(error) = writer_ref.write_all(line) {
        *writer = None;
        Err(error)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::Read;
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc;
    use std::sync::{Arc, Barrier, Mutex};

    use super::{
        TraceDestinations, TraceDirection, TraceDispatcher, TraceEvent, TracePayload,
        TracePayloadEncoding, TraceProtocol, TraceSession, MAX_TRACE_DATA_BYTES,
        MAX_TRACE_PAYLOAD_BYTES,
    };

    fn test_event(data: &str) -> TraceEvent {
        TraceEvent {
            timestamp: "2026-10-14T12:10:01Z".to_owned(),
            session_id: "session-1".to_owned(),
            direction: TraceDirection::Write,
            protocol: TraceProtocol::Lan,
            address: Some("192.0.2.1:5025".to_owned()),
            byte_length: data.len(),
            encoding: TracePayloadEncoding::Utf8,
            data: data.to_owned(),
        }
    }

    #[test]
    fn dispatches_events_on_background_worker() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let received = Arc::clone(&events);
        let dispatcher = TraceDispatcher::new(2, move |event| {
            received
                .lock()
                .expect("event mutex should not be poisoned")
                .push(event);
        })
        .expect("trace worker should start");

        dispatcher
            .try_enqueue(test_event("first"))
            .expect("event should fit in queue");
        dispatcher
            .try_enqueue(test_event("second"))
            .expect("event should fit in queue");
        drop(dispatcher);

        let events = events.lock().expect("event mutex should not be poisoned");
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].data, "first");
        assert_eq!(events[1].data, "second");
    }

    #[test]
    fn rejects_events_when_bounded_queue_is_full() {
        let (started_sender, started_receiver) = mpsc::channel();
        let release_worker = Arc::new(Barrier::new(2));
        let worker_release = Arc::clone(&release_worker);
        let invocation_count = AtomicUsize::new(0);
        let dispatcher = TraceDispatcher::new(1, move |_event| {
            if invocation_count.fetch_add(1, Ordering::Relaxed) == 0 {
                started_sender
                    .send(())
                    .expect("test should receive worker start");
                worker_release.wait();
            }
        })
        .expect("trace worker should start");

        dispatcher
            .try_enqueue(test_event("active"))
            .expect("first event should be accepted");
        started_receiver
            .recv()
            .expect("worker should start handling first event");
        dispatcher
            .try_enqueue(test_event("queued"))
            .expect("second event should fill queue");
        let rejected = dispatcher
            .try_enqueue(test_event("dropped"))
            .expect_err("third event should be rejected without blocking");
        assert_eq!(rejected.data, "dropped");
        assert_eq!(dispatcher.diagnostics().dropped_event_count(), 1);

        release_worker.wait();
    }

    #[test]
    fn bounds_payload_data_but_preserves_original_byte_length() {
        let bytes = vec![0xff; MAX_TRACE_PAYLOAD_BYTES + 1];
        let payload = TracePayload::from_bytes(&bytes);

        assert_eq!(payload.byte_length, bytes.len());
        assert_eq!(payload.encoding, TracePayloadEncoding::Base64);
        assert!(payload.data.len() <= MAX_TRACE_DATA_BYTES);
    }

    #[test]
    fn rejects_events_with_oversized_encoded_data() {
        let dispatcher = TraceDispatcher::new(1, |_event| {}).expect("trace worker should start");
        let mut event = test_event("small");
        event.data = "x".repeat(MAX_TRACE_DATA_BYTES + 1);

        let rejected = dispatcher
            .try_enqueue(event)
            .expect_err("oversized event should be rejected");

        assert_eq!(rejected.data.len(), MAX_TRACE_DATA_BYTES + 1);
        assert_eq!(dispatcher.diagnostics().dropped_event_count(), 1);
    }

    #[test]
    fn writes_json_lines_to_file_destination() {
        let path = std::env::temp_dir().join(format!(
            "instrument-trace-{}.jsonl",
            TraceSession::new(None).id()
        ));
        let mut destinations = TraceDestinations::new();
        destinations
            .add_file(&path)
            .expect("file destination should open");
        let dispatcher = TraceDispatcher::new(1, destinations.into_handler())
            .expect("trace worker should start");

        dispatcher
            .try_enqueue(test_event("file-event"))
            .expect("event should be accepted");
        drop(dispatcher);

        let content = fs::read_to_string(&path).expect("trace file should be readable");
        fs::remove_file(path).expect("temporary trace file should be removable");
        assert!(content.ends_with('\n'));
        assert!(content.contains("\"data\":\"file-event\""));
    }

    #[test]
    fn writes_json_lines_to_socket_destination() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("listener should bind");
        let address = listener.local_addr().expect("listener should have address");
        let receiver = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("socket client should connect");
            let mut content = String::new();
            stream
                .read_to_string(&mut content)
                .expect("socket data should be readable");
            content
        });

        let mut destinations = TraceDestinations::new();
        destinations
            .add_socket(address)
            .expect("socket destination should connect");
        let dispatcher = TraceDispatcher::new(1, destinations.into_handler())
            .expect("trace worker should start");
        dispatcher
            .try_enqueue(test_event("socket-event"))
            .expect("event should be accepted");
        drop(dispatcher);

        let content = receiver.join().expect("receiver should finish");
        assert!(content.ends_with('\n'));
        assert!(content.contains("\"data\":\"socket-event\""));
    }

    #[test]
    fn preserves_caller_session_id() {
        let session = TraceSession::new(Some("extension-session"));

        assert_eq!(session.id(), "extension-session");
    }

    #[test]
    fn generates_unique_session_ids_when_caller_id_is_missing() {
        let first = TraceSession::new(None);
        let second = TraceSession::new(Some(""));

        assert!(!first.id().is_empty());
        assert!(!second.id().is_empty());
        assert_ne!(first.id(), second.id());
    }

    #[test]
    fn preserves_utf8_payload_and_original_byte_length() {
        let payload = TracePayload::from_bytes("µ".as_bytes());

        assert_eq!(payload.byte_length, 2);
        assert_eq!(payload.encoding, TracePayloadEncoding::Utf8);
        assert_eq!(payload.data, "µ");
    }

    #[test]
    fn encodes_non_utf8_payload_as_base64() {
        let payload = TracePayload::from_bytes(&[0xff, 0x00, 0x01]);

        assert_eq!(payload.byte_length, 3);
        assert_eq!(payload.encoding, TracePayloadEncoding::Base64);
        assert_eq!(payload.data, "/wAB");
    }

    #[test]
    fn serializes_event_as_one_json_line() {
        let event = TraceEvent {
            timestamp: "2026-10-14T12:10:01Z".to_owned(),
            session_id: "session-1".to_owned(),
            direction: TraceDirection::Write,
            protocol: TraceProtocol::Lan,
            address: Some("192.0.2.1:5025".to_owned()),
            byte_length: 6,
            encoding: TracePayloadEncoding::Utf8,
            data: "*IDN?\\n".to_owned(),
        };

        let line = event.to_json_line().expect("event should serialize");

        assert!(line.ends_with('\n'));
        assert_eq!(line.matches('\n').count(), 1);
        assert!(line.contains("\"session_id\":\"session-1\""));
        assert!(line.contains("\"direction\":\"write\""));
        assert!(line.contains("\"protocol\":\"lan\""));
    }
}
