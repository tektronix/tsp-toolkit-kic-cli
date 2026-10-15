use std::{
    io::{Read, Write},
    ops::{Deref, DerefMut},
};

use crate::{
    trace::{TraceContext, TraceDirection, TraceProtocol},
    Interface,
};

pub struct Raw {
    interface: Box<dyn Interface>,
    pub(crate) trace: Option<TraceContext>,
    trace_protocol: Option<TraceProtocol>,
    trace_address: Option<String>,
}

impl Raw {
    pub fn new(interface: impl Interface + 'static) -> Self {
        Self {
            interface: Box::new(interface),
            trace: None,
            trace_protocol: None,
            trace_address: None,
        }
    }

    pub(crate) fn new_with_trace(
        interface: impl Interface + 'static,
        trace: Option<TraceContext>,
        protocol: TraceProtocol,
        address: Option<String>,
    ) -> Self {
        Self {
            interface: Box::new(interface),
            trace,
            trace_protocol: Some(protocol),
            trace_address: address,
        }
    }

    #[must_use]
    pub fn trace_context(&self) -> Option<&TraceContext> {
        self.trace.as_ref()
    }
}

impl Read for Raw {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let result = self.interface.read(buf);
        if let (Ok(bytes_read), Some(trace), Some(protocol)) =
            (&result, &self.trace, self.trace_protocol)
        {
            trace.record(
                TraceDirection::Read,
                protocol,
                self.trace_address.as_deref(),
                &buf[..*bytes_read],
            );
        }
        result
    }
}

impl Write for Raw {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let result = self.interface.write(buf);
        if let (Ok(bytes_written), Some(trace), Some(protocol)) =
            (&result, &self.trace, self.trace_protocol)
        {
            trace.record(
                TraceDirection::Write,
                protocol,
                self.trace_address.as_deref(),
                &buf[..*bytes_written],
            );
        }
        result
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.interface.flush()
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Cursor, Read, Write},
        sync::{mpsc, Arc},
        time::Duration,
    };

    use crate::{
        interface::NonBlock,
        trace::{TraceContext, TraceDirection, TraceDispatcher, TraceProtocol, TraceSession},
        Interface,
    };

    use super::Raw;

    struct TestInterface {
        input: Cursor<Vec<u8>>,
    }

    impl NonBlock for TestInterface {
        fn set_nonblocking(&mut self, _enable: bool) -> crate::error::Result<()> {
            Ok(())
        }
    }

    impl Read for TestInterface {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            self.input.read(buf)
        }
    }

    impl Write for TestInterface {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            Ok(buf.len().min(2))
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl Interface for TestInterface {}

    #[test]
    fn traces_successful_partial_reads_and_writes() {
        let (event_sender, event_receiver) = mpsc::channel();
        let dispatcher = Arc::new(
            TraceDispatcher::new(4, move |event| {
                event_sender
                    .send(event)
                    .expect("test receiver should be alive");
            })
            .expect("trace dispatcher should start"),
        );
        let context = TraceContext::new(TraceSession::new(Some("raw-test")), dispatcher);
        let mut raw = Raw::new_with_trace(
            TestInterface {
                input: Cursor::new(b"response".to_vec()),
            },
            Some(context),
            TraceProtocol::Lan,
            Some("127.0.0.1:5025".to_owned()),
        );

        let mut read_buffer = [0; 4];
        assert_eq!(raw.read(&mut read_buffer).expect("read should succeed"), 4);
        assert_eq!(raw.write(b"request").expect("write should succeed"), 2);

        let read_event = event_receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("read event should be delivered");
        assert_eq!(read_event.direction, TraceDirection::Read);
        assert_eq!(read_event.byte_length, 4);
        assert_eq!(read_event.data, "resp");
        assert_eq!(read_event.session_id, "raw-test");
        assert_eq!(read_event.protocol, TraceProtocol::Lan);
        assert_eq!(read_event.address.as_deref(), Some("127.0.0.1:5025"));

        let write_event = event_receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("write event should be delivered");
        assert_eq!(write_event.direction, TraceDirection::Write);
        assert_eq!(write_event.byte_length, 2);
        assert_eq!(write_event.data, "re");
    }
}

impl Deref for Raw {
    type Target = Box<dyn Interface>;

    fn deref(&self) -> &Self::Target {
        &self.interface
    }
}

impl DerefMut for Raw {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.interface
    }
}
