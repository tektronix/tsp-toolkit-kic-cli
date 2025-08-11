//! Trait definitions that need to be satisfied for any instrument.

pub mod abort;
pub mod authenticate;
pub mod firmware;
pub mod info;
pub mod language;
pub mod login;
pub mod reset;
pub mod script;

use std::{
    io::{Read, Write},
    time::Duration,
};

use crate::interface::NonBlock;
use crate::{error::Result, InstrumentError};
pub use abort::Abort;
pub use firmware::Flash;
pub use info::Info;
pub use language::{CmdLanguage, Language};
pub use login::{Login, State};
pub use reset::Reset;
pub use script::Script;
use tracing::{debug, trace};

/// A marker trait that defines the traits any [`Instrument`] needs to have.
pub trait Instrument:
    Flash + Info + Language + Login + Script + Read + Write + NonBlock + Reset + Abort
{
}

/// Read the output until one of the strings in `one_of` is found
///
/// # Errors
/// This function may result in IO errors from trying to read from `rw`.
#[tracing::instrument(skip(rw))]
pub fn read_until<T: Read + Write + ?Sized>(
    rw: &mut T,
    one_of: &[String],
    max_attempts: usize,
    delay_between_attempts: Duration,
) -> Result<String> {
    let mut accumulate = String::new();
    for _ in 0..max_attempts {
        std::thread::sleep(delay_between_attempts);
        let mut buf: Vec<u8> = vec![0u8; 512];
        match rw.read(&mut buf) {
            Ok(_) => Ok(()),
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                continue;
            }
            Err(e) => Err(e),
        }?;
        let first_null = buf.iter().position(|&x| x == b'\0').unwrap_or(buf.len());
        let buf = &buf[..first_null];
        if !buf.is_empty() {
            accumulate = format!("{accumulate}{}", String::from_utf8_lossy(buf));
        }
        for s in one_of {
            if accumulate.contains(s) {
                return Ok(accumulate.trim().to_string());
            }
        }
    }
    Err(InstrumentError::Other(String::default()))
}

/// Read from a 'rw' until we are sure we have cleared the output queue.
///
/// # Warning
/// This functions calls a TSP command and therefore should not be used before
/// we know whether the instrument is in TSP mode (only applicable for TTI)
///
/// # Errors
/// Whatever can errors can occur with [`std::io::Read`], [`std::io::Write`] or
/// [`kic_lib::interface::NonBlock`].
#[tracing::instrument(skip(rw))]
pub fn clear_output_queue<T: Read + Write + ?Sized>(
    rw: &mut T,
    max_attempts: usize,
    delay_between_attempts: Duration,
) -> Result<()> {
    let timestamp = chrono::Utc::now().to_string();

    debug!("Sending print({timestamp})");
    let mut loop_count = 0;
    loop {
        match rw.write_all(format!("print(\"{timestamp}\")\n").as_bytes()) {
            Ok(_) => break,
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // The send buffer is full. Try this write again when the buffer might
                // have cleared out. Keep retrying until success.
                loop_count += 1;
                std::thread::sleep(delay_between_attempts);
                continue;
            }
            Err(e) => return Err(e.into()),
        };
    }
    trace!("Write successfully completed after {loop_count} attempts");

    match read_until(rw, &[timestamp], max_attempts, delay_between_attempts) {
        Ok(_) => Ok(()),
        Err(InstrumentError::Other(_)) => Err(InstrumentError::Other(
            "unable to clear instrument output queue".to_string(),
        )),
        Err(e) => Err(e),
    }
}
