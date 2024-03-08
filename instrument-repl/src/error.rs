//! All the errors that this crate can emit are defined in the
//! [`error::InstrumentError`] enum.

use std::sync::mpsc::SendError;

use thiserror::Error;

use crate::{command::Request, instrument::ParsedResponse, state_machine::ReadState};

/// Define errors that originate from this crate
#[derive(Error, Debug)]
#[allow(clippy::module_name_repetitions)]
pub enum InstrumentReplError {
    /// An Error occurred in the tsp-instrument crate.
    #[error("instrument error occurred: {source}")]
    InstrumentError {
        ///The original [`tsp_toolkit_kic_lib::InstrumentError`]
        #[from]
        source: tsp_toolkit_kic_lib::InstrumentError,
    },

    /// An IO error occurred
    #[error("IO error occurred: {source}")]
    IOError {
        /// The original `[std::io::Error]`
        #[from]
        source: std::io::Error,
    },

    /// An error occurred while attempting to parse the data from the instrument.
    #[error("error parsing data from instrument: {data:?}")]
    DataParseError {
        /// The data that couldn't be parsed
        data: Vec<u8>,
    },

    /// An error occurred during a state-machine transition
    #[error("state machine transition error: in \"{state}\" state, encountered unexpected input \"{input}\"")]
    StateMachineTransitionError {
        /// The [`ReadState`] we were in.
        state: ReadState,
        /// The input that was causing the transition.
        input: ParsedResponse,
    },

    /// An uncategorized error.
    #[error("{0}")]
    Other(String),

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

    /// There was an issue sending data between threads of the application
    #[error("internal communication problem: {source}")]
    InternalCommError {
        /// The original error
        #[from]
        source: SendError<Request>,
    },

    /// There was an error deserializing a JSON message
    #[error("deserialization error: {source}")]
    DeserializationError {
        ///The original error
        #[from]
        source: serde_json::Error,
    },
}

pub(crate) type Result<T> = std::result::Result<T, InstrumentReplError>;
