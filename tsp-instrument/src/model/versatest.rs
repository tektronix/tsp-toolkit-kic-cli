use std::{
    io::{BufRead, Read, Write},
    time::Duration,
};

use crate::{
    instrument::{self, info::InstrumentInfo, language, Info, Login, Script},
    interface::Interface,
    interface::NonBlock,
    Flash, InstrumentError,
};
use bytes::Buf;
use language::Language;
use serde::{Deserialize, Serialize};

impl Instrument {
    #[must_use]
    pub fn is(info: &InstrumentInfo) -> bool {
        info.model.as_ref().map_or(false, is_versatest)
    }

    #[must_use]
    pub const fn new(interface: Box<dyn Interface>) -> Self {
        Self {
            info: None,
            interface,
        }
    }

    pub fn add_info(&mut self, info: InstrumentInfo) -> &Self {
        self.info = Some(info);
        self
    }
}

fn is_versatest(model: impl AsRef<str>) -> bool {
    ["VERSATEST-600", "TSPop"].contains(&model.as_ref())
}

pub struct Instrument {
    info: Option<InstrumentInfo>,
    interface: Box<dyn Interface>,
}

//Implement device_interface::Interface since it is a subset of instrument::Instrument trait.
impl instrument::Instrument for Instrument {}

impl Info for Instrument {}

impl Language for Instrument {}

impl Login for Instrument {
    fn check_login(&mut self) -> crate::error::Result<instrument::State> {
        self.write_all(b"print('unlocked')\n")?;

        let mut resp: Vec<u8> = vec![0; 256];
        let _read = self.read(&mut resp)?;

        let resp = std::str::from_utf8(resp.as_slice())
            .unwrap_or("")
            .trim_matches(char::from(0))
            .trim();

        if resp.contains("FAILURE") {
            Ok(instrument::State::Needed)
        } else {
            Ok(instrument::State::NotNeeded)
        }
    }

    fn login(&mut self, token: &[u8]) -> crate::error::Result<()> {
        if instrument::State::NotNeeded == self.check_login()? {
            return Ok(());
        }

        self.write_all(format!("password {}\n", String::from_utf8_lossy(token)).as_bytes())?;

        if instrument::State::Needed == self.check_login()? {
            return Err(InstrumentError::LoginRejected);
        }

        Ok(())
    }
}

impl Script for Instrument {}

impl Read for Instrument {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.interface.read(buf)
    }
}

impl Write for Instrument {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.interface.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.interface.flush()
    }
}

/// The information necessary to flash an instrument.
#[derive(Serialize, Deserialize, Debug)]
struct FirmwareInfo {
    /// For VersaTest only: `true` - the firmware is for a module; `false`: the firmware is for the mainframe
    #[serde(rename = "IsModule")]
    is_module: bool,

    /// For VersaTest only: The slot number of the module to update.
    #[serde(rename = "Slot")]
    slot: u8,
}
pub const VERSATEST_FLASH_UTIL_STR: &[u8] = include_bytes!("resources/flashUtil.tsp");
impl Flash for Instrument {
    fn flash_firmware(
        &mut self,
        image: &[u8],
        firmware_info: Option<u16>,
    ) -> crate::error::Result<()> {
        let mut is_module = false;
        let slot_number: u16 = firmware_info.unwrap_or(0);
        if slot_number > 0 {
            is_module = true;
        }

        //TODO This is temporary: Only use while not defined in FW
        if is_module {
            self.write_script(b"FlashUtil", VERSATEST_FLASH_UTIL_STR, false, true)?;
        }
        //.update {"FileName": "C:/Users/esarver1/Downloads/trebuchet-mainframe-sd-225642.x", "IsModule": false, "Slot": 1}
        //.update {"FileName": "C:/Users/esarver1/Downloads/kingarthur-module-225665.x", "IsModule": true, "Slot": 1}

        self.write_all(b"localnode.prompts=0\n")?;
        let image = image.reader();
        self.write_all(b"flash\n")?;

        for line in image.lines() {
            self.write_all(format!("{}\n", line?).as_bytes())?;
        }
        self.write_all(b"endflash\n")?;

        if is_module {
            //TODO This is temporary: Only use while not defined in FW
            self.write_all(b"FlashUtil()\n")?;
            self.write_all(format!("flashupdate(slot[{slot_number}])\n").as_bytes())?;

            let flash_util_global_functions = [b"flashupdate", b"flashverify", b"flashencode"];

            for func in flash_util_global_functions {
                //wait before deleting functions
                std::thread::sleep(Duration::from_millis(100));
                let _ =
                    self.write_all(format!("{} = nil\n", String::from_utf8_lossy(func)).as_bytes());
            }

            let script_name = "FlashUtil";
            self.write_all(format!("{script_name} = nil\n").as_bytes())?;
            //TODO use this when the FW team has implemented it:
            // self.write(format!("slot[{slot_number}].firmware.update()\n").as_bytes());
        } else {
            //Update Mainframe
            self.write_all(b"firmware.update()\n")?;
        }
        //self.write("localnode.prompts=1\n".to_string().as_bytes());

        Ok(())
    }
}

impl NonBlock for Instrument {
    fn set_nonblocking(&mut self, enable: bool) -> crate::error::Result<()> {
        self.interface.set_nonblocking(enable)
    }
}

