use crate::models::send::{SendFile, SendText, SendType};
use serde::{Deserialize, Serialize};

/// Response from public Send access
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendAccess {
    /// Send ID
    pub id: String,

    /// Send type
    #[serde(rename = "type")]
    pub send_type: SendType,

    /// Encrypted name
    pub name: String,

    /// Text data (if type=0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<SendText>,

    /// File data (if type=1)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<SendFile>,

    /// Encrypted key
    pub key: String,

    /// Access count
    pub access_count: u32,

    /// Whether password required
    pub password_required: bool,
}
