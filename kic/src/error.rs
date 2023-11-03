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
}
