use std::{
    io::{BufRead, ErrorKind, Read, Write},
    time::Duration,
};

use bytes::Buf;
use indicatif::{ProgressBar, ProgressStyle};
use tracing::{self, error, trace};

use crate::{
    instrument::{
        self,
        authenticate::Authentication,
        info::InstrumentInfo,
        language::{CmdLanguage, Language},
        Abort, Info, Login, Reset, Script,
    },
    interface::{connection_addr::ConnectionInfo, NonBlock},
    model::Model,
    protocol::Protocol,
    Flash, InstrumentError,
};

pub struct Instrument {
    info: Option<InstrumentInfo>,
    protocol: Protocol,
    auth: Authentication,
}

impl Instrument {
    #[must_use]
    pub const fn is(info: &InstrumentInfo) -> bool {
        info.model.is_tti()
    }

    #[must_use]
    pub fn model_is(model: impl AsRef<str>) -> bool {
        let Ok(model) = model.as_ref().parse::<Model>() else {
            return false;
        };
        model.is_tti()
    }

    /// Connect to an instrument with the given connection information.
    ///
    /// # Errors
    /// There can be issues in creating the protocol from the given [`ConnectionInfo`].
    /// There can also be issues in getting the instrument information using
    /// [`ConnectionInfo::get_info()`].
    #[tracing::instrument(skip(conn, auth))]
    pub fn connect(conn: &ConnectionInfo, auth: Authentication) -> Result<Self, InstrumentError> {
        let protocol = Protocol::connect(conn)?;

        Ok(Self {
            info: None,
            protocol,
            auth,
        })
    }

    #[must_use]
    pub const fn new(protocol: Protocol, auth: Authentication) -> Self {
        Self {
            info: None,
            protocol,
            auth,
        }
    }

    pub fn add_info(&mut self, info: InstrumentInfo) -> &Self {
        self.info = Some(info);
        self
    }
}

//Implement device_interface::Interface since it is a subset of instrument::Instrument trait.
impl instrument::Instrument for Instrument {}

impl Info for Instrument {}

impl Language for Instrument {
    fn get_language(&mut self) -> Result<CmdLanguage, InstrumentError> {
        self.write_all(b"*LANG?\n")?;
        for _i in 0..5 {
            std::thread::sleep(Duration::from_millis(100));
            let mut lang: Vec<u8> = vec![0; 256];
            let read_size = match self.read(&mut lang) {
                Ok(read_size) => read_size,
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    continue;
                }
                Err(e) => {
                    error!("{e:?}: {e}");
                    return Err(e.into());
                }
            };
            let lang = &lang[0..read_size];
            let lang = std::str::from_utf8(lang).unwrap_or("").trim();

            if lang.contains("TSP") {
                return Ok(CmdLanguage::Tsp);
            } else if lang.contains("SCPI") {
                return Ok(CmdLanguage::Scpi);
            }
        }
        Err(InstrumentError::InformationRetrievalError {
            details: ("could not read language of the instrument").to_string(),
        })
    }

    fn change_language(&mut self, lang: CmdLanguage) -> Result<(), InstrumentError> {
        self.write_all(format!("*LANG {lang}\n").as_bytes())?;
        Ok(())
    }
}

impl Login for Instrument {
    fn check_login(&mut self) -> crate::error::Result<instrument::State> {
        self.write_all(b"*TST?\n")?;
        for _i in 0..5 {
            std::thread::sleep(Duration::from_millis(100));
            let mut resp: Vec<u8> = vec![0; 256];
            let read_size = match self.read(&mut resp) {
                Ok(read_size) => read_size,
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    continue;
                }
                Err(e) => {
                    error!("{e:?}: {e}");
                    return Err(e.into());
                }
            };
            let resp = &resp[..read_size];

            let resp = std::str::from_utf8(resp).unwrap_or("").trim();

            if resp.contains("SUCCESS: Logged in") || resp.contains('0') {
                return Ok(instrument::State::NotNeeded);
            }

            if resp.contains("FAILURE") {
                if resp.contains("LOGOUT") {
                    return Ok(instrument::State::LogoutNeeded);
                }
                return Ok(instrument::State::Needed);
            }
        }
        Ok(instrument::State::Needed)
    }

    fn login(&mut self) -> crate::error::Result<()> {
        let mut inst_login_state = self.check_login()?;
        if instrument::State::NotNeeded == inst_login_state {
            return Ok(());
        } else if instrument::State::LogoutNeeded == inst_login_state {
            return Err(InstrumentError::InterfaceLoginErr);
        }

        if let Some(password) = self.auth.read_password()? {
            self.write_all(format!("login {password}\n").as_bytes())?;
        }

        inst_login_state = self.check_login()?;
        if instrument::State::NotNeeded == inst_login_state {
            let info = self.info()?;
            self.auth
                .save_credential(&info.model, &info.serial_number)?;
        } else if instrument::State::Needed == inst_login_state {
            return Err(InstrumentError::LoginRejected);
        }

        Ok(())
    }
}

