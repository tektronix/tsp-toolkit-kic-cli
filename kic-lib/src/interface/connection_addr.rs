use std::fmt::Display;
use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::str::FromStr;
use std::time::Duration;

use reqwest::blocking::Client;

use tracing::{instrument, trace};

#[cfg(feature = "visa")]
use visa_rs::{AsResourceManager, VisaString};

#[cfg(feature = "visa")]
use tracing::error;

use crate::instrument::info::InstrumentInfo;
use crate::model::{Model, Vendor};
use crate::InstrumentError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionInfo {
    /// A raw socket connection.
    Lan { addr: SocketAddr },
    /// A VXI-11 connection (requires VISA to use)
    Vxi11 { string: String, addr: Ipv4Addr },

    #[allow(clippy::doc_markdown)] // RustDoc wants "HiSLIP" to be a code term, but it isn't
    /// A HiSLIP connection (requires VISA to use)
    HiSlip { string: String, addr: IpAddr },
    /// A raw socket connection over VISA (requires VISA to use)
    VisaSocket { string: String, addr: SocketAddr },
    /// A GPIB connection (requires VISA to use)
    Gpib { string: String },
    /// A USBTMC connection (requires VISA to use)
    Usb {
        string: String,
        vendor: Vendor,
        model: Model,
        serial: String,
        interface_number: Option<u16>,
    },
}

impl Display for ConnectionInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Lan { addr } => addr.to_string(),
            Self::Vxi11 { string, .. }
            | Self::HiSlip { string, .. }
            | Self::VisaSocket { string, .. }
            | Self::Gpib { string }
            | Self::Usb { string, .. } => string.to_string(),
        };

        write!(f, "{s}")
    }
}

impl ConnectionInfo {
    /// Check to see if this instrument can be connected to.
    ///
    /// # Errors
    /// Errors can come in the form of [`reqwest`] errors or, if the "visa" feature is
    /// enabled, [`visa_rs`] IO Errors.
    #[instrument(skip(self))]
    pub fn ping(&self) -> Result<InstrumentInfo, InstrumentError> {
        match self {
            Self::Lan { .. }
            | Self::Vxi11 { .. }
            | Self::HiSlip { .. }
            | Self::VisaSocket { .. } => self.get_info(),
            Self::Gpib { string } | Self::Usb { string, .. } => self.ping_usb_gpib(string),
        }
    }

    #[cfg(feature = "visa")]
    fn ping_usb_gpib(&self, addr: &str) -> Result<InstrumentInfo, InstrumentError> {
        let rm = match visa_rs::DefaultRM::new() {
            Ok(r) => r,
            Err(e) => {
                error!("Instrument unavailable: {e}");
                return Err(e.into());
            }
        };

        let Some(expr) = VisaString::from_string(addr.to_string()) else {
            return Err(InstrumentError::AddressParsingError(format!(
                "{addr} was not a valid visa resource string"
            )));
        };

        match rm.find_res(&expr) {
            Ok(_) => {}
            Err(e) => {
                error!("Unable to find instrument: {e}");
                return Err(e.into());
            }
        }
        self.get_info()
    }

    #[cfg(not(feature = "visa"))]
    #[allow(clippy::unused_self)] // This is the counterpart to the visa enabled-version so we need
                                  // to keep the same shape.
    const fn ping_usb_gpib(&self, _: &str) -> Result<InstrumentInfo, InstrumentError> {
        Err(InstrumentError::NoVisa)
    }

