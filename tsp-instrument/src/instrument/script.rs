//! A trait that allows for the writing of a TSP script file to the instrument.

use std::io::{BufRead, Write};

use bytes::Buf;

use crate::error::Result;

/// The [`Instrument`] can write a script to be executed.
pub trait Script
where
    Self: Write,
{
    /// Write the given script to the instrument with the given name.
    ///
    /// # Parameters
    /// - `name` - is the environment-compatible name of the script
    /// - `script` - is the contents of the script
    /// - `save_script` - `true` if the script should be saved to non-volatile memory
    /// - `run_script` - `true` if the script should be run after load
    ///
    /// # Notes
    /// - The script name will not be validated to ensure that it is compatible with the
    ///     scripting environment.
    /// - The given script content will only be validated by the instrument, but not
    ///     the [`write_script`] function.
    ///
    /// # Errors
    /// Returns an [`InstrumentError`] if any errors occurred.
    fn write_script(
        &mut self,
        name: &[u8],
        script: &[u8],
        save_script: bool,
        run_script: bool,
    ) -> Result<()> {
        let name = String::from_utf8_lossy(name).to_string();
        let script = script.reader(); //String::from_utf8_lossy(script.as_ref()).to_string();
        self.write_all(b"_orig_prompts = localnode.prompts localnode.prompts = 0\n")?;
        self.flush()?;
        self.write_all(format!("{name}=nil\n").as_bytes())?;
        self.flush()?;
        self.write_all(format!("loadscript {name}\n").as_bytes())?;

        for line in script.lines() {
            self.write_all(format!("{}\n", line?).as_bytes())?;
        }

        self.write_all(b"\nendscript\n")?;
        self.flush()?;

        if save_script {
            self.write_all(format!("{name}.save()\n").as_bytes())?;
            self.flush()?;
        }

        if run_script {
            self.write_all(format!("{name}.run()\n").as_bytes())?;
            self.flush()?;
        }

        self.write_all(b"localnode.prompts = _orig_prompts _orig_prompts = nil\n")?;
        self.flush()?;

        Ok(())
    }
}
