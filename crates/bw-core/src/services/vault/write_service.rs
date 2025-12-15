//! Write service for vault CRUD operations
//!
//! Coordinates validation, encryption, API calls, and cache updates for
//! creating, updating, and deleting vault items and folders.
//!
//! NOTE: Write operations require the SDK Client to be initialized with keys.

use super::{CipherService, ConfirmationService, ValidationService, VaultError};
use crate::models::vault::{
    Cipher, CipherId, CipherRequestModel, CipherView, Folder, FolderId, FolderRequestModel,
    FolderView,
};
use crate::services::api::{ApiClient, BitwardenApiClient, endpoints};
use crate::services::storage::{AccountManager, JsonFileStorage, Storage, StorageKey};
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Service for vault write operations (create, update, delete)
pub struct WriteService {
    api_client: Arc<BitwardenApiClient>,
    storage: Arc<Mutex<JsonFileStorage>>,
    cipher_service: Arc<CipherService>,
    validation_service: Arc<ValidationService>,
    confirmation_service: Arc<ConfirmationService>,
    account_manager: Arc<AccountManager>,
}

impl WriteService {
    /// Create new write service
    pub fn new(
        api_client: Arc<BitwardenApiClient>,
        storage: Arc<Mutex<JsonFileStorage>>,
        cipher_service: Arc<CipherService>,
        validation_service: Arc<ValidationService>,
        confirmation_service: Arc<ConfirmationService>,
        account_manager: Arc<AccountManager>,
    ) -> Self {
        Self {
            api_client,
            storage,
            cipher_service,
            validation_service,
            confirmation_service,
            account_manager,
        }
    }

    /// Get the active user ID
    async fn get_user_id(&self) -> Result<String, VaultError> {
        self.account_manager
            .get_active_user_id()
            .await
            .map_err(|e| VaultError::StorageError(e.to_string()))?
            .ok_or(VaultError::NotAuthenticated)
    }

    // ========== Cipher Operations ==========

    /// Create new cipher (item)
    pub async fn create_cipher(
        &self,
        mut cipher_view: CipherView,
        _session: &str,
    ) -> Result<Cipher, VaultError> {
        // 1. Validate input structure
        self.validation_service
            .validate_cipher_create(&cipher_view)?;

        // 2. Generate ID if not present (SDK uses Option<CipherId>)
        if cipher_view.id.is_none() {
            cipher_view.id = Some(CipherId::new(uuid::Uuid::new_v4()));
        }

        // 3. Set timestamps (SDK uses DateTime<Utc>)
        let now = Utc::now();
        cipher_view.revision_date = now;
        cipher_view.creation_date = now;

        // 4. Encrypt using SDK (returns EncryptionContext)
        let encryption_context = self.cipher_service.encrypt_cipher(cipher_view)?;

        // 5. Convert to API request format
        let request: CipherRequestModel = encryption_context.into();

        // 6. Send to API
        let created: Cipher = self
            .api_client
            .post_with_auth(endpoints::api::ciphers::BASE, &request)
            .await
            .map_err(|e| VaultError::ApiError(e.to_string()))?;

        // 7. Update local cache
        self.add_cipher_to_cache(&created).await?;

        Ok(created)
    }

    /// Update existing cipher
    pub async fn update_cipher(
        &self,
        id: &str,
        mut cipher_view: CipherView,
        _session: &str,
    ) -> Result<Cipher, VaultError> {
        // 1. Validate item exists
        self.validate_cipher_exists(id).await?;

        // 2. Parse and set ID
        let cipher_id = id
            .parse::<uuid::Uuid>()
            .map_err(|_| VaultError::ItemNotFound)?;
        cipher_view.id = Some(CipherId::new(cipher_id));

        // 3. Validate update structure
        self.validation_service
            .validate_cipher_update(&cipher_view)?;

        // 4. Update timestamp
        cipher_view.revision_date = Utc::now();

        // 5. Encrypt using SDK
        let encryption_context = self.cipher_service.encrypt_cipher(cipher_view)?;

        // 6. Convert to API request format
        let request: CipherRequestModel = encryption_context.into();

        // 7. Send to API
        let updated: Cipher = self
            .api_client
            .put_with_auth(&endpoints::api::ciphers::by_id(id), &request)
            .await
            .map_err(|e| VaultError::ApiError(e.to_string()))?;

        // 8. Update cache
        self.update_cipher_in_cache(&updated).await?;

        Ok(updated)
    }

