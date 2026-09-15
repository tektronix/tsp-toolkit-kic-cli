use std::{
    io::{Read, Write},
    ops::{Deref, DerefMut},
};

use visa_rs::{
    enums::attribute::{AttrTermchar, AttrTermcharEn, AttrTmoValue, HasAttribute},
    flags::AccessMode,
    AsResourceManager, VisaString, TIMEOUT_INFINITE,
};

/// LF is the line terminator used by raw VISA sockets so `viRead` can end a read as
/// soon as a full TSP reply line has arrived, instead of requiring the buffer to fill.
const LINE_TERMINATOR: u8 = b'\n';

use crate::{interface::NonBlock, protocol::stb::Stb, InstrumentError, Interface};

pub struct Visa {
    _rm: visa_rs::DefaultRM,
    inst: visa_rs::Instrument,
    nonblocking: bool,
    uses_status_byte: bool,
}

impl Visa {
    /// Create a new VISA-based resource
    ///
    /// # Errors
    /// Errors can occur when creating the [`DefaultRM`], creating the [`VisaString`],
    /// and opening the [`visa_rs::Instrument`]
    pub fn new(resource_string: &str, uses_status_byte: bool) -> Result<Self, InstrumentError> {
        let rm = visa_rs::DefaultRM::new()?;
        let Some(resource_string) = VisaString::from_string(resource_string.to_string()) else {
            return Err(InstrumentError::VisaParseError(format!(
                "VISA unable to parse '{resource_string}' as resource string"
            )));
        };
        let inst: visa_rs::Instrument =
            rm.open(&resource_string, AccessMode::NO_LOCK, TIMEOUT_INFINITE)?;
        if !uses_status_byte {
            // Raw sockets have no status byte, so a read must stop at line end instead
            // of waiting to fill the buffer or for VISA's pause-detection heuristic.
            inst.set_attr(AttrTermcharEn::new_checked(1u16).expect("valid VISA bool"))?;
            inst.set_attr(AttrTermchar::new_checked(LINE_TERMINATOR).expect("valid VISA termchar"))?;
        }
        Ok(Self {
            _rm: rm,
            inst,
            nonblocking: true,
            uses_status_byte,
        })
    }

    pub const fn uses_status_byte(&self) -> bool {
        self.uses_status_byte
    }
}

impl NonBlock for Visa {
    fn set_nonblocking(&mut self, enable: bool) -> Result<(), InstrumentError> {
        if !self.uses_status_byte {
            let timeout = if enable {
                0
            } else {
                u32::MAX
            };
            self.inst
                .set_attr(AttrTmoValue::new_checked(timeout).expect("valid VISA timeout"))?;
        }
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
        if self.nonblocking && self.uses_status_byte {
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
        match self.inst.read(buf) {
            Err(e) if self.nonblocking && !self.uses_status_byte && e.kind() == std::io::ErrorKind::TimedOut => {
                Err(std::io::Error::new(
                    std::io::ErrorKind::WouldBlock,
                    "No message available",
                ))
            }
            result => result,
        }
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
