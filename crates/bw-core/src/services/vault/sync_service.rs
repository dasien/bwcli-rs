//! Vault synchronization service
//!
//! Handles downloading vault data from Bitwarden API and caching locally.
//! Uses TypeScript CLI compatible flat storage format with user-namespaced keys.

use super::errors::VaultError;
use crate::models::vault::SyncResponse;
use crate::services::api::{ApiClient, BitwardenApiClient, endpoints};
use crate::services::storage::{AccountManager, JsonFileStorage, Storage, StorageKey};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Service for vault synchronization operations
pub struct SyncService {
    api_client: Arc<BitwardenApiClient>,
    storage: Arc<Mutex<JsonFileStorage>>,
}

impl SyncService {
    pub fn new(api_client: Arc<BitwardenApiClient>, storage: Arc<Mutex<JsonFileStorage>>) -> Self {
        Self {
            api_client,
            storage,
        }
    }

    /// Sync vault from server
    ///
    /// # Arguments
    /// * `force` - Force full sync even if recently synced
    ///
    /// # Returns
    /// Last sync timestamp (ISO 8601 format)
    pub async fn sync(&self, _force: bool) -> Result<String, VaultError> {
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

        // Fetch vault data from API
        let sync_response: SyncResponse = self
            .api_client
            .get_with_auth(endpoints::api::SYNC)
            .await
            .map_err(|e| VaultError::ApiError(e.to_string()))?;

        // Store vault data using TypeScript CLI compatible flat keys
        // Convert Vec to HashMap<id, item> for storage (matches TypeScript CLI format)
        let now = chrono::Utc::now().to_rfc3339();
        let mut storage = self.storage.lock().await;

        // Convert ciphers Vec to HashMap keyed by ID
        let ciphers_map: HashMap<String, _> = sync_response
            .ciphers
            .into_iter()
            .map(|c| (c.id.clone(), c))
            .collect();
        storage
            .set(
                &StorageKey::UserCiphers.format(Some(&user_id)),
                &ciphers_map,
            )
            .await
            .map_err(|e| VaultError::StorageError(e.to_string()))?;

        // Convert folders Vec to HashMap keyed by ID
        let folders_map: HashMap<String, _> = sync_response
            .folders
            .into_iter()
            .map(|f| (f.id.clone(), f))
            .collect();
        storage
            .set(
                &StorageKey::UserFolders.format(Some(&user_id)),
                &folders_map,
            )
            .await
            .map_err(|e| VaultError::StorageError(e.to_string()))?;

        // Convert collections Vec to HashMap keyed by ID
        let collections_map: HashMap<String, _> = sync_response
            .collections
            .into_iter()
            .map(|c| (c.id.clone(), c))
            .collect();
        storage
            .set(
                &StorageKey::UserCollections.format(Some(&user_id)),
                &collections_map,
            )
            .await
            .map_err(|e| VaultError::StorageError(e.to_string()))?;

        // Organizations come from profile in sync response, not separate field
        // Extract from profile if available and convert to HashMap
        let organizations: HashMap<String, serde_json::Value> = sync_response
            .profile
            .as_ref()
            .and_then(|p| p.get("organizations"))
            .and_then(|o| o.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|org| {
                        org.get("id")
                            .and_then(|id| id.as_str())
                            .map(|id| (id.to_string(), org.clone()))
                    })
                    .collect()
            })
            .unwrap_or_default();

        storage
            .set(
                &StorageKey::UserOrganizations.format(Some(&user_id)),
                &organizations,
            )
            .await
            .map_err(|e| VaultError::StorageError(e.to_string()))?;

        storage
            .set(
                &StorageKey::UserLastSync.format(Some(&user_id)),
                &now,
            )
            .await
            .map_err(|e| VaultError::StorageError(e.to_string()))?;

        Ok(now)
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
