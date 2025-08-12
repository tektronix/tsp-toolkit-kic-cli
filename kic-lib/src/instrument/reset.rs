//! A trait that allows for resetting the instrument.

use std::io::Write;

use crate::error::Result;

/// The [`Instrument`] can be reset.
pub trait Reset
where
    Self: Write,
{
    /// Reset the instrument.
    ///
    /// # Notes
    /// - Reset the instrument using *RST.
    /// # Errors
    /// Returns an [`InstrumentError`] if any errors occurred.
    fn reset(&mut self) -> Result<()> {
        self.write_all(b"*RST\n")?;
        self.flush()?;
        //Do we need to wait?
        Ok(())
    }
}
