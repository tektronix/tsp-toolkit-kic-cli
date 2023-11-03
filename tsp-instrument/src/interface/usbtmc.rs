use std::{
    fmt::Display,
    hash::Hash,
    io::{ErrorKind, Read, Write},
    str::FromStr,
    time::Duration,
};

use rusb::{Context, DeviceList};
use tmc::InstrumentHandle;

use crate::{error::Result, interface, interface::NonBlock, InstrumentError};

const KEITHLEY_VID: u16 = 0x05e6;

/// An address representing how to connect to a USBTMC device.
#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Clone)]
pub struct UsbtmcAddr {
    pub device: rusb::Device<Context>,
    pub model: String,
    pub serial: String,
}

impl Hash for UsbtmcAddr {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        format!("{}:{}:{}", "USB", self.model, self.serial).hash(state);
    }
}

impl Eq for UsbtmcAddr {}

impl PartialEq for UsbtmcAddr {
    fn eq(&self, other: &Self) -> bool {
        self.serial == other.serial && self.model == other.model
    }
}

impl Display for UsbtmcAddr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "USB:{}:{}", self.model, self.serial)
    }
}

impl FromStr for UsbtmcAddr {
    type Err = InstrumentError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        //USB:2450:01234567
        let split: Vec<&str> = s.split(':').collect();
        if split.len() != 3 {
            return Err(InstrumentError::AddressParsingError {
                unparsable_string: s.to_string(),
            });
        }

        let (ix, model, serial) = (split[0], split[1], split[2]);

        if ix.trim() != "USB" {
            return Err(InstrumentError::AddressParsingError {
                unparsable_string: s.to_string(),
            });
        }
        let context = rusb::Context::new()?;
        for device in DeviceList::new_with_context(context)?.iter() {
            let desc = device.device_descriptor()?;
            if desc.vendor_id() == KEITHLEY_VID
                && desc.product_id() == u16::from_str_radix(model, 16)?
            {
                let handle = device.open()?;
                let languages = handle.read_languages(Duration::from_millis(100))?;
                let Some(language) = languages.first() else {
                    continue;
                };
                let sn = handle.read_serial_number_string(
                    *language,
                    &desc,
                    Duration::from_millis(100),
                )?;
                if *serial == sn {
                    return Ok(Self {
                        device,
                        model: model.to_string(),
                        serial: serial.to_string(),
                    });
                }
            }
        }

        Err(InstrumentError::ConnectionError {
            details: format!(
                "USB device with model '{model}' and serial number '{serial}' could not be found"
            ),
        })
    }
}

#[derive(Debug)]
pub struct Stream {
    handle: InstrumentHandle<rusb::Context>,
    nonblocking: bool,
}

impl TryFrom<UsbtmcAddr> for Stream {
    type Error = InstrumentError;

    fn try_from(addr: UsbtmcAddr) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            handle: tmc::Instrument::new(addr.device)?
                .ok_or(InstrumentError::ConnectionError {
                    details: "unable to connect to USB instrument".to_string(),
                })?
                .open()?,
            nonblocking: false,
        })
    }
}

impl TryFrom<InstrumentHandle<rusb::Context>> for Stream {
    type Error = InstrumentError;
    fn try_from(handle: InstrumentHandle<rusb::Context>) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            handle,
            nonblocking: false,
        })
    }
}

impl NonBlock for Stream {
    fn set_nonblocking(&mut self, enable: bool) -> Result<()> {
        self.nonblocking = enable;
        Ok(())
    }
}

impl interface::Interface for Stream {}

impl Write for Stream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.handle.write_raw(buf)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        //Nothing to force-flush on USBTMC, handled by `write_raw` above
        Ok(())
    }
}

impl Read for Stream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let message_available = if self.nonblocking {
            self.handle.read_stb(Some(Duration::from_millis(10)))?
        } else {
            while !self.handle.read_stb(Some(Duration::from_millis(10)))? {}
            true
        };

        if message_available {
            let msg = self.handle.read_raw(Some(match u32::try_from(buf.len()) {
                Ok(v) => v,
                Err(e) => {
                    return Err(std::io::Error::new(
                        ErrorKind::Other,
                        format!("buffer larger than can be read: {e}"),
                    ));
                }
            }))?;
            msg.take(buf.len() as u64).read(buf)
        } else {
            Ok(0)
        }
    }
}
