//! Vault service module
//!
//! Provides high-level vault operations coordinating between storage, API client, and SDK.

use crate::models::vault::{
    Cipher, CipherListView, CipherView, Collection, CollectionView, Folder, FolderView,
    Organization, OrganizationId,
};
use crate::services::api::BitwardenApiClient;
use crate::services::storage::{AccountManager, JsonFileStorage, Storage, StorageKey};
use bitwarden_core::Client;
use std::collections::HashMap;
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
    account_manager: Arc<AccountManager>,
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
        let cipher_service = CipherService::new(sdk_client);
        let search_service = SearchService::new();
        let totp_service = TotpService::new();

        Self {
            sync_service,
            cipher_service,
            search_service,
            totp_service,
            storage,
            account_manager,
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
    /// Returns CipherListView for efficiency (less data than full CipherView)
    ///
    /// # Arguments
    /// * `filters` - Optional filters to apply to the results
    /// * `_session` - BW_SESSION key (SDK handles keys internally)
    pub async fn list_items(
        &self,
        filters: &ItemFilters,
        _session: &str,
    ) -> Result<Vec<CipherListView>, VaultError> {
        let ciphers = self.get_ciphers().await?;

        // Filters expressible on encrypted metadata first, so we decrypt as
        // little as possible.
        let filtered = self.search_service.filter_ciphers(&ciphers, filters);
        let cipher_vec: Vec<Cipher> = filtered.into_values().collect();
        let decrypted = self.cipher_service.decrypt_ciphers(cipher_vec).await?;

        // `--search` and `--url` need plaintext, so they apply after.
        Ok(self.search_service.filter_decrypted(decrypted, filters))
    }

    /// List all folders
    ///
    /// # Arguments
    /// * `search` - Optional search term to filter folders
    /// * `_session` - BW_SESSION key (SDK handles keys internally)
    pub async fn list_folders(
        &self,
        search: Option<&str>,
        _session: &str,
    ) -> Result<Vec<FolderView>, VaultError> {
        let folders = self.get_folders().await?;
        let folders_vec: Vec<Folder> = folders.into_values().collect();
        let mut decrypted_folders = self.cipher_service.decrypt_folders(folders_vec)?;

        if let Some(search_term) = search {
            decrypted_folders = self
                .search_service
                .filter_folders(decrypted_folders, search_term);
        }

        Ok(decrypted_folders)
    }

    /// List all collections
    ///
    /// # Arguments
    /// * `organization_id` - Optional organization ID to filter by
    /// * `search` - Optional search term to filter collections
    /// * `_session` - BW_SESSION key (SDK handles keys internally)
    pub async fn list_collections(
        &self,
        organization_id: Option<&str>,
        search: Option<&str>,
        _session: &str,
    ) -> Result<Vec<CollectionView>, VaultError> {
        let collections = self.get_collections().await?;
        let collections_vec: Vec<Collection> = collections.into_values().collect();
        let mut decrypted_collections = self.cipher_service.decrypt_collections(collections_vec)?;

        // Filter by organization (SDK uses OrganizationId)
        if let Some(org_id_str) = organization_id {
            if let Ok(org_uuid) = org_id_str.parse::<uuid::Uuid>() {
                let org_id = OrganizationId::new(org_uuid);
                decrypted_collections.retain(|c| c.organization_id == org_id);
            }
        }

        // Filter by search
        if let Some(search_term) = search {
            decrypted_collections = self
                .search_service
                .filter_collections(decrypted_collections, search_term);
        }

        Ok(decrypted_collections)
    }

    /// List all organizations
    pub async fn list_organizations(&self) -> Result<Vec<Organization>, VaultError> {
        let orgs = self.get_organizations_data().await?;
        Ok(orgs.into_values().collect())
    }

    // Get operations

    /// Get specific item by ID or search term
    ///
    /// # Arguments
    /// * `id_or_search` - ID or search term to find the item
    /// * `_session` - BW_SESSION key (SDK handles keys internally)
    pub async fn get_item(
        &self,
        id_or_search: &str,
        _session: &str,
    ) -> Result<CipherView, VaultError> {
        let ciphers = self.get_ciphers().await?;

        // Fast path: exact id lookup needs no decryption.
        if let Some(cipher) = ciphers.get(id_or_search) {
            return self.cipher_service.decrypt_cipher(cipher.clone()).await;
        }

        // Otherwise names are encrypted, so searching requires decrypting.
        let candidates: Vec<Cipher> = ciphers
            .values()
            .filter(|c| c.deleted_date.is_none())
            .cloned()
            .collect();
        let list = self.cipher_service.decrypt_ciphers(candidates).await?;

        let mut matches = self.search_service.find_by_name(&list, id_or_search);

        match matches.len() {
            0 => Err(VaultError::ItemNotFound),
            1 => {
                let (id, _) = matches.remove(0);
                let cipher = ciphers
                    .get(&id.to_string())
                    .ok_or(VaultError::ItemNotFound)?
                    .clone();
                self.cipher_service.decrypt_cipher(cipher).await
            }
            // Returning an arbitrary match would silently hand back the wrong
            // secret (`bw get password <name>`), so make the caller disambiguate.
            _ => Err(VaultError::MultipleItemsFound {
                search: id_or_search.to_string(),
                matches: matches
                    .iter()
                    .map(|(id, name)| format!("{name} ({id})"))
                    .collect::<Vec<_>>()
                    .join(", "),
            }),
        }
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

    /// Get the active user ID
    async fn get_user_id(&self) -> Result<String, VaultError> {
        self.account_manager
            .get_active_user_id()
            .await
            .map_err(|e| VaultError::StorageError(e.to_string()))?
            .ok_or(VaultError::NotAuthenticated)
    }

    /// Get ciphers from flat storage (stored as HashMap<id, Cipher>)
    async fn get_ciphers(&self) -> Result<HashMap<String, Cipher>, VaultError> {
        let user_id = self.get_user_id().await?;
        let storage = self.storage.lock().await;
        let key = StorageKey::UserCiphers.format(Some(&user_id));
        storage
            .get::<HashMap<String, Cipher>>(&key)
            .map_err(|e| Self::cache_error(&key, e))?
            .ok_or(VaultError::NotSynced)
    }

    /// Encrypted ciphers as stored, excluding trash.
    ///
    /// Exposed for callers that hand ciphers to SDK APIs which decrypt
    /// internally (export), so they must not be pre-decrypted.
    pub async fn encrypted_ciphers(&self) -> Result<Vec<Cipher>, VaultError> {
        Ok(self
            .get_ciphers()
            .await?
            .into_values()
            .filter(|c| c.deleted_date.is_none())
            .collect())
    }

    /// Encrypted folders as stored. See [`Self::encrypted_ciphers`].
    pub async fn encrypted_folders(&self) -> Result<Vec<Folder>, VaultError> {
        Ok(self.get_folders().await?.into_values().collect())
    }

    /// Turn a cache deserialization failure into something actionable.
    ///
    /// Vault data is stored as SDK types, several of which use
    /// `deny_unknown_fields`. Cache written by an older client can therefore
    /// carry a field the current SDK rejects, which fails the *whole* map and
    /// leaves the CLI unable to read the vault. Re-syncing rewrites it.
    fn cache_error(key: &str, e: impl std::fmt::Display) -> VaultError {
        VaultError::StorageError(format!(
            "cached vault data at '{key}' could not be read ({e}). \
             Run 'bw sync --force' to refresh it."
        ))
    }

    /// Get folders from flat storage (stored as HashMap<id, Folder>)
    async fn get_folders(&self) -> Result<HashMap<String, Folder>, VaultError> {
        let user_id = self.get_user_id().await?;
        let storage = self.storage.lock().await;
        let key = StorageKey::UserFolders.format(Some(&user_id));
        storage
            .get::<HashMap<String, Folder>>(&key)
            .map_err(|e| Self::cache_error(&key, e))?
            .ok_or(VaultError::NotSynced)
    }

    /// Get collections from flat storage (stored as HashMap<id, Collection>)
    async fn get_collections(&self) -> Result<HashMap<String, Collection>, VaultError> {
        let user_id = self.get_user_id().await?;
        let storage = self.storage.lock().await;
        let key = StorageKey::UserCollections.format(Some(&user_id));
        storage
            .get::<HashMap<String, Collection>>(&key)
            .map_err(|e| Self::cache_error(&key, e))?
            .ok_or(VaultError::NotSynced)
    }

    /// Get organizations from flat storage (stored as HashMap<id, Organization>)
    async fn get_organizations_data(&self) -> Result<HashMap<String, Organization>, VaultError> {
        let user_id = self.get_user_id().await?;
        let storage = self.storage.lock().await;
        Ok(storage
            .get::<HashMap<String, Organization>>(
                &StorageKey::UserOrganizations.format(Some(&user_id)),
            )
            .map_err(|e| VaultError::StorageError(e.to_string()))?
            .unwrap_or_default())
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
                .and_then(|l| l.uris.as_ref())
                .and_then(|uris| uris.first())
                .and_then(|u| u.uri.clone())
                .ok_or(VaultError::FieldNotFound("uri")),
            FieldType::Notes => cipher
                .notes
                .clone()
                .ok_or(VaultError::FieldNotFound("notes")),
        }
    }
}
