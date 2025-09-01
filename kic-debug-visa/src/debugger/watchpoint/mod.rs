///The Watchpoint struct to hold the deserialized
/// json data when .debug setWatchpoint is invoked
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct WatchpointInfo {
    #[serde(rename = "Enable")]
    pub enable: bool,
    #[serde(rename = "Expression")]
    pub expression: String,
}
