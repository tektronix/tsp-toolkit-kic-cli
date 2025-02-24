use std::fmt::Display;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedResponse {
    Prompt,
    PromptWithError,
    TspErrorStart,
    TspError(String),
    TspErrorEnd,
    Data(Vec<u8>),
    ProgressIndicator,
    NodeStart,
    NodeEnd,
}

impl Display for ParsedResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Prompt => "prompt".to_string(),
            Self::PromptWithError => "prompt with error".to_string(),
            Self::TspErrorStart => "start of error dump".to_string(),
            Self::TspError(e) => format!("error item: \"{e}\""),
            Self::TspErrorEnd => "end of error dump".to_string(),
            Self::Data(d) => format!("textual data: \"{d:?}\""),
            Self::ProgressIndicator => "progress indicator".to_string(),
            Self::NodeStart => "node data start".to_string(),
            Self::NodeEnd => "node data end".to_string(),
        };
        write!(f, "{s}")
    }
}

fn find_first_of(input: &[u8], search: &[Vec<u8>]) -> Option<usize> {
    let mut lowest_pos = input.len();
    for i in search {
        let temp = input
            .windows(i.len())
            .position(|w| w == i)
            .map_or(lowest_pos, |x| x);
        if temp < lowest_pos {
            lowest_pos = temp;
        }
    }
    if lowest_pos < input.len() {
        Some(lowest_pos)
    } else {
        None
    }
}

impl ParsedResponse {
    #[must_use]
    pub fn find_next(input: &[u8]) -> Option<usize> {
        find_first_of(
            input,
            &[
                b"TSP>".to_vec(),
                b"TSP?".to_vec(),
                b"ERM>START".to_vec(),
                b"ERM>DONE".to_vec(),
                b"ERM>".to_vec(),
                b">>>>".to_vec(),
                b"NODE>START".to_vec(),
                b"NODE>END".to_vec(),
            ],
        )
    }

    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn parse_next(input: &[u8]) -> Option<(Self, Vec<u8>)> {
        if input.is_empty() || input[0] == 0u8 {
            return None;
        }
        let s = String::from_utf8_lossy(input).trim_start().to_string();

        if s.starts_with("NODE>START") {
            let v = if input.len() > 10 {
                input[10..].to_vec()
            } else {
                Vec::new()
            };
            return Some((Self::NodeStart, v));
        }
        if s.starts_with("NODE>END") {
            let v = if input.len() > 8 {
                input[8..].to_vec()
            } else {
                Vec::new()
            };
            return Some((Self::NodeEnd, v));
        }
        if s.starts_with("TSP>") {
            let v = if input.len() > 4 {
                input[4..].to_vec()
            } else {
                Vec::new()
            };
            return Some((Self::Prompt, v));
        }
        if s.starts_with("TSP?") {
            let v = if input.len() > 4 {
                input[4..].to_vec()
            } else {
                Vec::new()
            };
            return Some((Self::PromptWithError, v));
        }
        if s.starts_with("ERM>START") {
            let v = if input.len() > 9 {
                input[9..].to_vec()
            } else {
                Vec::new()
            };
            return Some((Self::TspErrorStart, v));
        }
        if s.starts_with("ERM>DONE") {
            let v = if input.len() > 8 {
                input[8..].to_vec()
            } else {
                Vec::new()
            };
            return Some((Self::TspErrorEnd, v));
        }
        if s.starts_with("ERM>") {
            let (v, r): (Vec<u8>, Vec<u8>) = if input.len() > 4 {
                Self::find_next(&input[4..]).map_or_else(
                    || (input[4..].to_vec(), Vec::new()),
                    |next_token| {
                        (
                            #[allow(clippy::arithmetic_side_effects)]
                            input[4..(next_token + 4)].to_vec(),
                            #[allow(clippy::arithmetic_side_effects)]
                            input[(next_token + 4)..].to_vec(),
                        )
                    },
                )
            } else {
                (Vec::new(), Vec::new())
            };
            let msg = String::from_utf8_lossy(&v).to_string();
            let msg = msg.trim().to_string();
            return Some((Self::TspError(msg), r));
        }
        if s.starts_with(">>>>") {
            let v = if input.len() > 4 {
                input[4..].to_vec()
            } else {
                Vec::new()
            };
            return Some((Self::ProgressIndicator, v));
        }
        let (msg, r): (Vec<u8>, Vec<u8>) = Self::find_next(input).map_or_else(
            || (input.to_owned(), Vec::new()),
            |next_token| (input[..next_token].to_vec(), input[next_token..].to_vec()),
        );
        Some((Self::Data(msg), r))
    }
}

