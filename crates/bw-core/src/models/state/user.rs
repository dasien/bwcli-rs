use serde::{Deserialize, Serialize};

/// User profile information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserProfile {
    /// User ID (GUID)
    pub id: String,

    /// Email address
    pub email: String,

    /// Display name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Email verified flag
    #[serde(default)]
    pub email_verified: bool,

    /// Premium subscription flag
    #[serde(default)]
    pub premium: bool,

    /// Security stamp (for session validation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub security_stamp: Option<String>,
}
