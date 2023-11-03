use std::fmt::Display;

use crate::{error::Result, instrument::ParsedResponse, InstrumentReplError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReadState {
    #[default]
    Init,
    TextDataReadStart,
    TextDataReadContinue,
    BinaryDataReadStart,
    BinaryDataReadContinue,
    DataReadEnd,
    DataReadEndPendingError,
    ErrorReadStart,
    ErrorReadContinue,
    ErrorReadEnd,
    FileLoading,
}

impl ReadState {
    pub const fn new() -> Self {
        Self::Init
    }
}

impl Display for ReadState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Init => "init",
                Self::TextDataReadStart => "start of textual read",
                Self::TextDataReadContinue => "continuing textual read",
                Self::BinaryDataReadStart => "start of raw binary read",
                Self::BinaryDataReadContinue => "continuing raw binary read",
                Self::DataReadEnd => "end of instrument output read",
                Self::DataReadEndPendingError => "end of instrument output with pending errors",
                Self::ErrorReadStart => "start of error dump",
                Self::ErrorReadContinue => "continuing error dump read",
                Self::ErrorReadEnd => "end of error dump read",
                Self::FileLoading => "loading file",
            }
        )
    }
}

impl ReadState {
    pub fn next_state(self, input: &ParsedResponse) -> Result<Self> {
        type IR = ParsedResponse;
        #[allow(clippy::match_same_arms, clippy::unnested_or_patterns)]
        match (&self, input) {
            // Transitions from Init
            (Self::Init, IR::Prompt) => Ok(Self::DataReadEnd),
            (Self::Init, IR::PromptWithError) => Ok(Self::DataReadEndPendingError),
            (Self::Init, IR::TspErrorStart) => Ok(Self::ErrorReadStart),
            (Self::Init, IR::Data(_)) => Ok(Self::TextDataReadStart),
            (Self::Init, IR::BinaryData(_)) => Ok(Self::BinaryDataReadStart),
            (Self::Init, IR::ProgressIndicator) => Ok(Self::FileLoading),

            // Transitions from TextDataReadStart
            (Self::TextDataReadStart, IR::Prompt) => Ok(Self::DataReadEnd),
            (Self::TextDataReadStart, IR::PromptWithError) => Ok(Self::DataReadEndPendingError),
            (Self::TextDataReadStart, IR::TspErrorStart) => Ok(Self::ErrorReadStart),
            (Self::TextDataReadStart, IR::Data(_)) => Ok(Self::TextDataReadContinue),
            (Self::TextDataReadStart, IR::ProgressIndicator) => Ok(Self::FileLoading),

            // Transitions from TextDataReadContinue
            (Self::TextDataReadContinue, IR::Prompt) => Ok(Self::DataReadEnd),
            (Self::TextDataReadContinue, IR::PromptWithError) => Ok(Self::DataReadEndPendingError),
            (Self::TextDataReadContinue, IR::Data(_)) => Ok(self),
            (Self::TextDataReadContinue, IR::ProgressIndicator) => Ok(Self::FileLoading),

            // Transition from BinaryDataReadStart
            (Self::BinaryDataReadStart, IR::Prompt) => Ok(Self::DataReadEnd),
            (Self::BinaryDataReadStart, IR::PromptWithError) => Ok(Self::DataReadEndPendingError),
            (Self::BinaryDataReadStart, IR::TspErrorStart) => Ok(self),
            (Self::BinaryDataReadStart, IR::BinaryData(_)) => Ok(Self::BinaryDataReadContinue),
            (Self::BinaryDataReadStart, IR::Data(_)) => Ok(Self::BinaryDataReadContinue),
            (Self::BinaryDataReadStart, IR::ProgressIndicator) => Ok(Self::FileLoading),

            // Transition from BinaryDataReadContinue
            (Self::BinaryDataReadContinue, IR::Prompt) => Ok(Self::DataReadEnd),
            (Self::BinaryDataReadContinue, IR::PromptWithError) => Ok(Self::DataReadEndPendingError),
            (Self::BinaryDataReadContinue, IR::TspErrorStart) => Ok(Self::ErrorReadStart),
            (Self::BinaryDataReadContinue, IR::BinaryData(_)) => Ok(self),
            (Self::BinaryDataReadContinue, IR::Data(_)) => Ok(self),
            (Self::BinaryDataReadContinue, IR::ProgressIndicator) => Ok(Self::FileLoading),

            // Transitions from DataReadEnd
            (Self::DataReadEnd, IR::Prompt) => Ok(self),
            (Self::DataReadEnd, IR::PromptWithError) => Ok(Self::DataReadEndPendingError),
            (Self::DataReadEnd, IR::TspErrorStart) => Ok(Self::ErrorReadStart),
            (Self::DataReadEnd, IR::Data(_)) => Ok(Self::TextDataReadStart),
            (Self::DataReadEnd, IR::BinaryData(_)) => Ok(Self::BinaryDataReadStart),
            (Self::DataReadEnd, IR::ProgressIndicator) => Ok(Self::FileLoading),

            // Transitions from DataReadEndPendingError
            (Self::DataReadEndPendingError, IR::Prompt) => Ok(Self::DataReadEnd),
            (Self::DataReadEndPendingError, IR::PromptWithError) => Ok(Self::DataReadEndPendingError),
            (Self::DataReadEndPendingError, IR::TspErrorStart) => Ok(Self::ErrorReadStart),
            (Self::DataReadEndPendingError, IR::Data(_)) => Ok(Self::TextDataReadStart),
            (Self::DataReadEndPendingError, IR::BinaryData(_)) => Ok(Self::BinaryDataReadStart),
            (Self::DataReadEndPendingError, IR::ProgressIndicator) => Ok(Self::FileLoading),

            // Transitions from ErrorReadStart
            (Self::ErrorReadStart, IR::TspError(_)) => Ok(Self::ErrorReadContinue),
            (Self::ErrorReadStart, IR::TspErrorEnd) => Ok(Self::ErrorReadEnd),
            (Self::ErrorReadStart, IR::ProgressIndicator) => Ok(Self::FileLoading),

            // Transitions from ErrorReadContinue
            (Self::ErrorReadContinue, IR::TspError(_)) => Ok(self),
            (Self::ErrorReadContinue, IR::TspErrorEnd) => Ok(Self::ErrorReadEnd),
            (Self::ErrorReadContinue, IR::ProgressIndicator) => Ok(Self::FileLoading),

            // Transitions from ErrorReadEnd
            (Self::ErrorReadEnd, IR::Prompt) => Ok(Self::DataReadEnd),
            (Self::ErrorReadEnd, IR::PromptWithError) => Ok(Self::DataReadEndPendingError),
            (Self::ErrorReadEnd, IR::TspErrorStart) => Ok(Self::ErrorReadStart),
            (Self::ErrorReadEnd, IR::Data(_)) => Ok(Self::TextDataReadStart),
            (Self::ErrorReadEnd, IR::BinaryData(_)) => Ok(Self::BinaryDataReadStart),
            (Self::ErrorReadEnd, IR::ProgressIndicator) => Ok(Self::FileLoading),

            // inputs that never cause a transition (input ignored in state machine)
            // TODO This might be better served as a transition to a "FileLoading" state
            (Self::FileLoading, IR::Prompt) => Ok(Self::Init),
            (Self::FileLoading, IR::PromptWithError) => Ok(Self::DataReadEndPendingError),
            (Self::FileLoading, IR::TspErrorStart) => Ok(Self::ErrorReadStart),
            (Self::FileLoading, IR::Data(_)) => Ok(Self::TextDataReadStart),
            (Self::FileLoading, IR::BinaryData(_)) => Ok(Self::BinaryDataReadStart),
            (Self::FileLoading, IR::ProgressIndicator) => Ok(self),

            // Erroneous transitions that require recovery
            // Listed explicitly to make sure we don't miss anything
            // Starting with Init
            (Self::Init, IR::TspError(_))
            | (Self::Init, IR::TspErrorEnd)
            // TextDataReadStart
            | (Self::TextDataReadStart, IR::TspError(_))
            | (Self::TextDataReadStart, IR::TspErrorEnd)
            | (Self::TextDataReadStart, IR::BinaryData(_))
            // TextDataReadContinue
            | (Self::TextDataReadContinue, IR::TspErrorStart)
            | (Self::TextDataReadContinue, IR::TspError(_))
            | (Self::TextDataReadContinue, IR::TspErrorEnd)
            | (Self::TextDataReadContinue, IR::BinaryData(_))
            // BinaryDataReadStart
            | (Self::BinaryDataReadStart, IR::TspError(_))
            | (Self::BinaryDataReadStart, IR::TspErrorEnd)
            // BinaryDataReadContinue
            | (Self::BinaryDataReadContinue, IR::TspError(_))
            | (Self::BinaryDataReadContinue, IR::TspErrorEnd)
            // DataReadEnd
            | (Self::DataReadEnd, IR::TspError(_))
            | (Self::DataReadEnd, IR::TspErrorEnd)
            // DataReadEndPendingError
            | (Self::DataReadEndPendingError, IR::TspError(_))
            | (Self::DataReadEndPendingError, IR::TspErrorEnd)
            // ErrorReadStart
            | (Self::ErrorReadStart, IR::Prompt)
            | (Self::ErrorReadStart, IR::PromptWithError)
            | (Self::ErrorReadStart, IR::TspErrorStart)
            | (Self::ErrorReadStart, IR::Data(_))
            | (Self::ErrorReadStart, IR::BinaryData(_))
            // ErrorReadContinue
            | (Self::ErrorReadContinue, IR::Prompt)
            | (Self::ErrorReadContinue, IR::PromptWithError)
            | (Self::ErrorReadContinue, IR::TspErrorStart)
            | (Self::ErrorReadContinue, IR::Data(_))
            | (Self::ErrorReadContinue, IR::BinaryData(_))
            // FileLoading
            | (Self::FileLoading, IR::TspError(_))
            | (Self::FileLoading, IR::TspErrorEnd)
            // ErrorReadEnd
            | (Self::ErrorReadEnd, IR::TspError(_))
            | (Self::ErrorReadEnd, IR::TspErrorEnd) => {
                Err(InstrumentReplError::StateMachineTransitionError { state: self, input: input.clone()})
            }

        }
    }
}

