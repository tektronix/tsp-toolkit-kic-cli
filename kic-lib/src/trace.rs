use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine;

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
        match std::str::from_utf8(bytes) {
            Ok(text) => Self {
                byte_length: bytes.len(),
                encoding: TracePayloadEncoding::Utf8,
                data: text.to_owned(),
            },
            Err(_) => Self {
                byte_length: bytes.len(),
                encoding: TracePayloadEncoding::Base64,
                data: base64::engine::general_purpose::STANDARD.encode(bytes),
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
#[cfg(test)]
mod tests {
    use super::{
        TraceDirection, TraceEvent, TracePayload, TracePayloadEncoding, TraceProtocol, TraceSession,
    };

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
