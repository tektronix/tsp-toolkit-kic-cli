use thiserror::Error;

/// Define errors that originate from this crate
#[derive(Error, Debug)]
#[allow(clippy::module_name_repetitions)]
pub enum KicError {
    /// The user didn't provide required information or the information provided was
    /// invalid
    #[error("Error parsing arguments: {details}")]
    ArgParseError {
        /// The reason why the arguments failed to parse.
        details: String,
    },

    /// Another user must relinquish the instrument before it can be logged into.
    #[error("there is another session connected to the instrument that must logout")]
    InstrumentLogoutRequired,

    /// The instrument is protected over the given interface. This should ONLY be used
    /// for checking the login status of an instrument.
    #[error("the instrument is password protected")]
    InstrumentPasswordProtected,

    /// The user tried to connect an instrument with a VISA resource string, but no
    /// VISA driver was detected.
    #[error("no VISA driver detected but a connection to a VISA device was requested")]
    NoVisa,

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// The requested action was not supported.
    #[error("the requested action is not supported: {0}")]
    UnsupportedAction(String),

    #[error("instrument error: {0}")]
    InstrumentError(#[from] kic_lib::InstrumentError),
}
