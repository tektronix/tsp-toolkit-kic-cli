use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Breakpoint {
    #[serde(rename = "LineNumber")]
    pub line_number: u32,
    #[serde(rename = "Enable")]
    pub enable: bool,
    #[serde(rename = "Condition")]
    pub condition: String,
}
