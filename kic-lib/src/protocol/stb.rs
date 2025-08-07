use crate::InstrumentError;

use crate::error::Result;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Stb {
    Stb(u16),
    NotSupported,
}

impl Stb {
    pub(crate) const fn is_bit_set(stb: u16, bit: u8) -> bool {
        if bit > 15 {
            return false;
        }

        ((stb >> bit) & 0x0001) == 1
    }

    /// Check to see if the MSB bit is set.
    ///
    /// Not used on Trebuchet
    ///
    /// # Errors
    /// An error is returned if `read_stb` is not supported.
    pub fn measurement_summary(&self) -> Result<bool> {
        match self {
            Self::Stb(s) => Ok(Self::is_bit_set(*s, 0)),
            Self::NotSupported => Err(InstrumentError::Other(
                "read_stb() not supported".to_string(),
            )),
        }
    }

    /// Check to see if the SSB bit is set
    ///
    /// # Errors
    /// An error is returned if `read_stb` is not supported.
    pub fn system_summary(&self) -> Result<bool> {
        match self {
            Self::Stb(s) => Ok(Self::is_bit_set(*s, 1)),
            Self::NotSupported => Err(InstrumentError::Other(
                "read_stb() not supported".to_string(),
            )),
        }
    }

    /// Check to see if the EAV bit is set
    ///
    /// # Errors
    /// An error is returned if `read_stb` is not supported.
    pub fn error_available(&self) -> Result<bool> {
        match self {
            Self::Stb(s) => Ok(Self::is_bit_set(*s, 2)),
            Self::NotSupported => Err(InstrumentError::Other(
                "read_stb() not supported".to_string(),
            )),
        }
    }

    /// Check to see if the QSB bit is set.
    ///
    /// Not used on Trebuchet
    ///
    /// # Errors
    /// An error is returned if `read_stb` is not supported.
    pub fn questionable_summary(&self) -> Result<bool> {
        match self {
            Self::Stb(s) => Ok(Self::is_bit_set(*s, 3)),
            Self::NotSupported => Err(InstrumentError::Other(
                "read_stb() not supported".to_string(),
            )),
        }
    }

    /// Check to see if the MAV bit is set
    ///
    /// # Errors
    /// An error is returned if `read_stb` is not supported.
    pub fn message_available(&self) -> Result<bool> {
        match self {
            Self::Stb(s) => Ok(Self::is_bit_set(*s, 4)),
            Self::NotSupported => Err(InstrumentError::Other(
                "read_stb() not supported".to_string(),
            )),
        }
    }

    /// Check to see if the ESR bit is set
    ///
    /// # Errors
    /// An error is returned if `read_stb` is not supported.
    pub fn event_summary(&self) -> Result<bool> {
        match self {
            Self::Stb(s) => Ok(Self::is_bit_set(*s, 5)),
            Self::NotSupported => Err(InstrumentError::Other(
                "read_stb() not supported".to_string(),
            )),
        }
    }

    /// Check to see if the SRQ bit is set
    ///
    /// # Errors
    /// An error is returned if `read_stb` is not supported.
    pub fn srq(&self) -> Result<bool> {
        match self {
            Self::Stb(s) => Ok(Self::is_bit_set(*s, 6)),
            Self::NotSupported => Err(InstrumentError::Other(
                "read_stb() not supported".to_string(),
            )),
        }
    }
}