#[cfg(test)]
mod unit {
    use crate::instrument::ParsedResponse;

    use super::ReadState;

    #[test]
    fn normal_happy_path_transitions() {
        let mut actual: Vec<ReadState> = Vec::new();
        let mut current = ReadState::default();
        let inputs = vec![
            ParsedResponse::Prompt,                                   //Init
            ParsedResponse::Data(Vec::new()),                         //TextDataReadStart
            ParsedResponse::Data(Vec::new()),                         //TextDataReadContinue
            ParsedResponse::PromptWithError,                          //DataReadEndPendingError
            ParsedResponse::TspErrorStart,                            //ErrorReadStart
            ParsedResponse::TspError("Some error".to_string()),       //ErrorReadContinue
            ParsedResponse::TspError("Some other error".to_string()), //ErrorReadContinue
            ParsedResponse::TspErrorEnd,                              //ErrorReadEnd
            ParsedResponse::Prompt,                                   //Init
            ParsedResponse::ProgressIndicator,                        //FileLoading
            ParsedResponse::ProgressIndicator,                        //FileLoading
            ParsedResponse::ProgressIndicator,                        //FileLoading
            ParsedResponse::ProgressIndicator,                        //FileLoading
            ParsedResponse::ProgressIndicator,                        //FileLoading
            ParsedResponse::Prompt,                                   //DataReadEnd
        ];

        let expected = vec![
            ReadState::Init,
            ReadState::DataReadEnd,
            ReadState::TextDataReadStart,
            ReadState::TextDataReadContinue,
            ReadState::DataReadEndPendingError,
            ReadState::ErrorReadStart,
            ReadState::ErrorReadContinue,
            ReadState::ErrorReadContinue,
            ReadState::ErrorReadEnd,
            ReadState::DataReadEnd,
            ReadState::FileLoading,
            ReadState::FileLoading,
            ReadState::FileLoading,
            ReadState::FileLoading,
            ReadState::FileLoading,
            ReadState::Init,
        ];

        actual.push(current);
        for i in inputs {
            current = current.next_state(&i).expect("should get next state");
            actual.push(current);
        }

        assert_eq!(actual, expected);
    }