impl Script for Instrument {}

impl Flash for Instrument {
    fn flash_firmware(&mut self, image: &[u8], _: Option<u16>) -> crate::error::Result<()> {
        #[allow(irrefutable_let_patterns)] //This is marked as irrefutable when building without
        //visa
        let spinner = if let Protocol::Raw(_) = self.protocol {
            let pb = ProgressBar::new(1);
            #[allow(clippy::literal_string_with_formatting_args)]
            // This is a template for ProgressStyle that requires this syntax
            pb.set_style(
                ProgressStyle::with_template(" {spinner:.green} [{elapsed_precise}] {msg}")
                    .unwrap(),
            );
            pb.enable_steady_tick(Duration::from_millis(100));
            pb.set_message("Loading Firmware...");

            Some(pb)
        } else {
            None
        };
        let mut image = image.reader();

        self.write_all(b"localnode.prompts=localnode.DISABLE\n")?;
        self.write_all(b"if ki.upgrade ~= nil and ki.upgrade.noacklater ~= nil then ki.upgrade.noacklater() end\n")?;
        self.write_all(b"prevflash\n")?;

        self.write_all(image.fill_buf().unwrap())?;

        self.write_all(b"endflash\n")?;

        if let Some(pb) = spinner {
            pb.finish_with_message(
                "Firmware file transferred successfully. Upgrade running on instrument.",
            );
        } else {
            eprintln!("Firmware file transferred successfully. Upgrade running on instrument.");
        }
        Ok(())
    }
}

impl Read for Instrument {
    #[tracing::instrument(skip(self, buf))]
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let b = self.protocol.read(buf)?;
        let ascii = String::from_utf8_lossy(buf);
        let ascii = ascii.trim_end().trim_matches(['\0', '\n', '\r']);
        if !ascii.is_empty() {
            trace!("read from instrument: '{ascii}'");
        }
        Ok(b)
    }
}

impl Write for Instrument {
    #[tracing::instrument(skip(self, buf))]
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if String::from_utf8_lossy(buf).contains("login") {
            trace!("writing to instrument: 'login ****'");
        } else {
            trace!("writing to instrument: '{}'", String::from_utf8_lossy(buf));
        }
        self.protocol.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.protocol.flush()
    }

    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        self.protocol.write_all(buf)
    }
}

impl NonBlock for Instrument {
    fn set_nonblocking(&mut self, enable: bool) -> crate::error::Result<()> {
        match &mut self.protocol {
            Protocol::Raw(r) => r.set_nonblocking(enable),

            #[cfg(feature = "visa")]
            Protocol::Visa { .. } => Ok(()),
        }
    }
}

impl Drop for Instrument {
    #[tracing::instrument(skip(self))]
    fn drop(&mut self) {
        trace!("calling tti drop...");
        let _ = self.reset();
        let _ = self.write_all(b"logout\n");
        std::thread::sleep(Duration::from_millis(100));
    }
}

impl Reset for Instrument {
    fn reset(&mut self) -> crate::error::Result<()> {
        trace!("calling tti reset...");
        let _ = self.write_all(b"abort\n");
        std::thread::sleep(Duration::from_millis(100));
        let _ = self.write_all(b"*RST\n");
        std::thread::sleep(Duration::from_millis(100));
        Ok(())
    }
}

impl Abort for Instrument {
    #[tracing::instrument(skip(self))]
    fn abort(&mut self) -> crate::error::Result<()> {
        trace!("Calling tti abort...");
        let _ = self.write_all(b"abort\n");
        std::thread::sleep(Duration::from_millis(100));
        Ok(())
    }
}
#[cfg(test)]
mod unit {
    use std::{
        assert_matches::assert_matches,
        io::{BufRead, Read, Write},
    };

