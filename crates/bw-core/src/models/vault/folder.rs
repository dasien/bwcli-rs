//! Folder data models

use serde::{Deserialize, Serialize};

/// Encrypted folder
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Folder {
    /// Folder ID (UUID)
    pub id: String,

    /// Encrypted folder name (EncString)
    pub name: String,

    /// Revision date (ISO 8601)
    pub revision_date: String,
}

/// Decrypted folder view
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderView {
    pub id: String,
    pub name: String,
    pub revision_date: String,
}
