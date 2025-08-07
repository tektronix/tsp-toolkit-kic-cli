use std::{
    io::{ErrorKind, Read, Write},
    time::Duration,
};

use indicatif::{ProgressBar, ProgressStyle};
use tracing::{error, trace};

use crate::{
    instrument::{
        self, authenticate::Authentication, info::InstrumentInfo, language, Abort, Info, Login,
        Reset, Script,
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
        info.model.is_3700_70x()
    }

    #[must_use]
    pub fn model_is(model: impl AsRef<str>) -> bool {
        let Ok(model) = model.as_ref().parse::<Model>() else {
            return false;
        };
        model.is_3700_70x()
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

impl language::Language for Instrument {}

impl Login for Instrument {
    fn check_login(&mut self) -> crate::error::Result<instrument::State> {
        self.write_all(b"print('unlocked')\n")?;
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

            if resp.contains("unlocked") {
                return Ok(instrument::State::NotNeeded);
            }
            if resp.contains("Port in use") {
                return Ok(instrument::State::LogoutNeeded);
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
            self.write_all(format!("password {password}\n").as_bytes())?;
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
    /*
    Note: The packet size and delay was experimentally obtianed here.
    Anything over 4096 chunk size and under 10ms delay causes the unit
    to hang up on the unpdate around 6800k.
     */
    fn flash_firmware(&mut self, image: &[u8], _: Option<u16>) -> crate::error::Result<()> {
        self.write_all(b"localnode.prompts = 0\n")?;
        self.write_all(b"prevflash\n")?;

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

        for chunk in image.chunks(4096) {
            self.write_all(chunk)?;
            std::thread::sleep(Duration::from_millis(10)); //The position and duration of this delay is intentional
        }

        std::thread::sleep(Duration::from_millis(10)); //The position and duration of this delay is intentional
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
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.protocol.read(buf)
    }
}

impl Write for Instrument {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
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
        trace!("calling ki3700 drop...");
        let _ = self.reset();
    }
}

impl Reset for Instrument {
    #[tracing::instrument(skip(self))]
    fn reset(&mut self) -> crate::error::Result<()> {
        trace!("calling ki3700 reset...");
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
        trace!("Calling ki3700 abort...");
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
        instrument::{self, authenticate::Authentication, info::Info, Login, Script},
        interface::{self, NonBlock},
        protocol::{self, raw::Raw},
        test_util, Flash, InstrumentError,
    };

    use super::Instrument;

    #[test]
    fn login_not_needed() {
        let mut interface = MockInterface::new();
        let mut seq = Sequence::new();

        // A successful login attempt on a TTI instrument is as follows:
        // 1. Instrument connects to interface
        // 2. Instrument sends "*STB?\n"
        // 3. Instrument reads from interface and receives status byte
        // 4. Instrument returns `instrument::State::NotNeeded`

        interface.expect_flush().times(..).returning(|| Ok(()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"print('unlocked')\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_read()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf.len() >= 2)
            .return_once(|buf: &mut [u8]| {
                let msg = b"unlocked\n";
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
            .withf(|buf: &[u8]| buf == b"print('unlocked')\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_read()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf.len() >= 2)
            .return_once(|buf: &mut [u8]| {
                let msg = b"unlocked\n";
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
            .withf(|buf: &[u8]| buf == b"password\n")
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
    #[allow(clippy::too_many_lines)] //Allow for now.
    fn login_success() {
        let mut _interface = MockInterface::new();
        let mut interface = MockInterface::new();

        let mut seq = Sequence::new();

        interface.expect_flush().times(..).returning(|| Ok(()));

        // check_login()
        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"print('unlocked')\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_read()
            .times(5)
            .withf(|buf: &[u8]| buf.len() >= 8)
            .returning(|buf: &mut [u8]| {
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
            .withf(|buf: &[u8]| buf == b"print('unlocked')\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_read()
            .times(5)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf.len() >= 8)
            .returning(|buf: &mut [u8]| {
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
            .withf(|buf: &[u8]| buf == b"password secret_token\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        // login() { second check_login() }
        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"print('unlocked')\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_read()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf.len() >= 8)
            .return_once(|buf: &mut [u8]| {
                let msg = b"unlocked\n";
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
                let msg = b"Keithley Instruments,MODEL 3706A,00000000,1.1.1\n";
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
            .withf(|buf: &[u8]| buf == b"print('unlocked')\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_read()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf.len() >= 8)
            .return_once(|buf: &mut [u8]| {
                let msg = b"unlocked\n";
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
            .withf(|buf: &[u8]| buf == b"password\n")
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
        let mut _interface = MockInterface::new();
        let mut interface = MockInterface::new();
        let mut seq = Sequence::new();

        interface.expect_flush().times(..).returning(|| Ok(()));
        // check_login()
        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"print('unlocked')\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_read()
            .times(5)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf.len() >= 8)
            .returning(|buf: &mut [u8]| {
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
            .withf(|buf: &[u8]| buf == b"print('unlocked')\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_read()
            .times(5)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf.len() >= 8)
            .returning(|buf: &mut [u8]| {
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
            .withf(|buf: &[u8]| buf == b"password secret_token\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        // login() { second check_login() }
        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"print('unlocked')\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_read()
            .times(5)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf.len() >= 8)
            .returning(|buf: &mut [u8]| {
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
            .withf(|buf: &[u8]| buf == b"print('unlocked')\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_read()
            .times(5)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf.len() >= 8)
            .returning(|buf: &mut [u8]| {
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
            .withf(|buf: &[u8]| buf == b"password\n")
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
    //    let mut _interface = MockInterface::new();
    //    let mut interface = MockInterface::new();
    //    let auth = MockAuthenticate::new();
    //    let mut seq = Sequence::new();

    //    interface.expect_flush().times(..).returning(|| Ok(()));
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
    //            let msg = b"KEITHLEY INSTRUMENTS INC.,MODEL 3706A,0123456789,1.2.3d\n";
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
    //        .withf(|buf: &[u8]| buf == b"password\n")
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

    //    let exp_vendor = "KEITHLEY INSTRUMENTS INC.".to_string();
    //    let exp_model = "3706A".to_string();
    //    let exp_serial = "0123456789".to_string();
    //    let exp_fw = "1.2.3d".to_string();

    //    assert_eq!(info.vendor.unwrap(), exp_vendor);
    //    assert_eq!(info.model.unwrap(), exp_model);
    //    assert_eq!(info.serial_number.unwrap(), exp_serial);
    //    assert_eq!(info.firmware_rev.unwrap(), exp_fw);
    //}

    #[test]
    fn write_script() {
        let mut interface = MockInterface::new();
        let mut seq = Sequence::new();

        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"_orig_prompts = localnode.prompts localnode.prompts = 0\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"localnode.prompts = _orig_prompts _orig_prompts = nil\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        //Accept any number of flushes
        interface.expect_flush().times(..).returning(|| Ok(()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"test_script=nil\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"loadscript test_script\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(move |buf: &[u8]| buf == b"line1\nline2\nline3"[..].reader().fill_buf().unwrap())
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"\nendscript\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"password\n")
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
            .write_script(b"test_script", &b"line1\nline2\nline3"[..], false, false)
            .expect("instrument should have written script to MockInterface");
    }

    #[test]
    fn write_script_run() {
        let mut interface = MockInterface::new();
        let mut seq = Sequence::new();

        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"_orig_prompts = localnode.prompts localnode.prompts = 0\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"localnode.prompts = _orig_prompts _orig_prompts = nil\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        //Accept any number of flushes
        interface.expect_flush().times(..).returning(|| Ok(()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"test_script=nil\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"loadscript test_script\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(move |buf: &[u8]| buf == b"line1\nline2\nline3"[..].reader().fill_buf().unwrap())
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"\nendscript\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"test_script.run()\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"password\n")
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
            .write_script(b"test_script", &b"line1\nline2\nline3"[..], false, true)
            .expect("instrument should have written script to MockInterface");
    }

    #[test]
    fn write_script_save() {
        let mut interface = MockInterface::new();
        let mut seq = Sequence::new();

        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"_orig_prompts = localnode.prompts localnode.prompts = 0\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"localnode.prompts = _orig_prompts _orig_prompts = nil\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        //Accept any number of flushes
        interface.expect_flush().times(..).returning(|| Ok(()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"test_script=nil\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"loadscript test_script\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(move |buf: &[u8]| buf == b"line1\nline2\nline3"[..].reader().fill_buf().unwrap())
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"\nendscript\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"test_script.save()\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"password\n")
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
            .write_script(b"test_script", &b"line1\nline2\nline3"[..], true, false)
            .expect("instrument should have written script to MockInterface");
    }

    #[test]
    fn write_script_save_run() {
        let mut interface = MockInterface::new();
        let mut seq = Sequence::new();
        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"_orig_prompts = localnode.prompts localnode.prompts = 0\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"localnode.prompts = _orig_prompts _orig_prompts = nil\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        //Accept any number of flushes
        interface.expect_flush().times(..).returning(|| Ok(()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"test_script=nil\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"loadscript test_script\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(move |buf: &[u8]| buf == b"line1\nline2\nline3"[..].reader().fill_buf().unwrap())
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"\nendscript\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"test_script.save()\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"test_script.run()\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"password\n")
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
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"localnode.prompts = 0\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

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
