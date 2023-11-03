//! Define the trait and datatypes necessary to describe an instrument.

use crate::{error::Result, usbtmc::UsbtmcAddr, InstrumentError};
use std::{
    fmt::Display,
    io::{Read, Write},
    net::SocketAddr,
    time::Duration,
};

/// A generic connection address that covers all the different connection types.
/// Each device interface type will also have a [`TryFrom`] impl that converts from
/// this enum to itself. [`From`] is **not** implemented because the conversion could
/// fail.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ConnectionAddr {
    /// A LAN connection is created with a [`SocketAddr`], which includes an IpAddr and
    /// a port for the connection.
    Lan(SocketAddr),

    /// A USBTMC connection is created with a [`UsbtmcAddr`].
    Usbtmc(UsbtmcAddr),
    //Add other device interface types here
    Unknown,
}

impl Display for ConnectionAddr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Lan(lan_info) => lan_info.to_string(),
            Self::Usbtmc(usb_info) => usb_info.to_string(),
            Self::Unknown => "<UNKNOWN>".to_string(),
        };
        write!(f, "{s}")
    }
}

/// The information about an instrument.
#[allow(clippy::module_name_repetitions)]
#[derive(serde::Serialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct InstrumentInfo {
    /// The human-readable name of the vendor that makes the instrument
    pub vendor: Option<String>,
    /// The model of the instrument
    pub model: Option<String>,
    /// The serial number of the instrument
    pub serial_number: Option<String>,
    /// The firmware revision of the instrument.
    pub firmware_rev: Option<String>,
    /// The [`ConnectionAddr`] of the instrument.
    #[serde(skip)]
    pub address: Option<ConnectionAddr>,
}

/// Get the [`InstrumentInfo`] from the given object that implements [`Read`] and
/// [`Write`].
///
/// # Errors
/// The following error classes can occur:
/// - Any [`std::io::Error`] that can occur with a [`Read`] or [`Write`] call
/// - Any error in converting the retrieved IDN string into [`InstrumentInfo`]
#[allow(clippy::module_name_repetitions)]
pub fn get_info<T: Read + Write + ?Sized>(rw: &mut T) -> Result<InstrumentInfo> {
    rw.write_all(b"*IDN?\n")?;
    let mut info: Option<InstrumentInfo> = None;
    for _ in 0..100 {
        std::thread::sleep(Duration::from_millis(100));

        let mut buf = vec![0u8; 100];
        let _ = rw.read(&mut buf)?;
        let first_null = buf.iter().position(|&x| x == b'\0').unwrap_or(buf.len());
        let buf = &buf[..first_null];
        if let Ok(i) = buf.try_into() {
            info = Some(i);
            break;
        }
    }
    info.ok_or(InstrumentError::InformationRetrievalError {
        details: "unable to read instrument info".to_string(),
    })
}

/// A trait to get the information from an instrument.
pub trait Info: Read + Write {
    /// Get the information for the instrument.
    ///
    /// # Errors
    /// [`TeaspoonInstrumentError::InformationRetrievalError`] if an instrument did not
    /// return the requested information.
    fn info(&mut self) -> Result<InstrumentInfo> {
        get_info(self)
    }
}

impl TryFrom<&[u8]> for InstrumentInfo {
    type Error = InstrumentError;

    fn try_from(idn: &[u8]) -> std::result::Result<Self, Self::Error> {
        let parts: Vec<&[u8]> = idn
            .split(|c| *c == b',' || *c == b'\n' || *c == b'\0')
            .collect();

        let (vendor, model, serial_number, firmware_rev) = match &parts[..] {
            &[v, m, s, f, ..] => {
                let fw_rev = String::from_utf8_lossy(f)
                    .to_string()
                    .trim_end_matches(|c| c == char::from(0))
                    .trim()
                    .to_string();
                (
                    Some(String::from_utf8_lossy(v).to_string()),
                    String::from_utf8_lossy(m)
                        .to_string()
                        .split("MODEL ")
                        .last()
                        .map(std::string::ToString::to_string),
                    Some(String::from_utf8_lossy(s).to_string()),
                    Some(fw_rev),
                )
            }
            _ => {
                return Err(InstrumentError::InformationRetrievalError {
                    details: "unable to parse instrument information".to_string(),
                });
            }
        };

        if model.is_none() {
            return Err(InstrumentError::InformationRetrievalError {
                details: "unable to parse model".to_string(),
            });
        }

        Ok(Self {
            vendor,
            model,
            serial_number,
            firmware_rev,
            address: None,
        })
    }
}

impl Display for InstrumentInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let vendor = self.vendor.as_ref().map_or_else(
            || String::from("<UNKNOWN VENDOR>"),
            std::clone::Clone::clone,
        );

        let model: String = self
            .model
            .as_ref()
            .map_or_else(|| String::from("<UNKNOWN MODEL>"), std::clone::Clone::clone);

        let sn: String = self.serial_number.as_ref().map_or_else(
            || String::from("<UNKNOWN SERIAL NUMBER>"),
            std::clone::Clone::clone,
        );

        let fw_rev = self.firmware_rev.as_ref().map_or_else(
            || String::from("<UNKNOWN FIRMWARE REVISION>"),
            std::clone::Clone::clone,
        );

        let addr = self
            .address
            .as_ref()
            .map_or_else(|| ConnectionAddr::Unknown, std::clone::Clone::clone);

        if addr == ConnectionAddr::Unknown {
            write!(f, "{vendor},MODEL {model},{sn},{fw_rev}")
        } else {
            write!(f, "{vendor},MODEL {model},{sn},{fw_rev},{addr}")
        }
    }
}
