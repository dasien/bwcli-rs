//! Vault service module
//!
//! Provides high-level vault operations coordinating between storage, API client, and SDK.

use crate::models::vault::{CipherView, CollectionView, FolderView, Organization, VaultData};
use crate::services::api::BitwardenApiClient;
use crate::services::sdk::Client;
use crate::services::storage::{JsonFileStorage, Storage};
use std::sync::Arc;
use tokio::sync::Mutex;

pub mod cipher_service;
pub mod confirmation_service;
pub mod errors;
pub mod search_service;
pub mod sync_service;
pub mod totp_service;
pub mod validation_service;
pub mod write_service;

pub use cipher_service::CipherService;
pub use confirmation_service::ConfirmationService;
pub use errors::VaultError;
pub use search_service::{ItemFilters, SearchService};
pub use sync_service::SyncService;
pub use totp_service::TotpService;
pub use validation_service::ValidationService;
pub use write_service::WriteService;

/// Field types for extraction
#[derive(Debug, Clone, Copy)]
pub enum FieldType {
    Username,
    Password,
    Uri,
    Notes,
}

/// Main vault service coordinating all vault operations
pub struct VaultService {
    sync_service: SyncService,
    cipher_service: CipherService,
    search_service: SearchService,
    totp_service: TotpService,
    storage: Arc<Mutex<JsonFileStorage>>,
}

impl VaultService {
    /// Create new vault service
    pub fn new(
        api_client: Arc<BitwardenApiClient>,
        storage: Arc<Mutex<JsonFileStorage>>,
        sdk_client: Arc<Client>,
    ) -> Self {
        let sync_service = SyncService::new(Arc::clone(&api_client), Arc::clone(&storage));

        let cipher_service = CipherService::new(Arc::clone(&sdk_client));

        let search_service = SearchService::new();

        let totp_service = TotpService::new(Arc::clone(&sdk_client));

        Self {
            sync_service,
            cipher_service,
            search_service,
            totp_service,
            storage,
        }
    }

    // Sync operations

    /// Sync vault from server
    pub async fn sync(&self, force: bool) -> Result<String, VaultError> {
        self.sync_service.sync(force).await
    }

    /// Get last sync timestamp
    pub async fn get_last_sync(&self) -> Result<Option<String>, VaultError> {
        self.sync_service.get_last_sync().await
    }

    // List operations

    /// List all items with optional filters
    pub async fn list_items(&self, filters: &ItemFilters) -> Result<Vec<CipherView>, VaultError> {
        let vault_data = self.get_vault_data().await?;
        let filtered = self
            .search_service
            .filter_ciphers(&vault_data.ciphers, filters);
        self.cipher_service.decrypt_ciphers(&filtered).await
    }

    /// List all folders
    pub async fn list_folders(&self, search: Option<&str>) -> Result<Vec<FolderView>, VaultError> {
        let vault_data = self.get_vault_data().await?;
        let mut folders = self
            .cipher_service
            .decrypt_folders(&vault_data.folders)
            .await?;

        if let Some(search_term) = search {
            folders = self.search_service.filter_folders(folders, search_term);
        }

        Ok(folders)
    }

    /// List all collections
    pub async fn list_collections(
        &self,
        organization_id: Option<&str>,
        search: Option<&str>,
    ) -> Result<Vec<CollectionView>, VaultError> {
        let vault_data = self.get_vault_data().await?;
        let mut collections = self
            .cipher_service
            .decrypt_collections(&vault_data.collections)
            .await?;

        // Filter by organization
        if let Some(org_id) = organization_id {
            collections.retain(|c| c.organization_id == org_id);
        }

        // Filter by search
        if let Some(search_term) = search {
            collections = self
                .search_service
                .filter_collections(collections, search_term);
        }

        Ok(collections)
    }

    /// List all organizations
    pub async fn list_organizations(&self) -> Result<Vec<Organization>, VaultError> {
        let vault_data = self.get_vault_data().await?;
        Ok(vault_data.organizations)
    }

    // Get operations

    /// Get specific item by ID or search term
    pub async fn get_item(&self, id_or_search: &str) -> Result<CipherView, VaultError> {
        let vault_data = self.get_vault_data().await?;

        // Try to find by ID first
        let cipher = if let Some(cipher) = vault_data.ciphers.iter().find(|c| c.id == id_or_search)
        {
            cipher
        } else {
            // Search by name
            self.search_service
                .find_cipher_by_name(&vault_data.ciphers, id_or_search)
                .ok_or(VaultError::ItemNotFound)?
        };

        self.cipher_service.decrypt_cipher(cipher).await
    }

    /// Get specific field from item
    pub async fn get_field(
        &self,
        id_or_search: &str,
        field: FieldType,
    ) -> Result<String, VaultError> {
        let cipher_view = self.get_item(id_or_search).await?;
        self.extract_field(&cipher_view, field)
    }

    /// Generate TOTP code for item
    pub async fn get_totp(&self, id_or_search: &str) -> Result<String, VaultError> {
        let cipher_view = self.get_item(id_or_search).await?;

        let totp_secret = cipher_view
            .login
            .as_ref()
            .and_then(|l| l.totp.as_ref())
            .ok_or(VaultError::TotpNotConfigured)?;

        self.totp_service.generate_code(totp_secret).await
    }

    // Helper methods

    async fn get_vault_data(&self) -> Result<VaultData, VaultError> {
        let storage = self.storage.lock().await;
        storage
            .get::<VaultData>("vaultData")
            .map_err(|e| VaultError::StorageError(e.to_string()))?
            .ok_or(VaultError::NotSynced)
    }

    fn extract_field(&self, cipher: &CipherView, field: FieldType) -> Result<String, VaultError> {
        match field {
            FieldType::Username => cipher
                .login
                .as_ref()
                .and_then(|l| l.username.clone())
                .ok_or(VaultError::FieldNotFound("username")),
            FieldType::Password => cipher
                .login
                .as_ref()
                .and_then(|l| l.password.clone())
                .ok_or(VaultError::FieldNotFound("password")),
            FieldType::Uri => cipher
                .login
                .as_ref()
                .and_then(|l| l.uris.first())
                .and_then(|u| u.uri.clone())
                .ok_or(VaultError::FieldNotFound("uri")),
            FieldType::Notes => cipher
                .notes
                .clone()
                .ok_or(VaultError::FieldNotFound("notes")),
        }
    }
}