    /// Get the info from this connection information
    ///
    /// # Errors
    /// Errors may occur when fetching or parsing the data from LXI identification page
    /// or the IDN string (depending on the connection protocol)
    #[instrument(skip(self))]
    pub fn get_info(&self) -> Result<InstrumentInfo, InstrumentError> {
        trace!("getting instrument info");
        let xml = match self {
            Self::Lan { addr } if addr.ip().is_loopback() => {
                trace!("getting info over loopback");
                //Special case for TSPop
                let mut inst = TcpStream::connect(addr)?;
                inst.write_all(b"abort\n")?;
                inst.write_all(b"*CLS\n")?;
                std::thread::sleep(Duration::from_millis(100));
                inst.write_all(b"*IDN?\n")?;
                let buf = &mut [0u8; 128];
                let num_bytes = inst.read(buf)?;
                let buf = &buf[..num_bytes];
                drop(inst);
                return buf.try_into();
            }
            Self::Lan { .. }
            | Self::Vxi11 { .. }
            | Self::HiSlip { .. }
            | Self::VisaSocket { .. } => {
                trace!("getting information from LXI identification page");
                self.get_lxi_id_xml()?
            }
            // The USBTMC resource string requires the USB model identifier, so we can
            // get that directly and return it.
            Self::Usb {
                vendor,
                model,
                serial,
                ..
            } => {
                trace!("deriving information from USB resource string");
                return Ok(InstrumentInfo {
                    vendor: vendor.clone(),
                    model: model.clone(),
                    serial_number: serial.clone(),
                    firmware_rev: None,
                });
            }
            // GPIB just uses `*IDN?` and assumes the 2nd comma-separated element is
            // the value we need, parses it, and directly returns it.
            Self::Gpib { string } => {
                trace!("Getting information over GPIB");
                return Self::get_gpib_info(string);
            }
        };

        let Some(xml) = xml else {
            return Err(InstrumentError::InformationRetrievalError {
                details: "Unable to retrieve xml".to_string(),
            });
        };
        (&xml).try_into()
    }
    /// Retrieve the model information for this connection.
    ///
    /// # Errors
    /// Errors can occur from attempting to get the instrument information with
    /// [`ConnectionInfo::get_info()`].
    #[instrument(skip(self))]
    pub fn get_model(&self) -> Result<Model, InstrumentError> {
        trace!("Getting model");
        Ok(self.get_info()?.model)
    }

    #[cfg(feature = "visa")]
    fn get_gpib_info(string: &str) -> Result<InstrumentInfo, InstrumentError> {
        use std::io::{Read, Write};

        use visa_rs::{flags::AccessMode, AsResourceManager, DefaultRM, TIMEOUT_INFINITE};

        let rm = DefaultRM::new()?;
        let Some(string) = VisaString::from_string(string.to_string()) else {
            return Err(InstrumentError::VisaParseError(format!(
                "unable to convert '{string}' to VisaString"
            )));
        };
        let mut inst = rm.open(&string, AccessMode::NO_LOCK, TIMEOUT_INFINITE)?;
        inst.write_all(b"abort\n")?;
        inst.write_all(b"*CLS\n")?;
        std::thread::sleep(Duration::from_millis(100));
        inst.write_all(b"*IDN?\n")?;
        let buf = &mut [0u8; 128];
        let num_bytes = inst.read(buf)?;
        let buf = &buf[..num_bytes];
        buf.try_into()
    }

    #[cfg(not(feature = "visa"))]
    const fn get_gpib_info(_string: &str) -> Result<InstrumentInfo, InstrumentError> {
        Err(InstrumentError::NoVisa)
    }

    fn get_lxi_id_xml(&self) -> Result<Option<String>, InstrumentError> {
        // FIXME: If an instrument is serving `https`, the certificate will be self-signed.
        // for now, just ignore it. A better option would be to load a copy of the cert
        // into the rustls backend.
        let client = Client::builder()
            .danger_accept_invalid_certs(true)
            .timeout(Duration::from_millis(100))
            .build()?;

        // Most connection modes require getting an xml document, so assume that is the
        // case and save the XML document off here. Anything that can get the model
        // number via a different route (i.e. `*IDN?` or from the resource string)
        // should return directly from the associated match arm.
        let xml = match self {
            Self::Lan { addr } => {
                // We don't know whether the instrument serves `https` or not, but if
                // it does it will redirect, so just use `http`
                client
                    .get(format!("http://{}/lxi/identification", addr.ip()))
                    .timeout(Duration::from_secs(2))
                    .send()?
                    .text()?
            }
            Self::Vxi11 { addr, .. } => {
                // If the instrument is using VXI-11, we can be reasonably sure it
                // doesn't serve `https`, so this won't redirect.
                client
                    .get(format!("http://{addr}/lxi/identification"))
                    .send()?
                    .text()?
            }
            Self::HiSlip { addr, .. } => {
                // If the instrument is using HiSLIP, we can be reasonably sure it
                // is serving `https`.
                client
                    .get(format!("http://{addr}/lxi/identification"))
                    .send()?
                    .text()?
            }
            Self::VisaSocket { addr, .. } => {
                // We don't know whether the instrument serves `https` or not, but if
                // it does it will redirect, so just use `http`
                client
                    .get(format!("http://{}/lxi/identification", addr.ip()))
                    .send()?
                    .text()?
            }
            Self::Usb { .. } | Self::Gpib { .. } => return Ok(None),
        };

        Ok(Some(xml))
    }
}

