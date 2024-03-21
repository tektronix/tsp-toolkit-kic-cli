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
}
