//! Vault service module
//!
//! Provides high-level vault operations coordinating between storage, API client, and SDK.

use crate::models::vault::{CipherView, CollectionView, FolderView, Organization, VaultData};
use crate::services::api::BitwardenApiClient;
use crate::services::key_service::KeyService;
use crate::services::sdk::Client;
use crate::services::storage::{AccountManager, JsonFileStorage, Storage};
use bitwarden_crypto::SymmetricCryptoKey;
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
    key_service: KeyService,
    storage: Arc<Mutex<JsonFileStorage>>,
}

impl VaultService {
    /// Create new vault service
    pub fn new(
        api_client: Arc<BitwardenApiClient>,
        storage: Arc<Mutex<JsonFileStorage>>,
        sdk_client: Arc<Client>,
        account_manager: Arc<AccountManager>,
    ) -> Self {
        let sync_service = SyncService::new(Arc::clone(&api_client), Arc::clone(&storage));

        let cipher_service = CipherService::new(Arc::clone(&sdk_client));

        let search_service = SearchService::new();

        let totp_service = TotpService::new();

        let key_service = KeyService::new(Arc::clone(&storage), account_manager);

        Self {
            sync_service,
            cipher_service,
            search_service,
            totp_service,
            key_service,
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
    ///
    /// # Arguments
    /// * `filters` - Optional filters to apply to the results
    /// * `session` - BW_SESSION key for decryption
    pub async fn list_items(
        &self,
        filters: &ItemFilters,
        session: &str,
    ) -> Result<Vec<CipherView>, VaultError> {
        let user_key = self.get_user_key(session).await?;
        let vault_data = self.get_vault_data().await?;
        let filtered = self
            .search_service
            .filter_ciphers(&vault_data.ciphers, filters);
        self.cipher_service
            .decrypt_ciphers(&filtered, &user_key)
            .await
    }

    /// List all folders
    ///
    /// # Arguments
    /// * `search` - Optional search term to filter folders
    /// * `session` - BW_SESSION key for decryption
    pub async fn list_folders(
        &self,
        search: Option<&str>,
        session: &str,
    ) -> Result<Vec<FolderView>, VaultError> {
        let user_key = self.get_user_key(session).await?;
        let vault_data = self.get_vault_data().await?;
        let mut folders = self
            .cipher_service
            .decrypt_folders(&vault_data.folders, &user_key)
            .await?;

        if let Some(search_term) = search {
            folders = self.search_service.filter_folders(folders, search_term);
        }

        Ok(folders)
    }

    /// List all collections
    ///
    /// # Arguments
    /// * `organization_id` - Optional organization ID to filter by
    /// * `search` - Optional search term to filter collections
    /// * `session` - BW_SESSION key for decryption
    pub async fn list_collections(
        &self,
        organization_id: Option<&str>,
        search: Option<&str>,
        session: &str,
    ) -> Result<Vec<CollectionView>, VaultError> {
        let user_key = self.get_user_key(session).await?;
        let vault_data = self.get_vault_data().await?;
        let mut collections = self
            .cipher_service
            .decrypt_collections(&vault_data.collections, &user_key)
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
    ///
    /// # Arguments
    /// * `id_or_search` - ID or search term to find the item
    /// * `session` - BW_SESSION key for decryption
    pub async fn get_item(
        &self,
        id_or_search: &str,
        session: &str,
    ) -> Result<CipherView, VaultError> {
        let user_key = self.get_user_key(session).await?;
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

        self.cipher_service.decrypt_cipher(cipher, &user_key).await
    }

    /// Get specific field from item
    ///
    /// # Arguments
    /// * `id_or_search` - ID or search term to find the item
    /// * `field` - Field type to extract
    /// * `session` - BW_SESSION key for decryption
    pub async fn get_field(
        &self,
        id_or_search: &str,
        field: FieldType,
        session: &str,
    ) -> Result<String, VaultError> {
        let cipher_view = self.get_item(id_or_search, session).await?;
        self.extract_field(&cipher_view, field)
    }

    /// Generate TOTP code for item
    ///
    /// # Arguments
    /// * `id_or_search` - ID or search term to find the item
    /// * `session` - BW_SESSION key for decryption
    pub async fn get_totp(&self, id_or_search: &str, session: &str) -> Result<String, VaultError> {
        let cipher_view = self.get_item(id_or_search, session).await?;

        let totp_secret = cipher_view
            .login
            .as_ref()
            .and_then(|l| l.totp.as_ref())
            .ok_or(VaultError::TotpNotConfigured)?;

        self.totp_service.generate_code(totp_secret).await
    }

    // Helper methods

    /// Get the user key from protected storage using the session key
    async fn get_user_key(&self, session: &str) -> Result<SymmetricCryptoKey, VaultError> {
        self.key_service
            .get_user_key(session)
            .await
            .map_err(|e| VaultError::DecryptionError(e.to_string()))
    }

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
