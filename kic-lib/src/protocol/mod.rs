use crate::interface::connection_addr::ConnectionInfo;
use crate::protocol::raw::Raw;
use std::{
    error::Error,
    fmt::Display,
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
    sync::Arc,
    time::Duration,
};

#[cfg(not(target_os = "macos"))]
use std::path::Path;

#[cfg(target_os = "linux")]
use std::path::PathBuf;

use crate::{InstrumentError, Interface};

#[allow(unused_imports)] // ProgressState is only used in the 'visa' feature
use indicatif::{ProgressBar, ProgressState, ProgressStyle};

use rustls::{
    crypto::{aws_lc_rs, verify_tls12_signature, verify_tls13_signature, CryptoProvider},
    server::VerifierBuilderError,
    RootCertStore,
};
#[allow(unused_imports)] // warn is only used in 'visa' feature
use tracing::{debug, error, trace, warn};

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

/// A struct to not do any Certificate validation since the instruments create
/// self-signed certs that don't have a known certificate authority.
#[derive(Debug)]
struct NoCertificateVerification(CryptoProvider);

impl NoCertificateVerification {
    fn new(provider: CryptoProvider) -> Self {
        Self(provider)
    }
}

impl rustls::client::danger::ServerCertVerifier for NoCertificateVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        verify_tls12_signature(
            message,
            cert,
            dss,
            &self.0.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        verify_tls13_signature(
            message,
            cert,
            dss,
            &self.0.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        self.0.signature_verification_algorithms.supported_schemes()
    }
}

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

    #[tracing::instrument]
    fn try_tls_lan_connection(addr: &SocketAddr) -> Result<Self, InstrumentError> {
        trace!("Trying TLS connection");
        let mut sock = TcpStream::connect(addr)?;

        let config = rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(NoCertificateVerification::new(
                aws_lc_rs::default_provider(),
            )))
            .with_no_client_auth();

        let conn = Arc::new(config);
        let mut conn = rustls::ClientConnection::new(conn, addr.ip().into())?;
        let (comp_read, comp_write) = conn.complete_io(&mut sock)?;

        trace!("comp_read: {comp_read}, comp_write: {comp_write}");

        sock.set_nonblocking(true)?;
        sock.set_write_timeout(Some(Duration::from_millis(1000)))?;
        sock.set_read_timeout(Some(Duration::from_millis(1000)))?;
        Ok(Self::Raw(Raw::new(rustls::StreamOwned::new(conn, sock))))
    }

    fn try_lan_connection(addr: &SocketAddr) -> Result<Self, InstrumentError> {
        let stream = TcpStream::connect(addr)?;
        stream.set_nonblocking(true)?;
        stream.set_write_timeout(Some(Duration::from_millis(1000)))?;
        stream.set_read_timeout(Some(Duration::from_millis(1000)))?;
        Ok(Self::Raw(Raw::new(stream)))
    }

    /// Connects to the appropriate interface given a connection
    ///
    /// # Errors
    /// The errors that can occur are from each of the connection types: [`TcpStream`]
    /// and [`Visa`]
    pub fn connect(info: &ConnectionInfo) -> Result<Self, InstrumentError> {
        #[allow(unused_variables)]
        match info {
            ConnectionInfo::Lan { tls_addr, addr } => {
                // Attempt to connect with a tls addr (normally port 5026) if available
                if let Some(tls) = tls_addr {
                    match Self::try_tls_lan_connection(tls) {
                        Ok(tls_stream) => {
                            trace!("TLS connection succeeded");
                            Ok(tls_stream)
                        }
                        Err(e) => {
                            trace!(
                                "TLS connection failed: {e}, falling back to non-TLS connection"
                            );
                            // If TLS fails, use TcpStream
                            Ok(Self::try_lan_connection(addr)?)
                        }
                    }
                } else {
                    trace!("No TLS address supplied, connecting without TLS");
                    Ok(Self::try_lan_connection(addr)?)
                }
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
        let bytes = match self {
            Self::Raw(r) => r.read(buf),

            #[cfg(feature = "visa")]
            Self::Visa(v) => v.read(buf),
        };
        let ascii = String::from_utf8_lossy(buf);
        let ascii = ascii.trim_end().trim_matches(['\0', '\n', '\r']);
        if !ascii.is_empty() {
            trace!("read from instrument: '{ascii}'");
        }
        bytes
    }
}

impl Write for Protocol {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        const WRITE_ATTEMPT_LIMIT: u16 = 10000;
        trace!(
            "writing to instrument ({} bytes): '{}'",
            buf.len(),
            String::from_utf8_lossy(buf)
        );

        let mut attempts = 0;
        loop {
            let res = match self {
                Self::Raw(r) => r.write(buf),

                #[cfg(feature = "visa")]
                Self::Visa(v) => v.write(buf),
            };

            match res {
                Ok(n) => {
                    // Partial writes are valid — return immediately
                    return Ok(n);
                }

                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    attempts += 1;

                    if attempts == 1 {
                        trace!("write would-block: entering retry loop");
                    }

                    if attempts >= WRITE_ATTEMPT_LIMIT {
                        error!("Write failed after {attempts} attempts (WouldBlock)");
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::WouldBlock,
                            "write retry limit exceeded",
                        ));
                    }
                    std::thread::sleep(Duration::from_micros(1000));
                }

                Err(e) => return Err(e),
            }
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
        use std::io::{Error, ErrorKind, Write};
        use std::time::Duration;

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
                    pb.set_style(
                        ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{bar:10.cyan/blue}] {bytes}/{total_bytes} (ETA: {eta}) {msg}")
                            .unwrap()
                            .with_key("eta", |state: &ProgressState, w: &mut dyn std::fmt::Write| {
                                write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap()
                            }),
                    );
                    pb.set_message("Loading to instrument...");
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
            if buf.len() >= 4 && buf[0..4] != [0x50, 0x4B, 0x03, 0x04] {
                while buf[last_newline] != b'\n' && last_newline > start {
                    last_newline = last_newline.saturating_sub(1);
                }
            }
            trace!("start: {start}, end: {end}, len: {}", buf.len());
            if start != last_newline {
                end = last_newline;
            }

            // Count bytes to ensure entire chunk is written
            let mut offset = 0;
            let chunk = &buf[start..=end];

            while offset < chunk.len() {
                match self.write(&chunk[offset..]) {
                    Ok(0) => {
                        return Err(Error::new(
                            ErrorKind::WriteZero,
                            "failed to write to underlying transport",
                        ));
                    }
                    Ok(n) => {
                        offset += n;
                    }
                    Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Err(e) => return Err(e),
                }
            }

            // progress only advances after full chunk written
            if let Some(p) = pb.as_ref() {
                p.set_position((end + 1).try_into().unwrap_or_default());
            }
            start = end.saturating_add(1);
            end = if start.saturating_add(step) < buf.len() {
                start.saturating_add(step)
            } else {
                buf.len().saturating_sub(1)
            };
        }

        // write the final chunk safely
        if !buf.is_empty() && start < buf.len() {
            let chunk = &buf[start..=end];
            let mut offset = 0;

            while offset < chunk.len() {
                match self.write(&chunk[offset..]) {
                    Ok(0) => {
                        return Err(Error::new(
                            ErrorKind::WriteZero,
                            "failed to write final chunk",
                        ));
                    }
                    Ok(n) => offset += n,
                    Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Err(e) => return Err(e),
                }
            }
        }

        if let Some(p) = pb {
            p.set_style(
                ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] {msg}").unwrap(),
            );
            p.finish_with_message("Loading complete");
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
    use assert_matches::assert_matches;

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
