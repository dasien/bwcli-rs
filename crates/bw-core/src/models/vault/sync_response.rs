//! Sync response and vault data models
//!
//! Uses SDK API models for parsing and SDK domain types for storage.

use super::Organization;
use bitwarden_api_api::models::SyncResponseModel;
use bitwarden_collections::{collection::Collection, error::CollectionsParseError};
use bitwarden_core::OrganizationId;
use bitwarden_crypto::UnsignedSharedKey;
use bitwarden_send::Send;
use bitwarden_vault::{Cipher, Folder, VaultParseError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

    // Organizations come from the profile rather than a top-level list. The
    // server sends both `organizations` and `organizationsNew`; newer clients
    // are expected to prefer the latter and fall back.
    let profile_organizations = response
        .profile
        .as_deref()
        .and_then(|profile| {
            profile
                .organizations_new
                .as_ref()
                .or(profile.organizations.as_ref())
        })
        .map(Vec::as_slice)
        .unwrap_or_default();

    let organizations = profile_organizations
        .iter()
        .filter_map(Organization::from_api)
        .collect();

    // An organization whose key we cannot read is skipped rather than failing the
    // sync: its items simply stay undecryptable, which is the same outcome as
    // before and better than no sync at all.
    let organization_keys = profile_organizations
        .iter()
        .filter_map(|o| {
            let id = OrganizationId::new(o.id?);
            match o.key.as_deref()?.parse::<UnsignedSharedKey>() {
                Ok(key) => Some((id, key)),
                Err(e) => {
                    tracing::warn!("Skipping unreadable key for organization {id}: {e}");
                    None
                }
            }
        })
        .collect();

    // Sends ride along on the sync response. Individual failures are skipped
    // rather than failing the whole sync, matching how the SDK treats them.
    let sends = response
        .sends
        .unwrap_or_default()
        .into_iter()
        .filter_map(|s| match Send::try_from(s) {
            Ok(send) => Some(send),
            Err(e) => {
                tracing::warn!("Skipping send that failed to parse: {e}");
                None
            }
        })
        .collect();

    Ok(SyncData {
        ciphers,
        folders,
        collections,
        organizations,
        sends,
        organization_keys,
    })
}

/// Parsed sync data with SDK domain types
#[derive(Debug)]
pub struct SyncData {
    pub ciphers: Vec<Cipher>,
    pub folders: Vec<Folder>,
    pub collections: Vec<Collection>,
    pub organizations: Vec<Organization>,
    pub sends: Vec<Send>,
    /// Each organization's shared key, wrapped to the user's public key.
    ///
    /// Kept apart from [`Organization`], which is the shape `bw list
    /// organizations` prints — a key does not belong in user-facing output. The
    /// SDK needs these to decrypt organization-owned items and to encrypt an item
    /// being shared into an organization.
    pub organization_keys: HashMap<OrganizationId, UnsignedSharedKey>,
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
