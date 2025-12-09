//! Sync response and vault data models

use super::{Cipher, Collection, Folder, Organization};
use serde::{Deserialize, Serialize};

/// API sync endpoint response
///
/// Contains complete vault state from server.
/// Returned by GET /sync
/// Note: API uses camelCase for top-level field names
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncResponse {
    /// Encrypted ciphers (vault items)
    #[serde(default)]
    pub ciphers: Vec<Cipher>,

    /// Encrypted folders
    #[serde(default)]
    pub folders: Vec<Folder>,

    /// Collections
    #[serde(default)]
    pub collections: Vec<Collection>,

    /// Profile information
    #[serde(default)]
    pub profile: Option<serde_json::Value>,

    /// Domains
    #[serde(default)]
    pub domains: Option<serde_json::Value>,

    /// Policies
    #[serde(default)]
    pub policies: Option<serde_json::Value>,

    /// Sends
    #[serde(default)]
    pub sends: Option<serde_json::Value>,
}

/// Vault data stored in local storage
///
/// Persisted to data.json after successful sync.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultData {
    /// Last sync timestamp (ISO 8601)
    pub last_sync: String,

    /// Encrypted ciphers
    #[serde(default)]
    pub ciphers: Vec<Cipher>,

    /// Encrypted folders
    #[serde(default)]
    pub folders: Vec<Folder>,

    /// Collections
    #[serde(default)]
    pub collections: Vec<Collection>,

    /// Organizations
    #[serde(default)]
    pub organizations: Vec<Organization>,
}