/// If a string starts with `0x`, assume this is a u16 represented in hex. Otherwise,
/// assume it is decimal. Ignore parsing errors and just return an [`Some`] if
/// conversion succeeds or [`None`] otherwise.
fn u16_from_str(s: &str) -> Option<u16> {
    if s.len() < 2 {
        return s.parse::<u16>().ok();
    }
    match &s[..2] {
        "0x" => u16::from_str_radix(&s[2..], 16).ok(),
        _ => s.parse::<u16>().ok(),
    }
}

fn parse_raw_socket(s: &str) -> Option<ConnectionInfo> {
    // If the user supplied an IP address has a port number on it...
    let ip = s.parse::<SocketAddr>();
    if let Ok(ip) = ip {
        return Some(ConnectionInfo::Lan { addr: ip });
    }
    // If the user supplied an IP address with NO port number, default to port 5025
    let ip = s.parse::<IpAddr>();
    if let Ok(ip) = ip {
        return Some(ConnectionInfo::Lan {
            addr: SocketAddr::new(ip, 5025),
        });
    }
    None
}

fn parse_tcpip_resource_string(s: &str, parts: &[&str]) -> Result<ConnectionInfo, InstrumentError> {
    match parts
        .last()
        .expect("should have at least 1 element")
        .chars()
        .next()
    {
        Some('S') => {
            let addr = parts[1].parse::<IpAddr>()?;
            let port = match parts[2].parse::<u16>() {
                Ok(p) => p,
                Err(e) => {
                    return Err(InstrumentError::AddressParsingError(format!(
                        "unable to parse port number '{}': {e}",
                        parts[2]
                    )));
                }
            };
            Ok(ConnectionInfo::VisaSocket {
                string: s.trim().to_string(),
                addr: SocketAddr::new(addr, port),
            })
        }
        Some('I') => {
            if matches!(
                &parts[parts.len().saturating_sub(2)].chars().next(),
                Some('h')
            ) {
                let addr = parts[1].parse::<IpAddr>()?;
                Ok(ConnectionInfo::HiSlip {
                    string: s.trim().to_string(),
                    addr,
                })
            } else {
                // if it is a TCPIP connection that doesn't explicitly declare `hislip`, just
                // assume it is VXI-11
                let addr = parts[1].parse::<Ipv4Addr>()?;
                Ok(ConnectionInfo::Vxi11 {
                    string: s.trim().to_string(),
                    addr,
                })
            }
        }
        _ => Err(InstrumentError::AddressParsingError(format!(
            "'{s}' did not have a recognized VISA address"
        ))),
    }
}

impl FromStr for ConnectionInfo {
    type Err = InstrumentError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(lan) = parse_raw_socket(s) {
            return Ok(lan);
        }

        let resource_string: Vec<&str> = s.split("::").collect();

        if resource_string.is_empty() {
            return Err(InstrumentError::AddressParsingError(format!(
                "{s} did not contain the expected format for a connection string"
            )));
        }

        match &resource_string[0][..3] {
            "TCP" => parse_tcpip_resource_string(s, &resource_string),
            "USB" => {
                if resource_string.len() < 4 {
                    Err(InstrumentError::AddressParsingError(format!(
                        "'{s}' did not have a recognized VISA address"
                    )))
                } else {
                    let Some(vid) = u16_from_str(resource_string[1]) else {
                        return Err(InstrumentError::AddressParsingError(format!(
                            "'{s}' did not have a recognized VISA address"
                        )));
                    };
                    let vid: Vendor = vid.try_into()?;
                    let Some(pid) = u16_from_str(resource_string[2]) else {
                        return Err(InstrumentError::AddressParsingError(format!(
                            "'{s}' did not have a recognized VISA address"
                        )));
                    };
                    let pid: Model = Model::from_pid(pid);

                    let serial = resource_string[3].to_string();

                    let interface_number = if resource_string.len() == 5 {
                        if resource_string[4].starts_with("INSTR") {
                            None
                        } else {
                            u16_from_str(resource_string[4])
                        }
                    } else {
                        u16_from_str(resource_string[4])
                    };
                    Ok(Self::Usb {
                        string: s.trim().to_string(),
                        vendor: vid,
                        model: pid,
                        serial,
                        interface_number,
                    })
                }
            }
            "GPI" => Ok(Self::Gpib {
                string: s.trim().to_string(),
            }),
            _ => Err(InstrumentError::AddressParsingError(format!(
                "'{s}' did not have a recognized VISA address"
            ))),
        }
    }
}

#[cfg(test)]
pub mod unit {

    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

