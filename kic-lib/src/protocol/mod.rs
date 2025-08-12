use crate::interface::connection_addr::ConnectionInfo;
use crate::protocol::raw::Raw;
use std::{
    error::Error,
    fmt::Display,
    io::{Read, Write},
    net::TcpStream,
    time::Duration,
};

#[cfg(not(target_os = "macos"))]
use std::path::Path;

#[cfg(target_os = "linux")]
use std::path::PathBuf;

use crate::{InstrumentError, Interface};

#[allow(unused_imports)] // ProgressState is only used in the 'visa' feature
use indicatif::{ProgressBar, ProgressState, ProgressStyle};

#[allow(unused_imports)] // warn is only used in 'visa' feature
use tracing::{trace, warn};

#[cfg(feature = "visa")]
use visa_rs::{
    enums::{assert::AssertTrigPro, status::ErrorCode},
    flags::FlushMode,
};

/// Look for local installation of VISA.
///
/// # Returns
/// `true` if VISA is installed. `false` otherwise
///
/// # Panics
/// `parse::<PathBuf>()` is called and unwrapped, so it _shouldn't_ panic.
///
#[must_use]
pub fn is_visa_installed() -> bool {
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        let search_path =
            r"C:\Program Files (x86)\IVI Foundation\VISA\WinNT\Lib_x64\msc\visa64.lib";
        Path::new(search_path).exists()
    }
    #[cfg(target_os = "linux")]
    {
        let Some(search_paths) = std::env::var_os("LD_LIBRARY_PATH") else {
            return false;
        };
        let Ok(search_paths) = search_paths.into_string() else {
            return false;
        };
        for p in search_paths.split(':') {
            let Ok(mut dir) = Path::new(&p).read_dir() else {
                return false;
            };
            if dir.any(|e| {
                let Ok(e) = e else {
                    return false;
                };
                let Ok(f) = e.file_name().into_string() else {
                    return false;
                };

                //parse::<PathBuf> is infallible so unwrap is ok here.
                let path = p.parse::<PathBuf>().unwrap().join(f);

                path.file_stem()
                    .unwrap()
                    .to_string_lossy()
                    .contains("libvisa")
            }) {
                return true;
            }
        }
        false
    }
    #[cfg(target_os = "macos")]
    {
        false
    }
}

#[cfg(feature = "visa")]
pub mod visa;
#[cfg(feature = "visa")]
use crate::protocol::visa::Visa;

pub mod raw;

pub enum Protocol {
    Raw(Raw),

    #[cfg(feature = "visa")]
    Visa(Visa),
}

impl Protocol {
    /// Allows for the use of any [`Interface`] to be injected for testing. This creates
    /// a [`Protocol::Raw`] Protocol with the given [`Interface`].
    pub fn new(interface: impl Interface + 'static) -> Self {
        Self::Raw(Raw::new(interface))
    }

    /// Connects to the appropriate interface given a connection
    ///
    /// # Errors
    /// The errors that can occur are from each of the connection types: [`TcpStream`]
    /// and [`Visa`]
    pub fn connect(info: &ConnectionInfo) -> Result<Self, InstrumentError> {
        #[allow(unused_variables)]
        match info {
            ConnectionInfo::Lan { addr } => {
                let stream = TcpStream::connect(addr)?;
                stream.set_nonblocking(true)?;
                stream.set_write_timeout(Some(Duration::from_millis(1000)))?;
                stream.set_read_timeout(Some(Duration::from_millis(1000)))?;
                Ok(Self::Raw(Raw::new(stream)))
            }
            ConnectionInfo::Vxi11 { string, .. }
            | ConnectionInfo::HiSlip { string, .. }
            | ConnectionInfo::Usb { string, .. }
            | ConnectionInfo::Gpib { string, .. }
            | ConnectionInfo::VisaSocket { string, .. } => {
                #[cfg(feature = "visa")]
                {
                    use crate::interface::NonBlock;

                    let mut visa = Visa::new(string)?;
                    visa.set_nonblocking(true)?;
                    Ok(Self::Visa(visa))
                }
                #[cfg(not(feature = "visa"))]
                {
                    Err(InstrumentError::NoVisa)
                }
            }
        }
    }
}

pub mod stb;

pub trait ReadStb {
    type Error: Display + Error;
    /// # Errors
    /// The errors returned must be of, or convertible to the type `Self::Error`.
    fn read_stb(&mut self) -> core::result::Result<stb::Stb, Self::Error> {
        Ok(stb::Stb::NotSupported)
    }
}

pub trait Clear {
    type Error: Display + Error;
    /// # Errors
    /// The errors returned must be of, or convertible to the type `Self::Error`.
    fn clear(&mut self) -> core::result::Result<(), Self::Error>;
}

pub trait Trigger {
    type Error: Display + Error;

    /// # Errors
    /// The errors returned must be of, or convertible to the type `Self::Error`.
    fn trigger(&mut self) -> core::result::Result<(), Self::Error>;
}

impl Read for Protocol {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::Raw(r) => r.read(buf),

            #[cfg(feature = "visa")]
            Self::Visa(v) => v.read(buf),
        }
    }
}