    #[test]
    fn normal_happy_path_transitions_no_errors() {
        let mut actual: Vec<ReadState> = Vec::new();
        let mut current = ReadState::default();
        let inputs = vec![
            ParsedResponse::Prompt,            //Init
            ParsedResponse::Data(Vec::new()),  //TextDataReadStart
            ParsedResponse::Data(Vec::new()),  //TextDataReadContinue
            ParsedResponse::PromptWithError,   //DataReadEndPendingError
            ParsedResponse::TspErrorStart,     //ErrorReadStart
            ParsedResponse::TspErrorEnd,       //ErrorReadEnd
            ParsedResponse::Prompt,            //Init
            ParsedResponse::ProgressIndicator, //FileLoading
            ParsedResponse::ProgressIndicator, //FileLoading
            ParsedResponse::ProgressIndicator, //FileLoading
            ParsedResponse::ProgressIndicator, //FileLoading
            ParsedResponse::ProgressIndicator, //FileLoading
            ParsedResponse::Prompt,            //DataReadEnd
        ];

        let expected = vec![
            ReadState::Init,
            ReadState::DataReadEnd,
            ReadState::TextDataReadStart,
            ReadState::TextDataReadContinue,
            ReadState::DataReadEndPendingError,
            ReadState::ErrorReadStart,
            ReadState::ErrorReadEnd,
            ReadState::DataReadEnd,
            ReadState::FileLoading,
            ReadState::FileLoading,
            ReadState::FileLoading,
            ReadState::FileLoading,
            ReadState::FileLoading,
            ReadState::Init,
        ];

        actual.push(current);
        for i in inputs {
            current = current.next_state(&i).expect("should get next state");
            actual.push(current);
        }

        assert_eq!(actual, expected);
    }
}
