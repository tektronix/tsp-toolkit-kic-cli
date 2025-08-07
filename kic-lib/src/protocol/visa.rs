use std::{
    io::{Read, Write},
    ops::{Deref, DerefMut},
};

use visa_rs::{flags::AccessMode, AsResourceManager, VisaString, TIMEOUT_INFINITE};

use crate::{interface::NonBlock, protocol::stb::Stb, InstrumentError, Interface};

pub struct Visa {
    _rm: visa_rs::DefaultRM,
    inst: visa_rs::Instrument,
    nonblocking: bool,
}

impl Visa {
    /// Create a new VISA-based resource
    ///
    /// # Errors
    /// Errors can occur when creating the [`DefaultRM`], creating the [`VisaString`],
    /// and opening the [`visa_rs::Instrument`]
    pub fn new(resource_string: &str) -> Result<Self, InstrumentError> {
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
        })
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
        self.inst.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inst.flush()
    }
}

impl Read for Visa {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.nonblocking {
            let stb = Stb::Stb(
                self.inst
                    .read_stb()
                    .map_err(|e| std::io::Error::other(format!("error reading STB: {e}")))?,
            );

            if matches!(stb.message_available(), Ok(false)) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::WouldBlock,
                    "No message available",
                ));
            }
        }
        self.inst.read(buf)
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
