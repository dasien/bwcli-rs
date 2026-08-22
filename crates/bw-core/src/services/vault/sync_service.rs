//! Vault synchronization service
//!
//! Handles downloading vault data from Bitwarden API and caching locally.
//! Uses TypeScript CLI compatible flat storage format with user-namespaced keys.

use super::errors::VaultError;
use crate::models::vault::parse_sync_response;
use crate::services::api::{ApiClient, BitwardenApiClient};
use crate::services::storage::{AccountManager, JsonFileStorage, Storage, StorageKey};
use bitwarden_core::Client;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Service for vault synchronization operations
pub struct SyncService {
    api_client: Arc<BitwardenApiClient>,
    storage: Arc<Mutex<JsonFileStorage>>,
    sdk: Arc<Client>,
}

impl SyncService {
    pub fn new(
        api_client: Arc<BitwardenApiClient>,
        storage: Arc<Mutex<JsonFileStorage>>,
        sdk: Arc<Client>,
    ) -> Self {
        Self {
            api_client,
            storage,
            sdk,
        }
    }

    /// Generated API clients, authenticated via the SDK's token handler.
    fn api(&self) -> std::sync::Arc<bitwarden_core::client::ApiConfigurations> {
        self.sdk.internal.get_api_configurations()
    }

    /// Sync vault from server
    ///
    /// Skips the download when the server's account revision date is no newer
    /// than our last sync, unless `force` is set.
    ///
    /// # Arguments
    /// * `force` - Sync even if the server reports no changes
    ///
    /// # Returns
    /// Last sync timestamp (ISO 8601 format)
    pub async fn sync(&self, force: bool) -> Result<String, VaultError> {
        // Check authentication
        if !self.api_client.is_authenticated().await {
            return Err(VaultError::NotAuthenticated);
        }

        // Get active user ID for storage keys
        let account_manager = AccountManager::new(Arc::clone(&self.storage));
        let user_id = account_manager
            .get_active_user_id()
            .await
            .map_err(|e| VaultError::StorageError(e.to_string()))?
            .ok_or(VaultError::NotAuthenticated)?;

        if !force && !self.needs_sync().await? {
            // Nothing changed server-side; report the existing timestamp so
            // callers still see when the vault was last refreshed.
            if let Some(last_sync) = self.get_last_sync().await? {
                return Ok(last_sync);
            }
        }

        // Fetch vault data through the SDK's generated client. Its response
        // models are all-Option and tolerant of unknown fields, unlike
        // hand-rolled ones.
        let sync_response = self
            .api()
            .api_client
            .sync_api()
            .get(None)
            .await
            .map_err(|e| VaultError::ApiError(e.to_string()))?;

        // Parse into SDK domain types
        let sync_data = parse_sync_response(sync_response)
            .map_err(|e| VaultError::ApiError(format!("Failed to parse sync response: {}", e)))?;

        // Store vault data using TypeScript CLI compatible flat keys
        // Convert Vec to HashMap<id, item> for storage (matches TypeScript CLI format)
        let now = chrono::Utc::now().to_rfc3339();
        let mut storage = self.storage.lock().await;

        // Convert ciphers Vec to HashMap keyed by ID
        let ciphers_map: HashMap<String, _> = sync_data
            .ciphers
            .into_iter()
            .filter_map(|c| c.id.map(|id| (id.to_string(), c)))
            .collect();
        storage
            .set(
                &StorageKey::UserCiphers.format(Some(&user_id)),
                &ciphers_map,
            )
            .await
            .map_err(|e| VaultError::StorageError(e.to_string()))?;

        // Convert folders Vec to HashMap keyed by ID
        let folders_map: HashMap<String, _> = sync_data
            .folders
            .into_iter()
            .filter_map(|f| f.id.map(|id| (id.to_string(), f)))
            .collect();
        storage
            .set(
                &StorageKey::UserFolders.format(Some(&user_id)),
                &folders_map,
            )
            .await
            .map_err(|e| VaultError::StorageError(e.to_string()))?;

        // Convert collections Vec to HashMap keyed by ID
        let collections_map: HashMap<String, _> = sync_data
            .collections
            .into_iter()
            .filter_map(|c| c.id.map(|id| (id.to_string(), c)))
            .collect();
        storage
            .set(
                &StorageKey::UserCollections.format(Some(&user_id)),
                &collections_map,
            )
            .await
            .map_err(|e| VaultError::StorageError(e.to_string()))?;

        // Organizations live under the sync response's profile. Without this
        // the key is never written and `bw list organizations` always returns
        // an empty list.
        let organizations_map: HashMap<String, _> = sync_data
            .organizations
            .into_iter()
            .map(|o| (o.id.clone(), o))
            .collect();
        storage
            .set(
                &StorageKey::UserOrganizations.format(Some(&user_id)),
                &organizations_map,
            )
            .await
            .map_err(|e| VaultError::StorageError(e.to_string()))?;

        // Sends, keyed by id, so the SDK's Repository<Send> adapter can read them.
        let sends_map: HashMap<String, _> = sync_data
            .sends
            .into_iter()
            .filter_map(|s| s.id.map(|id| (id.to_string(), s)))
            .collect();
        storage
            .set(&StorageKey::UserSends.format(Some(&user_id)), &sends_map)
            .await
            .map_err(|e| VaultError::StorageError(e.to_string()))?;

        storage
            .set(&StorageKey::UserLastSync.format(Some(&user_id)), &now)
            .await
            .map_err(|e| VaultError::StorageError(e.to_string()))?;

        Ok(now)
    }

    /// Whether the server has changes we don't have yet.
    ///
    /// Compares the account revision date against our stored `lastSync`. Errs
    /// on the side of syncing: any missing or unparseable timestamp returns
    /// `true`.
    async fn needs_sync(&self) -> Result<bool, VaultError> {
        let Some(last_sync) = self.get_last_sync().await? else {
            return Ok(true);
        };

        let Ok(last_sync) = chrono::DateTime::parse_from_rfc3339(&last_sync) else {
            tracing::debug!("Stored lastSync is not valid RFC3339; syncing");
            return Ok(true);
        };

        let revision_ms: i64 = match self
            .api()
            .api_client
            .accounts_api()
            .get_account_revision_date()
            .await
        {
            Ok(ms) => ms,
            Err(e) => {
                // A revision-date probe failure shouldn't block a sync.
                tracing::debug!("Could not fetch account revision date ({e}); syncing");
                return Ok(true);
            }
        };

        // The server signals a deleted account with a negative timestamp.
        if revision_ms < 0 {
            return Err(VaultError::ApiError(
                "This account no longer exists on the server.".to_string(),
            ));
        }

        let Some(revision) = chrono::DateTime::from_timestamp_millis(revision_ms) else {
            tracing::debug!("Server returned an out-of-range revision date; syncing");
            return Ok(true);
        };

        Ok(revision > last_sync.with_timezone(&chrono::Utc))
    }

    /// Get last sync timestamp
    pub async fn get_last_sync(&self) -> Result<Option<String>, VaultError> {
        // Get active user ID
        let account_manager = AccountManager::new(Arc::clone(&self.storage));
        let user_id = match account_manager.get_active_user_id().await {
            Ok(Some(id)) => id,
            _ => return Ok(None),
        };

        let storage = self.storage.lock().await;
        let last_sync: Option<String> = storage
            .get(&StorageKey::UserLastSync.format(Some(&user_id)))
            .map_err(|e| VaultError::StorageError(e.to_string()))?;

        Ok(last_sync)
    }

    pub fn storage(&self) -> &Arc<Mutex<JsonFileStorage>> {
        &self.storage
    }
}