impl Write for Protocol {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        trace!("writing to instrument: '{}'", String::from_utf8_lossy(buf));
        match self {
            Self::Raw(r) => r.write(buf),

            #[cfg(feature = "visa")]
            Self::Visa(v) => v.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::Raw(r) => r.flush(),

            #[cfg(feature = "visa")]
            Self::Visa(v) => match v.visa_flush(FlushMode::IO_OUT_BUF) {
                Ok(v) => Ok(v),
                // viFlush(instrument, VI_IO_OUT_BUF) on USB throws this error, but we
                // can just ignore it.
                Err(e) if ErrorCode::from(e) == ErrorCode::ErrorInvMask => Ok(()),
                Err(e) => Err(std::io::Error::other(format!("VISA flush error: {e}"))),
            },
        }
    }

    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        // fit as much into a 1000-byte message as possible (For USBTMC)

        let mut start: usize = 0;

        let step: usize = match self {
            Self::Raw(_) => buf.len(),

            #[cfg(feature = "visa")]
            Self::Visa(_) => 1000, //TODO Need a way to make this 4500 for Treb and 1000 for
                                   //everything else.
        };
        let mut end: usize = if start.saturating_add(step) < buf.len() {
            start.saturating_add(step)
        } else {
            buf.len().saturating_sub(1)
        };
        let pb: Option<ProgressBar> = if buf.len() > 100_000 {
            match self {
                Self::Raw(_) => None,
                #[cfg(feature = "visa")]
                Self::Visa { .. } => {
                    // Only make progress bar for VISA connections and for messages > 100_000 bytes
                    let pb = ProgressBar::new(buf.len().try_into().unwrap_or_default());
                    #[allow(clippy::literal_string_with_formatting_args)] // This is a template for ProgressStyle that requires this syntax
                    pb.set_style(ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{bar:10.cyan/blue}] {bytes}/{total_bytes} (ETA: {eta}) {msg}").unwrap().with_key("eta", |state: &ProgressState, w: &mut dyn std::fmt::Write| write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap()));
                    pb.set_message("Loading firmware...");
                    Some(pb)
                }
            }
        } else {
            None
        };

        while end < buf.len().saturating_sub(1) {
            //Here we are trusting that a single line will not be more than 1000-bytes long
            let mut last_newline = end;
            // if the file is NOT a ZIP file, look for lines, otherwise, just obey chunking
            if buf[0..4] != [0x50, 0x4B, 0x03, 0x04] {
                while buf[last_newline] != b'\n' && last_newline > start {
                    last_newline = last_newline.saturating_sub(1);
                }
            }
            trace!("start: {start}, end: {end}, len: {}", buf.len());
            if start != last_newline {
                end = last_newline;
            }

            self.write(&buf[start..=end])?;

            if let Some(p) = pb.as_ref() {
                p.set_position(end.try_into().unwrap_or_default());
            }
            start = end.saturating_add(1);
            end = if start.saturating_add(step) < buf.len() {
                start.saturating_add(step)
            } else {
                buf.len().saturating_sub(1)
            };
        }

        //  write the last chunk
        if start == end {
            self.write(&[buf[start]])?;
        } else {
            self.write(&buf[start..=end])?;
        }
        if let Some(p) = pb {
            p.set_style(
                ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] {msg}").unwrap(),
            );
            p.finish_with_message("Loading firmware complete");
            //eprintln!("Loading firmware complete");
        }

        Ok(())
    }
}

impl Clear for Protocol {
    type Error = InstrumentError;
    fn clear(&mut self) -> core::result::Result<(), Self::Error> {
        match self {
            Self::Raw(r) => r.write_all(b"*CLS\n")?,

            #[cfg(feature = "visa")]
            Self::Visa(v) => v.clear()?,
        }

        Ok(())
    }
}

impl ReadStb for Protocol {
    type Error = InstrumentError;
    fn read_stb(&mut self) -> core::result::Result<stb::Stb, Self::Error> {
        match self {
            Self::Raw(_) => Ok(stb::Stb::NotSupported),

            #[cfg(feature = "visa")]
            Self::Visa(v) => Ok(stb::Stb::Stb(v.read_stb()?)),
        }
    }
}

impl Trigger for Protocol {
    type Error = InstrumentError;
    fn trigger(&mut self) -> core::result::Result<(), Self::Error> {
        match self {
            Self::Raw(r) => {
                r.write_all(b"*TRG\n")?;
            }

            #[cfg(feature = "visa")]
            Self::Visa(v) => {
                v.assert_trigger(AssertTrigPro::TrigProtDefault)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod unit {
    use std::assert_matches::assert_matches;

    use crate::protocol::stb::Stb;

    #[test]
    fn stb_test_mav() {
        let input = 0x0010;

        let actual = Stb::Stb(input).message_available();

        assert_matches!(actual, Ok(true));
    }

    #[test]
    fn stb_test_esr() {
        let input = 0x0020;

        let actual = Stb::Stb(input).event_summary();

        assert_matches!(actual, Ok(true));
    }

    #[test]
    fn stb_test_srq() {
        let input = 0x0040;

        let actual = Stb::Stb(input).srq();

        assert_matches!(actual, Ok(true));
    }

    #[test]
    fn stb_test_all() {
        for i in 0..=u16::MAX {
            let stb = Stb::Stb(i);
            //MAV
            if i & 0x0010 != 0 {
                assert_matches!(
                    stb.message_available(),
                    Ok(true),
                    "mav should be set - stb: {i:0>4x}"
                );
            } else {
                assert_matches!(
                    stb.message_available(),
                    Ok(false),
                    "mav should be unset - stb: {i:0>4x}"
                );
            }

            //ESR
            if i & 0x0020 != 0 {
                assert_matches!(
                    stb.event_summary(),
                    Ok(true),
                    "esr should be set - stb: {i:0>4x}"
                );
            } else {
                assert_matches!(
                    stb.event_summary(),
                    Ok(false),
                    "esr should be unset - stb: {i:0>4x}"
                );
            }

            //SRQ
            if i & 0x0040 != 0 {
                assert_matches!(stb.srq(), Ok(true), "srq should be set - stb: {i:0>4x}");
            } else {
                assert_matches!(stb.srq(), Ok(false), "srq should be unset - stb: {i:0>4x}");
            }
        }
    }
}