    use bytes::Buf;
    use mockall::{mock, Sequence};

    use crate::{
        instrument::{self, authenticate::Authentication, info::Info, Language, Login, Script},
        interface::{self, NonBlock},
        protocol::{self, raw::Raw},
        test_util, Flash, InstrumentError,
    };

    use super::Instrument;

    #[test]
    fn login_not_needed() {
        let mut interface = MockInterface::new();
        let mut seq = Sequence::new();

        interface.expect_flush().times(..).returning(|| Ok(()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"*TST?\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_read()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf.len() >= 2)
            .return_once(|buf: &mut [u8]| {
                let msg = b"0\n";
                if buf.len() >= msg.len() {
                    let bytes = msg[..]
                        .reader()
                        .read(buf)
                        .expect("MockInterface should write to buffer");
                    assert_eq!(bytes, msg.len());
                }
                Ok(msg.len())
            });

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"*TST?\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_read()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf.len() >= 2)
            .return_once(|buf: &mut [u8]| {
                let msg = b"0\n";
                if buf.len() >= msg.len() {
                    let bytes = msg[..]
                        .reader()
                        .read(buf)
                        .expect("MockInterface should write to buffer");
                    assert_eq!(bytes, msg.len());
                }
                Ok(msg.len())
            });
        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"logout\n")
            .returning(|buf: &[u8]| Ok(buf.len()));
        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"*RST\n")
            .returning(|buf: &[u8]| Ok(buf.len()));
        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"abort\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        let mut instrument: Instrument = Instrument::new(
            protocol::Protocol::Raw(Raw::new(interface)),
            Authentication::NoAuth,
        );

        assert_matches!(instrument.check_login(), Ok(instrument::State::NotNeeded));

