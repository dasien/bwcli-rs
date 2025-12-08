//! Sync response and vault data models

use super::{Cipher, Collection, Folder, Organization};
use serde::{Deserialize, Serialize};

/// API sync endpoint response
///
/// Contains complete vault state from server.
/// Returned by GET /api/sync
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

    /// Organizations
    #[serde(default)]
    pub organizations: Vec<Organization>,

    /// Profile information (optional, not used in MVP)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<serde_json::Value>,

    /// Domains (optional, not used in MVP)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domains: Option<serde_json::Value>,
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
