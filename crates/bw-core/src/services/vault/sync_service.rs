//! Vault synchronization service
//!
//! Handles downloading vault data from Bitwarden API and caching locally.

use super::errors::VaultError;
use crate::models::vault::{SyncResponse, VaultData};
use crate::services::api::{ApiClient, BitwardenApiClient};
use crate::services::storage::{JsonFileStorage, Storage};
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

        // Fetch vault data from API
        // Note: path is relative to api_url (https://api.bitwarden.com), so no /api prefix
        let sync_response: SyncResponse = self
            .api_client
            .get_with_auth("/sync")
            .await
            .map_err(|e| VaultError::ApiError(e.to_string()))?;

        // Create vault data structure
        let now = chrono::Utc::now().to_rfc3339();
        let vault_data = VaultData {
            last_sync: now.clone(),
            ciphers: sync_response.ciphers,
            folders: sync_response.folders,
            collections: sync_response.collections,
            organizations: vec![], // Organizations come from profile, not sync response
        };

        // Store in local storage (atomic operation)
        let mut storage = self.storage.lock().await;
        storage
            .set("vaultData", &vault_data)
            .await
            .map_err(|e| VaultError::StorageError(e.to_string()))?;

        Ok(now)
    }

    /// Get last sync timestamp
    pub async fn get_last_sync(&self) -> Result<Option<String>, VaultError> {
        let storage = self.storage.lock().await;
        let vault_data: Option<VaultData> = storage
            .get("vaultData")
            .map_err(|e| VaultError::StorageError(e.to_string()))?;

        Ok(vault_data.map(|v| v.last_sync))
    }

    pub fn storage(&self) -> &Arc<Mutex<JsonFileStorage>> {
        &self.storage
    }
}
