//! Collection data models

use serde::{Deserialize, Serialize};

/// Collection (shared folder within organization)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Collection {
    /// Collection ID (UUID)
    pub id: String,

    /// Organization ID
    pub organization_id: String,

    /// Encrypted collection name (EncString)
    pub name: String,

    /// External ID (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,

    /// Read only flag
    #[serde(default)]
    pub read_only: bool,
}

/// Decrypted collection view
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionView {
    pub id: String,
    pub organization_id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
    #[serde(default)]
    pub read_only: bool,
}
