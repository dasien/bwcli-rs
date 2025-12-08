use serde::{Deserialize, Serialize};

/// File Send data
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendFile {
    /// Encrypted file name (EncString)
    pub file_name: String,

    /// File size in bytes
    pub size: u64,

    /// File size string (human-readable)
    pub size_name: String,

    /// File ID
    pub id: String,
}
