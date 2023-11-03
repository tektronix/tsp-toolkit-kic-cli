use thiserror::Error;
#[derive(Error, Debug)]
#[allow(clippy::module_name_repetitions)]
pub enum DebugError {
    /// Unable to deserialize from a json string to an object using serde
    #[error("Deserialization error occurred: {source}")]
    DeserializationError {
        #[from]
        /// The original `serde_json` error
        source: serde_json::Error,
    },

    /// An error coming from `tsp_instrument`
    #[error("Instrument Error ocurred: {source}")]
    InstrumentError {
        #[from]
        /// The original `tsp_instrument` error
        source: tsp_instrument::InstrumentError,
    },

    /// An IO error occurred
    #[error("IO error occurred: {source}")]
    IOError {
        /// The original `[std::io::Error]`
        #[from]
        source: std::io::Error,
    },

    /// The interactive command from the user was not correct.
    #[error("command error: {details}")]
    CommandError {
        /// The details of why the command error occurred.
        details: String,
    },

    /// An error occurred when Clap tried to parse a command
    #[error("command parsing error: {source}")]
    ClapError {
        /// The original error
        #[from]
        source: clap::error::Error,
    },

    /// Some other error
    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, DebugError>;
