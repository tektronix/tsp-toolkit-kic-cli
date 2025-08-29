///The variable struct to hold the deserialized
/// json data when .debug setVariable is invoked
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct VariableInfo {
    #[serde(rename = "StackLevel")]
    pub stack_level: u32,
    #[serde(rename = "ArgumentList")]
    pub argument_list: Vec<String>,
    #[serde(rename = "Value")]
    pub value: String,
    #[serde(rename = "Scope")]
    pub scope_type: String,
}