pub(crate) struct ResponseParser {
    data: Vec<u8>,
}

impl ResponseParser {
    pub fn new<T: AsRef<[u8]>>(data: T) -> Self {
        let data = Vec::from(data.as_ref());
        Self { data }
    }
}

impl From<Vec<u8>> for ResponseParser {
    fn from(data: Vec<u8>) -> Self {
        Self { data }
    }
}

impl Iterator for ResponseParser {
    type Item = ParsedResponse;

    fn next(&mut self) -> Option<Self::Item> {
        let (ret, remainder) = ParsedResponse::parse_next(&self.data)?;

        let remainder = remainder.trim_ascii_start().to_vec();

        self.data = remainder;

        Some(ret)
    }
}

#[cfg(test)]
mod unit {
    use crate::instrument::ParsedResponse;

    use super::ResponseParser;

    #[test]
    fn instrument_response_parser_prompt_remainder() {
        let test = b"TSP>TSP?";
        let mut parser = ResponseParser::new(test);

        assert_eq!(parser.next(), Some(ParsedResponse::Prompt));
        assert_eq!(parser.next(), Some(ParsedResponse::PromptWithError));
        assert_eq!(parser.next(), None);
    }

    #[test]
    fn instrument_response_parser_prompt_whitespace() {
        let test = b"TSP>\nTSP?";
        let mut parser = ResponseParser::new(test);

        assert_eq!(parser.next(), Some(ParsedResponse::Prompt));
        assert_eq!(parser.next(), Some(ParsedResponse::PromptWithError));
        assert_eq!(parser.next(), None);
    }

    #[test]
    fn instrument_response_parser_errors() {
        let test = b"ERM>START\nERM>An Error Message\nERM>DONE";
        let mut parser = ResponseParser::new(test);

        assert_eq!(parser.next(), Some(ParsedResponse::TspErrorStart));
        assert_eq!(
            parser.next(),
            Some(ParsedResponse::TspError("An Error Message".to_string()))
        );
        assert_eq!(parser.next(), Some(ParsedResponse::TspErrorEnd));
        assert_eq!(parser.next(), None);
    }

    #[test]
    fn instrument_response_parser_progress_indicator() {
        let test = b">>>>\n>>>>\nTSP>>>>>\n>>>>";
        let mut parser = ResponseParser::new(test);

        assert_eq!(parser.next(), Some(ParsedResponse::ProgressIndicator));
        assert_eq!(parser.next(), Some(ParsedResponse::ProgressIndicator));
        assert_eq!(parser.next(), Some(ParsedResponse::Prompt));
        assert_eq!(parser.next(), Some(ParsedResponse::ProgressIndicator));
        assert_eq!(parser.next(), Some(ParsedResponse::ProgressIndicator));
        assert_eq!(parser.next(), None);
    }

    #[test]
    fn instrument_response_parser_data() {
        let test = b"TSP>\nSome data from the instrument\nMaybe across multiple lines\nTSP?";
        let mut parser = ResponseParser::new(test);

        assert_eq!(parser.next(), Some(ParsedResponse::Prompt));
        assert_eq!(
            parser.next(),
            Some(ParsedResponse::Data(
                b"Some data from the instrument\nMaybe across multiple lines\n".to_vec()
            ))
        );
        assert_eq!(parser.next(), Some(ParsedResponse::PromptWithError));
        assert_eq!(parser.next(), None);
    }

    #[test]
    fn instrument_response_parser_tuple_types() {
        let test = b"TSP>\nSome data from the instrument\nMaybe across multiple lines \nTSP?ERM> Some Error Message!!!! #0`~!@#$%^&*()-_=+[]}\\|;:'\",<.>/?";
        let mut parser = ResponseParser::new(test);

        assert_eq!(parser.next(), Some(ParsedResponse::Prompt));
        assert_eq!(
            parser.next(),
            Some(ParsedResponse::Data(
                b"Some data from the instrument\nMaybe across multiple lines \n".to_vec()
            ))
        );
        assert_eq!(parser.next(), Some(ParsedResponse::PromptWithError));
        assert_eq!(
            parser.next(),
            Some(ParsedResponse::TspError(
                "Some Error Message!!!! #0`~!@#$%^&*()-_=+[]}\\|;:'\",<.>/?".to_string()
            ))
        );
        assert_eq!(parser.next(), None);
    }
}