        assert!(instrument.login().is_ok());
    }

    #[test]
    #[allow(clippy::too_many_lines)] //Allow for now
    fn login_success() {
        let mut interface = MockInterface::new();
        let mut seq = Sequence::new();

        interface.expect_flush().times(..).returning(|| Ok(()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"*TST?\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_read()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf.len() >= 8)
            .return_once(|buf: &mut [u8]| {
                let msg = b"FAILURE\n";
                if buf.len() >= msg.len() {
                    let bytes = msg[..]
                        .reader()
                        .read(buf)
                        .expect("MockInstrument should write to buffer");
                    assert_eq!(bytes, msg.len());
                }
                Ok(msg.len())
            });

        // login() { first check_login() }
        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"*TST?\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_read()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf.len() >= 8)
            .return_once(|buf: &mut [u8]| {
                let msg = b"FAILURE\n";
                if buf.len() >= msg.len() {
                    let bytes = msg[..]
                        .reader()
                        .read(buf)
                        .expect("MockInstrument should write to buffer");
                    assert_eq!(bytes, msg.len());
                }
                Ok(msg.len())
            });

        // login() {write(b"login {token}")}
        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"login secret_token\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        // login() { second check_login() }
        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"*TST?\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_read()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf.len() >= 8)
            .return_once(|buf: &mut [u8]| {
                let msg = b"0\n";
                if buf.len() >= msg.len() {
                    let bytes = msg[..]
                        .reader()
                        .read(buf)
                        .expect("MockInstrument should write to buffer");
                    assert_eq!(bytes, msg.len());
                }
                Ok(msg.len())
            });

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"*CLS\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"*IDN?\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_read()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf.len() >= 8)
            .return_once(|buf: &mut [u8]| {
                let msg = b"Keithley Instruments,MODEL 2450,00000000,1.1.1\n";
                if buf.len() >= msg.len() {
                    let bytes = msg[..]
                        .reader()
                        .read(buf)
                        .expect("MockInstrument should write to buffer");
                    assert_eq!(bytes, msg.len());
                }
                Ok(msg.len())
            });
        // check_login()
        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"*TST?\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_read()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf.len() >= 8)
            .return_once(|buf: &mut [u8]| {
                let msg = b"0\n";
                if buf.len() >= msg.len() {
                    let bytes = msg[..]
                        .reader()
                        .read(buf)
                        .expect("MockInstrument should write to buffer");
                    assert_eq!(bytes, msg.len());
                }
                Ok(msg.len())
            });

        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"logout\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"*RST\n")
            .returning(|buf: &[u8]| Ok(buf.len()));
        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"abort\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        let mut instrument: Instrument = Instrument::new(
            protocol::Protocol::Raw(Raw::new(interface)),
            Authentication::Credential {
                username: String::new(),
                password: "secret_token".to_string(),
            },
        );

        assert_matches!(instrument.check_login(), Ok(instrument::State::Needed));
        assert_matches!(instrument.login(), Ok(()));
        assert_matches!(instrument.check_login(), Ok(instrument::State::NotNeeded));
    }

    #[test]
    #[allow(clippy::too_many_lines)] //Allow for now
    fn login_failure() {
        let mut interface = MockInterface::new();
        let mut seq = Sequence::new();

        interface.expect_flush().times(..).returning(|| Ok(()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"*TST?\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_read()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf.len() >= 8)
            .return_once(|buf: &mut [u8]| {
                let msg = b"FAILURE\n";
                if buf.len() >= msg.len() {
                    let bytes = msg[..]
                        .reader()
                        .read(buf)
                        .expect("MockInstrument should write to buffer");
                    assert_eq!(bytes, msg.len());
                }
                Ok(msg.len())
            });

        // login() { first check_login() }
        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"*TST?\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_read()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf.len() >= 8)
            .return_once(|buf: &mut [u8]| {
                let msg = b"FAILURE\n";
                if buf.len() >= msg.len() {
                    let bytes = msg[..]
                        .reader()
                        .read(buf)
                        .expect("MockInstrument should write to buffer");
                    assert_eq!(bytes, msg.len());
                }
                Ok(msg.len())
            });

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"login secret_token\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        // login() { second check_login() }
        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"*TST?\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_read()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf.len() >= 8)
            .return_once(|buf: &mut [u8]| {
                let msg = b"FAILURE\n";
                if buf.len() >= msg.len() {
                    let bytes = msg[..]
                        .reader()
                        .read(buf)
                        .expect("MockInstrument should write to buffer");
                    assert_eq!(bytes, msg.len());
                }
                Ok(msg.len())
            });

        // check_login()
        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"*TST?\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_read()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf.len() >= 8)
            .return_once(|buf: &mut [u8]| {
                let msg = b"FAILURE\n";
                if buf.len() >= msg.len() {
                    let bytes = msg[..]
                        .reader()
                        .read(buf)
                        .expect("MockInstrument should write to buffer");
                    assert_eq!(bytes, msg.len());
                }
                Ok(msg.len())
            });
        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"logout\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"*RST\n")
            .returning(|buf: &[u8]| Ok(buf.len()));
        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"abort\n")
            .returning(|buf: &[u8]| Ok(buf.len()));
        let mut instrument: Instrument = Instrument::new(
            protocol::Protocol::Raw(Raw::new(interface)),
            Authentication::Credential {
                username: String::new(),
                password: "secret_token".to_string(),
            },
        );

        assert_matches!(instrument.check_login(), Ok(instrument::State::Needed));

        assert_matches!(instrument.login(), Err(InstrumentError::LoginRejected));

        assert_matches!(instrument.check_login(), Ok(instrument::State::Needed));
    }

    //#[test]
    //fn info() {
    //    let mut interface = MockInterface::new();
    //    let auth = MockAuthenticate::new();
    //    let mut seq = Sequence::new();

    //    interface.expect_flush().times(..).returning(|| Ok(()));

    //    interface
    //        .expect_set_nonblocking()
    //        .times(..)
    //        .returning(|_| Ok(()));

    //    // check_login()
    //    interface
    //        .expect_write()
    //        .times(1)
    //        .in_sequence(&mut seq)
    //        .withf(|buf: &[u8]| buf == b"*IDN?\n")
    //        .returning(|buf: &[u8]| Ok(buf.len()));

    //    interface
    //        .expect_read()
    //        .times(1)
    //        .in_sequence(&mut seq)
    //        .withf(|buf: &[u8]| buf.len() >= 50)
    //        .return_once(|buf: &mut [u8]| {
    //            let msg = b"KEITHLEY INSTRUMENTS,MODEL 2450,0123456789,1.2.3d\n";
    //            if buf.len() >= msg.len() {
    //                let bytes = msg[..]
    //                    .reader()
    //                    .read(buf)
    //                    .expect("MockInterface should write to buffer");
    //                assert_eq!(bytes, msg.len());
    //            }
    //            Ok(msg.len())
    //        });

    //    interface
    //        .expect_write()
    //        .times(..)
    //        .withf(|buf: &[u8]| buf == b"logout\n")
    //        .returning(|buf: &[u8]| Ok(buf.len()));

    //    interface
    //        .expect_write()
    //        .times(..)
    //        .withf(|buf: &[u8]| buf == b"*RST\n")
    //        .returning(|buf: &[u8]| Ok(buf.len()));
    //    interface
    //        .expect_write()
    //        .times(..)
    //        .withf(|buf: &[u8]| buf == b"abort\n")
    //        .returning(|buf: &[u8]| Ok(buf.len()));
    //    let mut instrument: Instrument =
    //        Instrument::new(protocol::Protocol::Raw(Box::new(interface)), Authentication::NoAuth);

    //    let info = instrument
    //        .info()
    //        .expect("instrument can get instrument information from MockInterface");

    //    let exp_vendor = "KEITHLEY INSTRUMENTS".to_string();
    //    let exp_model = "2450".to_string();
    //    let exp_serial = "0123456789".to_string();
    //    let exp_fw = "1.2.3d".to_string();

    //    assert_eq!(info.vendor.unwrap(), exp_vendor);
    //    assert_eq!(info.model.unwrap(), exp_model);
    //    assert_eq!(info.serial_number.unwrap(), exp_serial);
    //    assert_eq!(info.firmware_rev.unwrap(), exp_fw);
    //}

    #[test]
    fn get_language_tsp() {
        let mut interface = MockInterface::new();
        let mut seq = Sequence::new();

        interface.expect_flush().times(..).returning(|| Ok(()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"*LANG?\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_read()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf.len() >= 4)
            .return_once(|buf: &mut [u8]| {
                let msg = b"TSP\n";
                if buf.len() >= msg.len() {
                    let bytes = msg[..]
                        .reader()
                        .read(buf)
                        .expect("MockInterface should write to buffer");
                    assert_eq!(bytes, msg.len());
                }
                Ok(msg.len())
            });

        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"logout\n")
            .returning(|buf: &[u8]| Ok(buf.len()));
        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"*RST\n")
            .returning(|buf: &[u8]| Ok(buf.len()));
        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"abort\n")
            .returning(|buf: &[u8]| Ok(buf.len()));
        let mut instrument: Instrument = Instrument::new(
            protocol::Protocol::Raw(Raw::new(interface)),
            Authentication::NoAuth,
        );

        assert_eq!(
            instrument.get_language().unwrap(),
            instrument::CmdLanguage::Tsp
        );
    }

    #[test]
    fn get_language_scpi() {
        let mut interface = MockInterface::new();
        let mut seq = Sequence::new();

        interface.expect_flush().times(..).returning(|| Ok(()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"*LANG?\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_read()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf.len() >= 5)
            .return_once(|buf: &mut [u8]| {
                let msg = b"SCPI\n";
                if buf.len() >= msg.len() {
                    let bytes = msg[..]
                        .reader()
                        .read(buf)
                        .expect("MockInterface should write to buffer");
                    assert_eq!(bytes, msg.len());
                }
                Ok(msg.len())
            });

        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"logout\n")
            .returning(|buf: &[u8]| Ok(buf.len()));
        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"*RST\n")
            .returning(|buf: &[u8]| Ok(buf.len()));
        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"abort\n")
            .returning(|buf: &[u8]| Ok(buf.len()));
        let mut instrument: Instrument = Instrument::new(
            protocol::Protocol::Raw(Raw::new(interface)),
            Authentication::NoAuth,
        );

        assert_eq!(
            instrument.get_language().unwrap(),
            instrument::CmdLanguage::Scpi
        );
    }

    #[test]
    fn change_language() {
        let mut interface = MockInterface::new();
        let mut seq = Sequence::new();

        interface.expect_flush().times(..).returning(|| Ok(()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"*LANG TSP\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"*LANG SCPI\n")
            .returning(|buf: &[u8]| Ok(buf.len()));
        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"logout\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"*RST\n")
            .returning(|buf: &[u8]| Ok(buf.len()));
        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"abort\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        let mut instrument: Instrument = Instrument::new(
            protocol::Protocol::Raw(Raw::new(interface)),
            Authentication::NoAuth,
        );

        assert!(instrument
            .change_language(instrument::CmdLanguage::Tsp)
            .is_ok());

        assert!(instrument
            .change_language(instrument::CmdLanguage::Scpi)
            .is_ok());
    }

    #[test]
    fn write_script() {
        let optional_writes: Vec<Vec<u8>> = vec![
            (*b"logout\n").into(),
            (*b"*RST\n").into(),
            (*b"abort\n").into(),
            (*b"_orig_prompts = localnode.prompts localnode.prompts = 0\n").into(),
            (*b"localnode.prompts = _orig_prompts _orig_prompts = nil\n").into(),
        ];
        let expected: Vec<Vec<u8>> = vec![
            (*b"test_script=nil\n").into(),
            (*b"loadscript test_script\n").into(),
            (*b"line1\nline2\nline3").into(),
            (*b"\nendscript\n").into(),
        ];
        let mut interface = MockInterface::new();
        let mut seq = Sequence::new();

        //Accept any number of flushes
        interface.expect_flush().times(..).returning(|| Ok(()));

        for o in optional_writes {
            interface
                .expect_write()
                .times(..)
                .withf(move |buf: &[u8]| buf == o)
                .returning(|buf: &[u8]| Ok(buf.len()));
        }

        for e in expected {
            interface
                .expect_write()
                .times(1)
                .in_sequence(&mut seq)
                .withf(move |buf: &[u8]| buf == e)
                .returning(|buf: &[u8]| Ok(buf.len()));
        }
        let mut instrument: Instrument = Instrument::new(
            protocol::Protocol::Raw(Raw::new(interface)),
            Authentication::NoAuth,
        );

        instrument
            .write_script(b"test_script", &b"line1\nline2\nline3"[..], false, false)
            .expect("instrument should have written script to MockInterface");
    }

    #[test]
    fn write_script_run() {
        let optional_writes: Vec<Vec<u8>> = vec![
            (*b"logout\n").into(),
            (*b"*RST\n").into(),
            (*b"abort\n").into(),
            (*b"_orig_prompts = localnode.prompts localnode.prompts = 0\n").into(),
            (*b"localnode.prompts = _orig_prompts _orig_prompts = nil\n").into(),
        ];
        let expected: Vec<Vec<u8>> = vec![
            (*b"test_script=nil\n").into(),
            (*b"loadscript test_script\n").into(),
            (*b"line1\nline2\nline3").into(),
            (*b"\nendscript\n").into(),
            (*b"test_script.run()\n").into(),
        ];
        let mut interface = MockInterface::new();
        let mut seq = Sequence::new();

        //Accept any number of flushes
        interface.expect_flush().times(..).returning(|| Ok(()));

        for o in optional_writes {
            interface
                .expect_write()
                .times(..)
                .withf(move |buf: &[u8]| buf == o)
                .returning(|buf: &[u8]| Ok(buf.len()));
        }

        for e in expected {
            interface
                .expect_write()
                .times(1)
                .in_sequence(&mut seq)
                .withf(move |buf: &[u8]| buf == e)
                .returning(|buf: &[u8]| Ok(buf.len()));
        }
        let mut instrument: Instrument = Instrument::new(
            protocol::Protocol::Raw(Raw::new(interface)),
            Authentication::NoAuth,
        );

        instrument
            .write_script(b"test_script", &b"line1\nline2\nline3"[..], false, true)
            .expect("instrument should have written script to MockInterface");
    }

    #[test]
    fn write_script_save() {
        let optional_writes: Vec<Vec<u8>> = vec![
            (*b"logout\n").into(),
            (*b"*RST\n").into(),
            (*b"abort\n").into(),
            (*b"_orig_prompts = localnode.prompts localnode.prompts = 0\n").into(),
            (*b"localnode.prompts = _orig_prompts _orig_prompts = nil\n").into(),
        ];
        let expected: Vec<Vec<u8>> = vec![
            (*b"test_script=nil\n").into(),
            (*b"loadscript test_script\n").into(),
            (*b"line1\nline2\nline3").into(),
            (*b"\nendscript\n").into(),
            (*b"test_script.save()\n").into(),
        ];
        let mut interface = MockInterface::new();
        let mut seq = Sequence::new();

        //Accept any number of flushes
        interface.expect_flush().times(..).returning(|| Ok(()));

        for o in optional_writes {
            interface
                .expect_write()
                .times(..)
                .withf(move |buf: &[u8]| buf == o)
                .returning(|buf: &[u8]| Ok(buf.len()));
        }

        for e in expected {
            interface
                .expect_write()
                .times(1)
                .in_sequence(&mut seq)
                .withf(move |buf: &[u8]| buf == e)
                .returning(|buf: &[u8]| Ok(buf.len()));
        }

        let mut instrument: Instrument = Instrument::new(
            protocol::Protocol::Raw(Raw::new(interface)),
            Authentication::NoAuth,
        );

        instrument
            .write_script(b"test_script", &b"line1\nline2\nline3"[..], true, false)
            .expect("instrument should have written script to MockInterface");
    }

    #[test]
    fn write_script_save_run() {
        let optional_writes: Vec<Vec<u8>> = vec![
            (*b"logout\n").into(),
            (*b"*RST\n").into(),
            (*b"abort\n").into(),
            (*b"_orig_prompts = localnode.prompts localnode.prompts = 0\n").into(),
            (*b"localnode.prompts = _orig_prompts _orig_prompts = nil\n").into(),
        ];
        let expected: Vec<Vec<u8>> = vec![
            (*b"test_script=nil\n").into(),
            (*b"loadscript test_script\n").into(),
            (*b"line1\nline2\nline3").into(),
            (*b"\nendscript\n").into(),
            (*b"test_script.save()\n").into(),
            (*b"test_script.run()\n").into(),
        ];
        let mut interface = MockInterface::new();
        let mut seq = Sequence::new();

        //Accept any number of flushes
        interface.expect_flush().times(..).returning(|| Ok(()));

        for o in optional_writes {
            interface
                .expect_write()
                .times(..)
                .withf(move |buf: &[u8]| buf == o)
                .returning(|buf: &[u8]| Ok(buf.len()));
        }

        for e in expected {
            interface
                .expect_write()
                .times(1)
                .in_sequence(&mut seq)
                .withf(move |buf: &[u8]| buf == e)
                .returning(|buf: &[u8]| Ok(buf.len()));
        }

        let mut instrument: Instrument = Instrument::new(
            protocol::Protocol::Raw(Raw::new(interface)),
            Authentication::NoAuth,
        );

        instrument
            .write_script(b"test_script", &b"line1\nline2\nline3"[..], true, true)
            .expect("instrument should have written script to MockInterface");
    }

    #[test]
    fn flash_firmware() {
        let mut interface = MockInterface::new();
        let mut seq = Sequence::new();

        interface.expect_flush().times(..).returning(|| Ok(()));

        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| String::from_utf8_lossy(buf).contains("localnode.prompts"))
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"if ki.upgrade ~= nil and ki.upgrade.noacklater ~= nil then ki.upgrade.noacklater() end\n")
            .returning(|buf: &[u8]| Ok(buf.len()) );

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"prevflash\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(move |buf: &[u8]| {
                buf == test_util::SIMPLE_FAKE_BINARY_FW
                    .reader()
                    .fill_buf()
                    .unwrap()
            })
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"endflash\n")
            .returning(|buf: &[u8]| Ok(buf.len()));
        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"logout\n")
            .returning(|buf: &[u8]| Ok(buf.len()));
        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"*RST\n")
            .returning(|buf: &[u8]| Ok(buf.len()));
        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"abort\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        let mut instrument: Instrument = Instrument::new(
            protocol::Protocol::Raw(Raw::new(interface)),
            Authentication::NoAuth,
        );

        instrument
            .flash_firmware(test_util::SIMPLE_FAKE_BINARY_FW, Some(0))
            .expect("instrument should have written fw to MockInterface");
    }

    // Define a mock interface to be used in the tests above.
    mock! {
        Interface {}

        impl interface::Interface for Interface {}


        impl Read for Interface {
            fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize>;
        }

        impl Write for Interface {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize>;

            fn flush(&mut self) -> std::io::Result<()>;
        }

        impl NonBlock for Interface {
            fn set_nonblocking(&mut self, enable: bool) -> crate::error::Result<()>;
        }

        impl Info for Interface {}
    }
}
