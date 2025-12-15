//! Sync response and vault data models
//!
//! Uses SDK API models for parsing and SDK domain types for storage.

use super::Organization;
use bitwarden_api_api::models::SyncResponseModel;
use bitwarden_collections::{collection::Collection, error::CollectionsParseError};
use bitwarden_vault::{Cipher, Folder, VaultParseError};
use serde::{Deserialize, Serialize};

/// Parse raw API sync response into SDK domain types
pub fn parse_sync_response(response: SyncResponseModel) -> Result<SyncData, VaultParseError> {
    let ciphers = response
        .ciphers
        .unwrap_or_default()
        .into_iter()
        .map(|c| c.try_into())
        .collect::<Result<Vec<Cipher>, _>>()?;

    let folders = response
        .folders
        .unwrap_or_default()
        .into_iter()
        .map(|f| f.try_into())
        .collect::<Result<Vec<Folder>, _>>()?;

    let collections = response
        .collections
        .unwrap_or_default()
        .into_iter()
        .map(|c| c.try_into())
        .collect::<Result<Vec<Collection>, _>>()
        .map_err(|e: CollectionsParseError| match e {
            CollectionsParseError::Crypto(c) => VaultParseError::Crypto(c),
            CollectionsParseError::MissingField(m) => VaultParseError::MissingField(m),
        })?;

    Ok(SyncData {
        ciphers,
        folders,
        collections,
    })
}

/// Parsed sync data with SDK domain types
#[derive(Debug)]
pub struct SyncData {
    pub ciphers: Vec<Cipher>,
    pub folders: Vec<Folder>,
    pub collections: Vec<Collection>,
}

/// Vault data stored in local storage
///
/// Uses SDK types for storage. Serializes to JSON format compatible with TypeScript CLI.
#[derive(Debug, Serialize, Deserialize)]
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
