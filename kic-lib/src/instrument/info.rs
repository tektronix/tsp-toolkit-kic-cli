//! Define the trait and datatypes necessary to describe an instrument.
use tracing::{debug, instrument};

use crate::{
    error::Result,
    model::{Model, Vendor},
    InstrumentError,
};
use std::{
    fmt::Display,
    io::{ErrorKind, Read, Write},
    time::Duration,
};

/// The information about an instrument.
#[allow(clippy::module_name_repetitions)]
#[derive(serde::Serialize, Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct InstrumentInfo {
    /// The human-readable name of the vendor that makes the instrument
    pub vendor: Vendor,
    /// The model of the instrument
    pub model: Model,
    /// The serial number of the instrument
    pub serial_number: String,
    /// The firmware revision of the instrument.
    pub firmware_rev: Option<String>,
}

/// Get the [`InstrumentInfo`] from the given object that implements [`Read`] and
/// [`Write`].
///
/// # Errors
/// The following error classes can occur:
/// - Any [`std::io::Error`] that can occur with a [`Read`] or [`Write`] call
/// - Any error in converting the retrieved IDN string into [`InstrumentInfo`]
#[allow(clippy::module_name_repetitions)]
#[instrument(skip(rw))]
pub fn get_info<T: Read + Write + ?Sized>(rw: &mut T) -> Result<InstrumentInfo> {
    debug!("Sending abort");
    rw.write_all(b"abort\n")?;
    std::thread::sleep(Duration::from_millis(100));
    debug!("Sending *CLS");
    rw.write_all(b"*CLS\n")?;
    std::thread::sleep(Duration::from_millis(100));
    debug!("Sending *IDN?");
    rw.write_all(b"*IDN?\n")?;
    let mut info: Option<InstrumentInfo> = None;
    for _ in 0..100 {
        std::thread::sleep(Duration::from_millis(100));

        let mut buf = vec![0u8; 100];
        let read_bytes = match rw.read(&mut buf) {
            Ok(b) => b,
            Err(e) if e.kind() == ErrorKind::WouldBlock => continue,
            Err(e) => return Err(e.into()),
        };
        let buf = &buf[..read_bytes];
        debug!("Buffer after *IDN?: {}", String::from_utf8_lossy(buf));
        if let Ok(i) = buf.try_into() {
            info = Some(i);
            break;
        }
    }
    info.ok_or_else(|| InstrumentError::InformationRetrievalError {
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
        let idn = idn
            .iter()
            .position(|&e| e == b'\0')
            .map_or(idn, |first_null| &idn[..first_null]);

        for line in idn.split(|c| *c == b'\n') {
            let parts: Vec<&[u8]> = line.trim_ascii().split(|c| *c == b',').collect();

            if parts.len() != 4 {
                continue;
            }

            match &parts[..] {
                &[v, m, s, f, ..] => {
                    let fw_rev = String::from_utf8_lossy(f)
                        .to_string()
                        .trim_end_matches(|c| c == char::from(0))
                        .trim()
                        .to_string();
                    let (vendor, model, serial_number, firmware_rev) = (
                        Some(String::from_utf8_lossy(v).trim().to_string()),
                        String::from_utf8_lossy(m)
                            .split_whitespace()
                            .last()
                            .map(|i| i.trim().to_string()),
                        Some(String::from_utf8_lossy(s).trim().to_string()),
                        Some(fw_rev),
                    );

                    let Some(vendor) = vendor else {
                        return Err(InstrumentError::InformationRetrievalError {
                            details: "unable to parse vendor".to_string(),
                        });
                    };

                    let Some(model) = model else {
                        return Err(InstrumentError::InformationRetrievalError {
                            details: "unable to parse model".to_string(),
                        });
                    };

                    let Some(serial_number) = serial_number else {
                        return Err(InstrumentError::InformationRetrievalError {
                            details: "unable to parse serial number".to_string(),
                        });
                    };

                    return Ok(Self {
                        vendor: vendor.parse::<Vendor>()?,
                        model: model.parse::<Model>()?,
                        serial_number,
                        firmware_rev,
                    });
                }
                _ => {
                    return Err(InstrumentError::InformationRetrievalError {
                        details: "unable to parse instrument information".to_string(),
                    });
                }
            };
        }
        Err(InstrumentError::InformationRetrievalError {
            details: "unable to get instrument information".to_string(),
        })
    }
}

fn parse_xml_info_field(
    xml: &roxmltree::Document,
    tag: &str,
) -> std::result::Result<String, InstrumentError> {
    let Some(item) = xml.descendants().find(|n| n.tag_name().name() == tag) else {
        return Err(InstrumentError::InformationRetrievalError {
            details: format!("unable to get {tag} tag from LXI identification XML"),
        });
    };

    let Some(item) = item.text() else {
        return Err(InstrumentError::InformationRetrievalError {
            details: format!(
                "the {tag} tag in the LXI identification XML did not contain any text"
            ),
        });
    };

    Ok(item.trim().to_string())
}

impl TryFrom<&String> for InstrumentInfo {
    type Error = InstrumentError;

    fn try_from(xml_data: &String) -> std::result::Result<Self, Self::Error> {
        let xml = roxmltree::Document::parse(xml_data)?;
        let vendor: Vendor = parse_xml_info_field(&xml, "Manufacturer")?.parse()?;
        let model: Model = parse_xml_info_field(&xml, "Model")?.parse()?;
        let serial_number = parse_xml_info_field(&xml, "SerialNumber")?;
        let firmware_rev = Some(parse_xml_info_field(&xml, "FirmwareRevision")?);

        Ok(Self {
            vendor,
            model,
            serial_number,
            firmware_rev,
        })
    }
}

// impl tryFrom for string lxi

impl Display for InstrumentInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let vendor = self.vendor.to_string();

        let model: String = self.model.to_string();

        let sn: String = self.serial_number.to_string();

        let fw_rev = self
            .firmware_rev
            .clone()
            .unwrap_or_else(|| String::from("<UNKNOWN FIRMWARE REVISION>"));

        write!(f, "{vendor},MODEL {model},{sn},{fw_rev}")
    }
}

#[cfg(test)]
mod unit {
    use crate::model::{Model, Vendor};

    use super::InstrumentInfo;

    #[test]
    fn idn_to_instrument_info_prompts() {
        let input = br"TSP>
KEITHLEY INSTRUMENTS,MODEL 2461,04331961,1.7.12b
TSP>";
        let expected = InstrumentInfo {
            vendor: Vendor::Keithley,
            model: Model::_2461,
            serial_number: "04331961".to_string(),
            firmware_rev: Some("1.7.12b".to_string()),
        };

        let actual = InstrumentInfo::try_from(&input[..]);
        assert!(actual.is_ok(), "Unable to parse InstrumentInfo from &[u8]");

        let actual = actual.unwrap();

        assert_eq!(actual, expected);
    }
}