    use super::{ConnectionInfo, Vendor};
    use crate::model::Model;

    fn multitest_connection_info_parse(cases: &[(&str, ConnectionInfo)]) {
        for c in cases {
            match c.0.parse::<ConnectionInfo>() {
                Ok(actual) => assert_eq!(
                    actual, c.1,
                    "'{}' did not convert to '{:?}' properly",
                    c.0, c.1
                ),
                Err(e) => panic!("'{}' could not be parsed into ConnectionInfo: {e}", c.0),
            }
        }
    }

    #[test]
    fn usb_resource_string_parsing() {
        multitest_connection_info_parse(&[
            (
                "USB0::0x5e6::0x2461::12345678::INSTR",
                ConnectionInfo::Usb {
                    string: "USB0::0x5e6::0x2461::12345678::INSTR".to_string(),
                    vendor: Vendor::Keithley,
                    model: Model::_2461,
                    serial: "12345678".to_string(),
                    interface_number: None,
                },
            ),
            (
                "USB0::0x699::0x5103::asdf::INSTR",
                ConnectionInfo::Usb {
                    string: "USB0::0x699::0x5103::asdf::INSTR".to_string(),
                    vendor: Vendor::Tektronix,
                    model: Model::MP5103,
                    serial: "asdf".to_string(),
                    interface_number: None,
                },
            ),
            (
                "USB0::0x699::0x2636::asdf::1::INSTR",
                ConnectionInfo::Usb {
                    string: "USB0::0x699::0x2636::asdf::1::INSTR".to_string(),
                    vendor: Vendor::Tektronix,
                    model: Model::_2636B,
                    serial: "asdf".to_string(),
                    interface_number: Some(1u16),
                },
            ),
        ]);
    }

    #[test]
    fn lan_parsing() {
        multitest_connection_info_parse(&[
            (
                "192.168.0.1",
                ConnectionInfo::Lan {
                    addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1)), 5025),
                },
            ),
            (
                "192.168.0.1:5",
                ConnectionInfo::Lan {
                    addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1)), 5),
                },
            ),
            (
                "2001:0db8:0000:0000:0000:ff00:0042:8329",
                ConnectionInfo::Lan {
                    addr: SocketAddr::new(
                        IpAddr::V6(Ipv6Addr::new(
                            0x2001, 0x0db8, 0x0, 0x0, 0x0, 0xff00, 0x0042, 0x8329,
                        )),
                        5025,
                    ),
                },
            ),
            (
                "2001:db8:0:0:0:ff00:42:8329",
                ConnectionInfo::Lan {
                    addr: SocketAddr::new(
                        IpAddr::V6(Ipv6Addr::new(
                            0x2001, 0x0db8, 0x0, 0x0, 0x0, 0xff00, 0x0042, 0x8329,
                        )),
                        5025,
                    ),
                },
            ),
            (
                "2001:db8::ff00:42:8329",
                ConnectionInfo::Lan {
                    addr: SocketAddr::new(
                        IpAddr::V6(Ipv6Addr::new(
                            0x2001, 0x0db8, 0x0, 0x0, 0x0, 0xff00, 0x0042, 0x8329,
                        )),
                        5025,
                    ),
                },
            ),
            (
                "[2001:db8::ff00:42:8329]:3",
                ConnectionInfo::Lan {
                    addr: SocketAddr::new(
                        IpAddr::V6(Ipv6Addr::new(
                            0x2001, 0x0db8, 0x0, 0x0, 0x0, 0xff00, 0x0042, 0x8329,
                        )),
                        3,
                    ),
                },
            ),
        ]);
    }

    #[test]
    fn visa_lan_parse() {
        multitest_connection_info_parse(&[
            (
                "TCPIP0::192.168.0.1::inst0::INSTR",
                ConnectionInfo::Vxi11 {
                    string: "TCPIP0::192.168.0.1::inst0::INSTR".to_string(),
                    addr: Ipv4Addr::new(192, 168, 0, 1),
                },
            ),
            (
                "TCPIP0::192.168.0.1::hislip0::INSTR",
                ConnectionInfo::HiSlip {
                    string: "TCPIP0::192.168.0.1::hislip0::INSTR".to_string(),
                    addr: IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1)),
                },
            ),
            (
                "TCPIP0::192.168.0.1::123::SOCKET",
                ConnectionInfo::VisaSocket {
                    string: "TCPIP0::192.168.0.1::123::SOCKET".to_string(),
                    addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1)), 123),
                },
            ),
        ]);
    }
}
