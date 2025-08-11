//! Trait that allows aborting current operation on the instrument.

use std::io::Write;

use crate::error::Result;

/// The current operation on the [`Instrument`] can be aborted.
pub trait Abort
where
    Self: Write,
{
    /// Abort current operation on the instrument.
    ///
    /// # Notes
    /// - Abort current operation on the instrument using 'abort'.
    /// # Errors
    /// Returns an [`InstrumentError`] if any errors occurred.
    fn abort(&mut self) -> Result<()> {
        self.write_all(b"abort\n")?;
        self.flush()?;
        //Do we need to wait?
        Ok(())
    }
}
