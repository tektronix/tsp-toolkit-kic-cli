//! Trait definitions that need to be satisfied for any instrument.

pub mod firmware;
pub mod info;
pub mod language;
pub mod login;
pub mod script;

use std::{
    io::{Read, Write},
    time::Duration,
};

pub use firmware::Flash;
pub use info::Info;
pub use language::{CmdLanguage, Language};
pub use login::{Login, State};
pub use script::Script;

use crate::interface::NonBlock;
use crate::{error::Result, InstrumentError};

/// A marker trait that defines the traits any [`Instrument`] needs to have.
pub trait Instrument: Flash + Info + Language + Login + Script + Read + Write + NonBlock {}

/// Read from a 'rw' until we are sure we have cleared the output queue.
///
/// # Errors
/// Whatever can errors can occur with [`std::io::Read`], [`std::io::Write`] or
/// [`tsp-instrument::interface::NonBlock`].
pub fn clear_output_queue<T: Read + Write + NonBlock + ?Sized>(
    rw: &mut T,
    max_attempts: usize,
    delay_between_attempts: Duration,
) -> Result<()> {
    let timestamp = chrono::Utc::now().to_string();

    rw.write_all(format!("print(\"{timestamp}\")\n").as_bytes())?;

    rw.set_nonblocking(true)?;

    let mut accumulate = String::new();
    for _ in 0..max_attempts {
        std::thread::sleep(delay_between_attempts);
        let mut buf: Vec<u8> = vec![0u8; 512];
        match rw.read(&mut buf) {
            Ok(_) => Ok(()),
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(delay_between_attempts);
                continue;
            }
            Err(e) => Err(e),
        }?;
        let first_null = buf.iter().position(|&x| x == b'\0').unwrap_or(buf.len());
        let buf = &buf[..first_null];
        if !buf.is_empty() {
            accumulate = format!("{accumulate}{}", String::from_utf8_lossy(buf));
        }
        if accumulate.contains(&timestamp) {
            return Ok(());
        }
    }
    Err(InstrumentError::Other(
        "unable to clear instrument output queue".to_string(),
    ))
}
