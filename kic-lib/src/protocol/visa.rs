use std::{
    io::{Read, Write},
    ops::{Deref, DerefMut},
};

use visa_rs::{flags::AccessMode, AsResourceManager, VisaString, TIMEOUT_INFINITE};

use crate::{
    interface::NonBlock,
    protocol::stb::Stb,
    trace::{TraceContext, TraceDirection, TraceProtocol},
    InstrumentError, Interface,
};

pub struct Visa {
    _rm: visa_rs::DefaultRM,
    inst: visa_rs::Instrument,
    nonblocking: bool,
    pub(crate) trace: Option<TraceContext>,
    trace_address: Option<String>,
}

impl Visa {
    /// Create a new VISA-based resource
    ///
    /// # Errors
    /// Errors can occur when creating the [`DefaultRM`], creating the [`VisaString`],
    /// and opening the [`visa_rs::Instrument`]
    pub fn new(resource_string: &str) -> Result<Self, InstrumentError> {
        Self::new_with_trace(resource_string, None)
    }

    pub(crate) fn new_with_trace(
        resource_string: &str,
        trace: Option<TraceContext>,
    ) -> Result<Self, InstrumentError> {
        let trace_address = resource_string.to_owned();
        let rm = visa_rs::DefaultRM::new()?;
        let Some(resource_string) = VisaString::from_string(resource_string.to_string()) else {
            return Err(InstrumentError::VisaParseError(format!(
                "VISA unable to parse '{resource_string}' as resource string"
            )));
        };
        let inst: visa_rs::Instrument =
            rm.open(&resource_string, AccessMode::NO_LOCK, TIMEOUT_INFINITE)?;
        Ok(Self {
            _rm: rm,
            inst,
            nonblocking: true,
            trace,
            trace_address: Some(trace_address),
        })
    }

    #[must_use]
    pub fn trace_context(&self) -> Option<&TraceContext> {
        self.trace.as_ref()
    }
}

impl NonBlock for Visa {
    fn set_nonblocking(&mut self, enable: bool) -> Result<(), InstrumentError> {
        self.nonblocking = enable;
        Ok(())
    }
}

impl Write for Visa {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let result = self.inst.write(buf);
        if let (Ok(bytes_written), Some(trace)) = (&result, &self.trace) {
            trace.record(
                TraceDirection::Write,
                TraceProtocol::Visa,
                self.trace_address.as_deref(),
                &buf[..*bytes_written],
            );
        }
        result
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inst.flush()
    }
}

impl Read for Visa {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.nonblocking {
            let stb = match self.inst.read_stb() {
                Ok(stb) => Stb::Stb(stb),
                Err(e) =>
                // If device is disconnected or STB read fails
                {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::NotConnected,
                        format!("Device disconnected or STB read failed: {e}"),
                    ))
                }
            };

            if matches!(stb.message_available(), Ok(false)) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::WouldBlock,
                    "No message available",
                ));
            }
        }
        let result = self.inst.read(buf);
        if let (Ok(bytes_read), Some(trace)) = (&result, &self.trace) {
            trace.record(
                TraceDirection::Read,
                TraceProtocol::Visa,
                self.trace_address.as_deref(),
                &buf[..*bytes_read],
            );
        }
        result
    }
}

impl Deref for Visa {
    type Target = visa_rs::Instrument;

    fn deref(&self) -> &Self::Target {
        &self.inst
    }
}

impl DerefMut for Visa {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inst
    }
}

impl Interface for Visa {}
