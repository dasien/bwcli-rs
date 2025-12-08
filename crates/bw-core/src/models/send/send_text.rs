use serde::{Deserialize, Serialize};

/// Text Send data
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendText {
    /// Encrypted text content (EncString)
    pub text: String,

    /// Whether text should be hidden by default
    pub hidden: bool,
}
