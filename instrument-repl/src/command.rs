use std::path::PathBuf;

use crate::TspError;

/// A request from a user that is to be dispatched within the program.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Request {
    /// A TSP command that should be sent to the instrument
    Tsp(String),
    /// A request for the errors from the instrument.
    GetError,
    Script {
        file: PathBuf,
    },
    TspLinkNodes {
        json_file: PathBuf,
    },
    Info {
        slot: Option<usize>,
    },
    Update {
        file: PathBuf,
        slot: Option<u16>,
    },
    Exit,
    Help {
        sub_cmd: Option<String>,
    },
    Usage(String),
    None,
}

/// Responses from the program or instrument that a [`Request`] was sent to.
pub enum Response {
    /// A response to be displayed to the user as text
    TextData(String),
    /// A response to be displayed to the user as binary data
    BinaryData(Vec<u8>),
    /// A response to be displayed to the user as a TSP error
    TspError(TspError),
    /// A response from an internal API that should be handled internally
    InternalApi(String),
}

/// A notification from the program or instrument that was otherwise unsolicited
pub enum Notification {
    /// A notification from an internal API. This data should probably be processed
    /// instead of being directly displayed to the user.
    InternalApi(String),
}