impl Drop for Instrument {
    fn drop(&mut self) {
        let _ = self.interface.write_all(b"abort\n");
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
        instrument::{self, info::Info, Login, Script},
        interface::{self, NonBlock},
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
            .withf(|buf: &[u8]| buf == b"abort\n")
            .returning(|buf: &[u8]| Ok(buf.len()));
        let mut instrument: Instrument = Instrument::new(Box::new(interface));

        assert_matches!(instrument.check_login(), Ok(instrument::State::NotNeeded));

        assert!(instrument.login(b"secret_token").is_ok());
    }

    #[test]
    #[allow(clippy::too_many_lines)] //Allow for now.
    fn login_success() {
        let mut interface = MockInterface::new();

        let mut seq = Sequence::new();

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
            .withf(|buf: &[u8]| buf == b"abort\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        let mut instrument: Instrument = Instrument::new(Box::new(interface));

        assert_matches!(instrument.check_login(), Ok(instrument::State::Needed));

        assert_matches!(instrument.login(b"secret_token"), Ok(()));

        assert_matches!(instrument.check_login(), Ok(instrument::State::NotNeeded));
    }

    #[test]
    #[allow(clippy::too_many_lines)] //Allow for now
    fn login_failure() {
        let mut interface = MockInterface::new();

        let mut seq = Sequence::new();

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
            .withf(|buf: &[u8]| buf == b"abort\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        let mut instrument: Instrument = Instrument::new(Box::new(interface));

        assert_matches!(instrument.check_login(), Ok(instrument::State::Needed));

        assert_matches!(
            instrument.login(b"secret_token"),
            Err(InstrumentError::LoginRejected)
        );

        assert_matches!(instrument.check_login(), Ok(instrument::State::Needed));
    }

    #[test]
    fn info() {
        let mut interface = MockInterface::new();

        let mut seq = Sequence::new();

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
            .withf(|buf: &[u8]| buf.len() >= 50)
            .return_once(|buf: &mut [u8]| {
                let msg = b"KEITHLEY INSTRUMENTS,MODEL 2450,0123456789,1.2.3d\n";
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
            .withf(|buf: &[u8]| buf == b"abort\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        let mut instrument: Instrument = Instrument::new(Box::new(interface));

        let info = instrument
            .info()
            .expect("instrument can get instrument information from MockInterface");

        let exp_vendor = "KEITHLEY INSTRUMENTS".to_string();
        let exp_model = "2450".to_string();
        let exp_serial = "0123456789".to_string();
        let exp_fw = "1.2.3d".to_string();

        assert_eq!(info.vendor.unwrap(), exp_vendor);
        assert_eq!(info.model.unwrap(), exp_model);
        assert_eq!(info.serial_number.unwrap(), exp_serial);
        assert_eq!(info.firmware_rev.unwrap(), exp_fw);
    }

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
            .withf(|buf: &[u8]| buf == b"line1\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"line2\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"line3\n")
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
            .withf(|buf: &[u8]| buf == b"abort\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        let mut instrument: Instrument = Instrument::new(Box::new(interface));

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
            .withf(|buf: &[u8]| buf == b"line1\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"line2\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"line3\n")
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
            .withf(|buf: &[u8]| buf == b"abort\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        let mut instrument: Instrument = Instrument::new(Box::new(interface));

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
            .withf(|buf: &[u8]| buf == b"line1\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"line2\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"line3\n")
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
            .withf(|buf: &[u8]| buf == b"abort\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        let mut instrument: Instrument = Instrument::new(Box::new(interface));

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
            .withf(|buf: &[u8]| buf == b"line1\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"line2\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"line3\n")
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
            .withf(|buf: &[u8]| buf == b"abort\n")
            .returning(|buf: &[u8]| Ok(buf.len()));
        let mut instrument: Instrument = Instrument::new(Box::new(interface));

        instrument
            .write_script(b"test_script", &b"line1\nline2\nline3"[..], true, true)
            .expect("instrument should have written script to MockInterface");
    }

    #[test]
    fn flash_firmware() {
        let mut interface = MockInterface::new();

        let mut seq = Sequence::new();

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"localnode.prompts=0\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"flash\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        for line in test_util::SIMPLE_FAKE_TEXTUAL_FW.reader().lines() {
            interface
                .expect_write()
                .times(1)
                .in_sequence(&mut seq)
                .withf(move |buf: &[u8]| {
                    buf == format!(
                        "{}\n",
                        line.as_ref()
                            .expect("textual test firmware should return all Ok for lines()")
                    )
                    .as_bytes()
                })
                .returning(|buf: &[u8]| Ok(buf.len()));
        }

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"endflash\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        interface
            .expect_write()
            .times(1)
            .in_sequence(&mut seq)
            .withf(|buf: &[u8]| buf == b"firmware.update()\n")
            .returning(|buf: &[u8]| Ok(buf.len()));
        interface
            .expect_write()
            .times(..)
            .withf(|buf: &[u8]| buf == b"abort\n")
            .returning(|buf: &[u8]| Ok(buf.len()));

        let mut instrument: Instrument = Instrument::new(Box::new(interface));

        instrument
            .flash_firmware(test_util::SIMPLE_FAKE_TEXTUAL_FW, Some(0))
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
    }
}
