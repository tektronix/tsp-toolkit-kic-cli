//! All the errors that this crate can emit are defined in the
//! [`error::InstrumentError`] enum.

use std::{net::AddrParseError, num::ParseIntError, string::FromUtf8Error};

use thiserror::Error;

/// Define errors that originate from this crate
#[derive(Error, Debug)]
#[allow(clippy::module_name_repetitions)]
pub enum InstrumentError {
    /// The `unparsable_string` was passed where an address was expected, but
    /// it couldn't be parsed to a valid address.
    #[error("address parsing error: {0}")]
    AddressParsingError(String),

    /// [`reqwest`] encountered an error when trying to fetch web data.
    #[error("web retrieval error: {0}")]
    WebRetrievalError(#[from] reqwest::Error),

    /// [`roxmltree`] encountered and issue in parsing XML.
    #[error("error parsing XML: {0}")]
    XmlParseError(#[from] roxmltree::Error),

    /// There was an error while trying to connect to the interface or instrument.
    #[error("connection error occurred: {details}")]
    ConnectionError {
        // The details of the [`ConnectionError`]
        details: String,
    },

    /// There was an issue while disconnecting from an instrument.
    #[error("unable to gracefully disconnect from instrument: {details}")]
    DisconnectError {
        /// More information about the disconnection error.
        details: String,
    },

    /// A resource file was unable to be decrypted.
    #[error("unable to decrypt resource: {source}")]
    ResourceDecryptError {
        ///**TODO:** Change this to the error that is produced when decryption fails
        #[from]
        source: FromUtf8Error,
    },

    /// An error that occurs while trying to retrieve information about an instrument
    /// such as the serial number, model, manufacturer, etc.
    #[error("instrument information retrieval error: {details}")]
    InformationRetrievalError {
        /// Any extra information about why the instrument information could not be
        /// retrieved.
        details: String,
    },

    /// An Error that originates from an instrument. This is generic for all instruments
    /// and is therefore just a [`String`].
    #[error("{error}")]
    InstrumentError {
        /// The error string provided by the instrument.
        error: String,
    },

    /// Converts a [`std::io::Error`] to a [`TeaspoonInterfaceError`]
    #[error("IO error: {source}")]
    IoError {
        /// The [`std::io::Error`] from which this [`TeaspoonInterfaceError::IoError`]
        /// was derived.
        #[from]
        source: std::io::Error,
    },

    /// The provided login details were either incorrect or the instrument is already
    /// claimed and cannot be claimed again.
    #[error("provided login details rejected or instrument already claimed.")]
    LoginRejected,

    /// The instrument is already claimed by another interface.
    #[error("Another interface has control, logout on that interface.")]
    InterfaceLoginErr,

    #[error("{source}")]
    ParseIntError {
        #[from]
        source: ParseIntError,
    },

    /// The TSP error that was received from the instrument was malformed.
    #[error("unable to parse TSP error from instrument {error}")]
    TspErrorParseError {
        /// The text of the malformed error that was provided by the instrument
        error: String,
    },

    /// There was an error getting the model from the instrument.
    #[error("model parse error: {0}")]
    GetModelError(String),

    /// The VISA feature was not enabled and therefore no visa methods can be called
    #[error("the requested feature requires a VISA module to be installed")]
    NoVisa,

    /// There was an error parsing an IP address
    #[error("address parsing error: {0}")]
    AddrParseError(#[from] AddrParseError),

    /// The queried instrument returned an unknown model number
    #[error("\"{model}\" is not a recognized model number")]
    UnknownInstrumentModel {
        /// The unknown model number
        model: String,
    },

    /// The queried instrument returned an unknown language type
    #[error("\"{lang} is not a recognized embedded instrument language\"")]
    UnknownLanguage {
        /// The unknown language type
        lang: String,
    },

    #[cfg(feature = "visa")]
    /// An error from the visa driver
    #[error("visa error: {source}")]
    VisaError {
        #[from]
        source: visa_rs::Error,
    },

    #[cfg(feature = "visa")]
    /// An error from the visa driver
    #[error("visa parse error: {0}")]
    VisaParseError(String),

    #[error("Instrument upgrade failed: {0}")]
    FwUpgradeFailure(String),

    #[error("unknown vendor error: {0}")]
    UnknownVendor(String),

    #[error("system keyring error: {0}")]
    KeyringError(#[from] keyring::Error),

    #[error("serialization or deserialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("authentication failure: {0}")]
    AuthenticationFailure(String),

    /// An uncategorized error.
    #[error("{0}")]
    Other(String),
}

pub(crate) type Result<T> = std::result::Result<T, InstrumentError>;