    /// Delete cipher (soft or permanent)
    pub async fn delete_cipher(
        &self,
        id: &str,
        permanent: bool,
        no_interaction: bool,
    ) -> Result<(), VaultError> {
        // 1. Validate item exists
        self.validate_cipher_exists(id).await?;

        // 2. Confirm if permanent
        if permanent && !no_interaction {
            if !self.confirmation_service.confirm_permanent_delete()? {
                return Err(VaultError::OperationCancelled);
            }
        }

        // 3. Send delete to API
        if permanent {
            self.api_client
                .delete_with_auth(&endpoints::api::ciphers::by_id(id))
                .await
                .map_err(|e| VaultError::ApiError(e.to_string()))?;
        } else {
            self.api_client
                .put_with_auth_no_response(&endpoints::api::ciphers::delete(id))
                .await
                .map_err(|e| VaultError::ApiError(e.to_string()))?;
        }

        // 4. Update cache
        if permanent {
            self.remove_cipher_from_cache(id).await?;
        } else {
            self.mark_cipher_deleted(id).await?;
        }

        Ok(())
    }

    /// Restore cipher from trash
    pub async fn restore_cipher(&self, id: &str) -> Result<Cipher, VaultError> {
        // 1. Validate item exists and is deleted
        self.validate_cipher_deleted(id).await?;

        // 2. Send restore to API
        let restored: Cipher = self
            .api_client
            .put_with_auth(
                &endpoints::api::ciphers::restore(id),
                &serde_json::json!({}),
            )
            .await
            .map_err(|e| VaultError::ApiError(e.to_string()))?;

        // 3. Update cache
        self.update_cipher_in_cache(&restored).await?;

        Ok(restored)
    }

    /// Move cipher to different folder
    pub async fn move_cipher(
        &self,
        cipher_id: &str,
        folder_id: Option<&str>,
        session: &str,
    ) -> Result<Cipher, VaultError> {
        // 1. Validate cipher exists
        self.validate_cipher_exists(cipher_id).await?;

        // 2. Validate folder exists if specified
        if let Some(fid) = folder_id {
            self.validate_folder_exists(fid).await?;
        }

        // 3. Get current cipher
        let cipher = self.get_cipher(cipher_id).await?;

        // 4. Decrypt cipher
        let mut cipher_view = self.cipher_service.decrypt_cipher(cipher)?;

        // 5. Update folder ID (SDK uses Option<FolderId>)
        cipher_view.folder_id = folder_id
            .map(|fid| {
                fid.parse::<uuid::Uuid>()
                    .map(FolderId::new)
                    .ok()
            })
            .flatten();

        // 6. Update via API
        self.update_cipher(cipher_id, cipher_view, session).await
    }

    // ========== Folder Operations ==========

    /// Create folder
    pub async fn create_folder(&self, name: String, _session: &str) -> Result<Folder, VaultError> {
        // 1. Validate name
        self.validation_service.validate_folder_name(&name)?;

        // 2. Create folder view and encrypt
        let folder_view = FolderView {
            id: None,
            name,
            revision_date: Utc::now(),
        };
        let encrypted = self.cipher_service.encrypt_folder(folder_view)?;

        // 3. Create request model
        let folder_request = FolderRequestModel {
            name: encrypted.name.to_string(),
        };

        // 4. Send to API
        let created: Folder = self
            .api_client
            .post_with_auth(endpoints::api::folders::BASE, &folder_request)
            .await
            .map_err(|e| VaultError::ApiError(e.to_string()))?;

        // 5. Update cache
        self.add_folder_to_cache(&created).await?;

        Ok(created)
    }

    /// Update folder name
    pub async fn update_folder(
        &self,
        id: &str,
        name: String,
        _session: &str,
    ) -> Result<Folder, VaultError> {
        // 1. Validate folder exists
        self.validate_folder_exists(id).await?;

        // 2. Validate name
        self.validation_service.validate_folder_name(&name)?;

        // 3. Parse folder ID
        let folder_id = id
            .parse::<uuid::Uuid>()
            .map_err(|_| VaultError::FolderNotFound)?;

        // 4. Create folder view and encrypt
        let folder_view = FolderView {
            id: Some(FolderId::new(folder_id)),
            name,
            revision_date: Utc::now(),
        };
        let encrypted = self.cipher_service.encrypt_folder(folder_view)?;

        // 5. Create request model
        let folder_request = FolderRequestModel {
            name: encrypted.name.to_string(),
        };

        // 6. Send to API
        let updated: Folder = self
            .api_client
            .put_with_auth(&endpoints::api::folders::by_id(id), &folder_request)
            .await
            .map_err(|e| VaultError::ApiError(e.to_string()))?;

        // 7. Update cache
        self.update_folder_in_cache(&updated).await?;

        Ok(updated)
    }

    /// Delete folder
    pub async fn delete_folder(&self, id: &str) -> Result<(), VaultError> {
        // 1. Validate folder exists
        self.validate_folder_exists(id).await?;

        // 2. Send delete to API
        self.api_client
            .delete_with_auth(&endpoints::api::folders::by_id(id))
            .await
            .map_err(|e| VaultError::ApiError(e.to_string()))?;

        // 3. Remove from cache
        self.remove_folder_from_cache(id).await?;

        Ok(())
    }

