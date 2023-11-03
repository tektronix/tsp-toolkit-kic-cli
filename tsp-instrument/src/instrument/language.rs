//! All of the enums, traits, and impls for language management on an instrument.

use std::{fmt::Display, str::FromStr};

use crate::InstrumentError;

/// The languages that could be on an instrument.
#[allow(clippy::module_name_repetitions)]
#[derive(Debug, PartialEq, Eq)]
pub enum CmdLanguage {
    /// The SCPI language
    Scpi,
    /// The TSP language
    Tsp,
}

impl FromStr for CmdLanguage {
    type Err = InstrumentError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim_matches(|c| c == char::from(0)).trim();
        match s {
            "SCPI" => Ok(Self::Scpi),
            "TSP" => Ok(Self::Tsp),
            _ => Err(InstrumentError::UnknownLanguage {
                lang: s.trim().to_string(),
            }),
        }
    }
}

impl Display for CmdLanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Scpi => write!(f, "SCPI"),
            Self::Tsp => write!(f, "TSP"),
        }
    }
}

/// The functions to interface with an instrument and check or set the language.
///
/// # Default
/// The default implementation of this trait assumes that the instrument only has the
/// TSP language, which is true for all instruments except TTI, as of the writing of
/// this comment)
pub trait Language {
    /// Get the current language on the instrument.
    ///
    /// # Errors
    /// [`InstrumentError`] is returned in the case of IO error, Unknown Language or other errors
    fn get_language(&mut self) -> Result<CmdLanguage, InstrumentError> {
        Ok(CmdLanguage::Tsp)
    }

    /// Set the language on the instrument to the given language.
    ///
    /// # Errors
    /// [`InstrumentError`] is returned in the case of IO error, Unknown Language or other errors
    fn change_language(&mut self, _lang: CmdLanguage) -> Result<(), InstrumentError> {
        Ok(())
    }
}
