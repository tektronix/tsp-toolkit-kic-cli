use crate::error::Result;

/// The trait an instrument must implement in order to flash the firmware onto an
/// instrument.
pub trait Flash {
    /// The method to flash a firmware image to an instrument.
    ///
    /// # Errors
    /// An error can occur in the write to or reading from the instrument as well as in
    /// reading the firmware image.
    fn flash_firmware(&mut self, image: &[u8], firmware_info: Option<u16>) -> Result<()>;
}