    // ========== Cache Management (Private) ==========

    async fn add_cipher_to_cache(&self, cipher: &Cipher) -> Result<(), VaultError> {
        let user_id = self.get_user_id().await?;
        let mut storage = self.storage.lock().await;

        let mut ciphers: HashMap<String, Cipher> = storage
            .get(&StorageKey::UserCiphers.format(Some(&user_id)))
            .map_err(|e| VaultError::StorageError(e.to_string()))?
            .unwrap_or_default();

        // Add cipher by ID (SDK uses Option<CipherId>)
        if let Some(id) = &cipher.id {
            ciphers.insert(id.to_string(), cipher.clone());
        }

        storage
            .set(&StorageKey::UserCiphers.format(Some(&user_id)), &ciphers)
            .await
            .map_err(|e| VaultError::StorageError(e.to_string()))?;

        storage
            .set(
                &StorageKey::UserLastSync.format(Some(&user_id)),
                &Utc::now().to_rfc3339(),
            )
            .await
            .map_err(|e| VaultError::StorageError(e.to_string()))?;

        Ok(())
    }

    async fn update_cipher_in_cache(&self, cipher: &Cipher) -> Result<(), VaultError> {
        let user_id = self.get_user_id().await?;
        let mut storage = self.storage.lock().await;

        let mut ciphers: HashMap<String, Cipher> = storage
            .get(&StorageKey::UserCiphers.format(Some(&user_id)))
            .map_err(|e| VaultError::StorageError(e.to_string()))?
            .ok_or(VaultError::NotSynced)?;

        if let Some(id) = &cipher.id {
            let id_str = id.to_string();
            if !ciphers.contains_key(&id_str) {
                return Err(VaultError::ItemNotFound);
            }
            ciphers.insert(id_str, cipher.clone());
        } else {
            return Err(VaultError::ItemNotFound);
        }

        storage
            .set(&StorageKey::UserCiphers.format(Some(&user_id)), &ciphers)
            .await
            .map_err(|e| VaultError::StorageError(e.to_string()))?;

        storage
            .set(
                &StorageKey::UserLastSync.format(Some(&user_id)),
                &Utc::now().to_rfc3339(),
            )
            .await
            .map_err(|e| VaultError::StorageError(e.to_string()))?;

        Ok(())
    }

    async fn remove_cipher_from_cache(&self, id: &str) -> Result<(), VaultError> {
        let user_id = self.get_user_id().await?;
        let mut storage = self.storage.lock().await;

        let mut ciphers: HashMap<String, Cipher> = storage
            .get(&StorageKey::UserCiphers.format(Some(&user_id)))
            .map_err(|e| VaultError::StorageError(e.to_string()))?
            .ok_or(VaultError::NotSynced)?;

        ciphers.remove(id);

        storage
            .set(&StorageKey::UserCiphers.format(Some(&user_id)), &ciphers)
            .await
            .map_err(|e| VaultError::StorageError(e.to_string()))?;

        storage
            .set(
                &StorageKey::UserLastSync.format(Some(&user_id)),
                &Utc::now().to_rfc3339(),
            )
            .await
            .map_err(|e| VaultError::StorageError(e.to_string()))?;

        Ok(())
    }

    async fn mark_cipher_deleted(&self, id: &str) -> Result<(), VaultError> {
        let user_id = self.get_user_id().await?;
        let mut storage = self.storage.lock().await;

        let mut ciphers: HashMap<String, Cipher> = storage
            .get(&StorageKey::UserCiphers.format(Some(&user_id)))
            .map_err(|e| VaultError::StorageError(e.to_string()))?
            .ok_or(VaultError::NotSynced)?;

        if let Some(cipher) = ciphers.get_mut(id) {
            cipher.deleted_date = Some(Utc::now());
        } else {
            return Err(VaultError::ItemNotFound);
        }

        storage
            .set(&StorageKey::UserCiphers.format(Some(&user_id)), &ciphers)
            .await
            .map_err(|e| VaultError::StorageError(e.to_string()))?;

        storage
            .set(
                &StorageKey::UserLastSync.format(Some(&user_id)),
                &Utc::now().to_rfc3339(),
            )
            .await
            .map_err(|e| VaultError::StorageError(e.to_string()))?;

        Ok(())
    }

    async fn add_folder_to_cache(&self, folder: &Folder) -> Result<(), VaultError> {
        let user_id = self.get_user_id().await?;
        let mut storage = self.storage.lock().await;

        let mut folders: HashMap<String, Folder> = storage
            .get(&StorageKey::UserFolders.format(Some(&user_id)))
            .map_err(|e| VaultError::StorageError(e.to_string()))?
            .unwrap_or_default();

        if let Some(id) = &folder.id {
            folders.insert(id.to_string(), folder.clone());
        }

        storage
            .set(&StorageKey::UserFolders.format(Some(&user_id)), &folders)
            .await
            .map_err(|e| VaultError::StorageError(e.to_string()))?;

        storage
            .set(
                &StorageKey::UserLastSync.format(Some(&user_id)),
                &Utc::now().to_rfc3339(),
            )
            .await
            .map_err(|e| VaultError::StorageError(e.to_string()))?;

        Ok(())
    }

    async fn update_folder_in_cache(&self, folder: &Folder) -> Result<(), VaultError> {
        let user_id = self.get_user_id().await?;
        let mut storage = self.storage.lock().await;

        let mut folders: HashMap<String, Folder> = storage
            .get(&StorageKey::UserFolders.format(Some(&user_id)))
            .map_err(|e| VaultError::StorageError(e.to_string()))?
            .ok_or(VaultError::NotSynced)?;

        if let Some(id) = &folder.id {
            let id_str = id.to_string();
            if !folders.contains_key(&id_str) {
                return Err(VaultError::FolderNotFound);
            }
            folders.insert(id_str, folder.clone());
        } else {
            return Err(VaultError::FolderNotFound);
        }

        storage
            .set(&StorageKey::UserFolders.format(Some(&user_id)), &folders)
            .await
            .map_err(|e| VaultError::StorageError(e.to_string()))?;

        storage
            .set(
                &StorageKey::UserLastSync.format(Some(&user_id)),
                &Utc::now().to_rfc3339(),
            )
            .await
            .map_err(|e| VaultError::StorageError(e.to_string()))?;

        Ok(())
    }

    async fn remove_folder_from_cache(&self, id: &str) -> Result<(), VaultError> {
        let user_id = self.get_user_id().await?;
        let mut storage = self.storage.lock().await;

        let mut folders: HashMap<String, Folder> = storage
            .get(&StorageKey::UserFolders.format(Some(&user_id)))
            .map_err(|e| VaultError::StorageError(e.to_string()))?
            .ok_or(VaultError::NotSynced)?;

        folders.remove(id);

        storage
            .set(&StorageKey::UserFolders.format(Some(&user_id)), &folders)
            .await
            .map_err(|e| VaultError::StorageError(e.to_string()))?;

        storage
            .set(
                &StorageKey::UserLastSync.format(Some(&user_id)),
                &Utc::now().to_rfc3339(),
            )
            .await
            .map_err(|e| VaultError::StorageError(e.to_string()))?;

        Ok(())
    }

    // ========== Validation Helpers ==========

    async fn validate_cipher_exists(&self, id: &str) -> Result<(), VaultError> {
        let user_id = self.get_user_id().await?;
        let storage = self.storage.lock().await;
        let ciphers: HashMap<String, Cipher> = storage
            .get(&StorageKey::UserCiphers.format(Some(&user_id)))
            .map_err(|e| VaultError::StorageError(e.to_string()))?
            .ok_or(VaultError::NotSynced)?;

        if !ciphers.contains_key(id) {
            return Err(VaultError::ItemNotFound);
        }

        Ok(())
    }

    async fn validate_cipher_deleted(&self, id: &str) -> Result<(), VaultError> {
        let user_id = self.get_user_id().await?;
        let storage = self.storage.lock().await;
        let ciphers: HashMap<String, Cipher> = storage
            .get(&StorageKey::UserCiphers.format(Some(&user_id)))
            .map_err(|e| VaultError::StorageError(e.to_string()))?
            .ok_or(VaultError::NotSynced)?;

        let cipher = ciphers.get(id).ok_or(VaultError::ItemNotFound)?;

        if cipher.deleted_date.is_none() {
            return Err(VaultError::ItemNotDeleted);
        }

        Ok(())
    }

    async fn validate_folder_exists(&self, id: &str) -> Result<(), VaultError> {
        let user_id = self.get_user_id().await?;
        let storage = self.storage.lock().await;
        let folders: HashMap<String, Folder> = storage
            .get(&StorageKey::UserFolders.format(Some(&user_id)))
            .map_err(|e| VaultError::StorageError(e.to_string()))?
            .ok_or(VaultError::NotSynced)?;

        if !folders.contains_key(id) {
            return Err(VaultError::FolderNotFound);
        }

        Ok(())
    }

    async fn get_cipher(&self, id: &str) -> Result<Cipher, VaultError> {
        let user_id = self.get_user_id().await?;
        let storage = self.storage.lock().await;
        let ciphers: HashMap<String, Cipher> = storage
            .get(&StorageKey::UserCiphers.format(Some(&user_id)))
            .map_err(|e| VaultError::StorageError(e.to_string()))?
            .ok_or(VaultError::NotSynced)?;

        ciphers.get(id).cloned().ok_or(VaultError::ItemNotFound)
    }
}
