//! Write service for vault CRUD operations
//!
//! Coordinates validation, encryption, API calls, and cache updates for
//! creating, updating, and deleting vault items and folders.
//!
//! NOTE: Write operations require the user key for encryption. The session parameter
//! must be provided to decrypt the user key from protected storage.

use super::{CipherService, ConfirmationService, ValidationService, VaultError};
use crate::models::vault::{Cipher, CipherRequest, CipherView, Folder, FolderRequest};
use crate::services::api::{ApiClient, BitwardenApiClient, endpoints};
use crate::services::key_service::KeyService;
use crate::services::storage::{AccountManager, JsonFileStorage, Storage, StorageKey};
use bitwarden_crypto::SymmetricCryptoKey;
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
    key_service: KeyService,
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
        let key_service = KeyService::new(Arc::clone(&storage), Arc::clone(&account_manager));
        Self {
            api_client,
            storage,
            cipher_service,
            validation_service,
            confirmation_service,
            key_service,
            account_manager,
        }
    }

    /// Get the user key from protected storage using the session key
    async fn get_user_key(&self, session: &str) -> Result<SymmetricCryptoKey, VaultError> {
        self.key_service
            .get_user_key(session)
            .await
            .map_err(|e| VaultError::DecryptionError(e.to_string()))
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
        session: &str,
    ) -> Result<Cipher, VaultError> {
        // 1. Validate input structure BEFORE attempting key retrieval
        // This ensures validation errors are returned even with invalid sessions
        self.validation_service
            .validate_cipher_create(&cipher_view)?;

        // 2. Get user key for encryption (only after validation passes)
        let user_key = self.get_user_key(session).await?;

        // 3. Generate ID if not present
        if cipher_view.id.is_empty() {
            cipher_view.id = uuid::Uuid::new_v4().to_string();
        }

        // Set timestamps
        let now = Utc::now().to_rfc3339();
        cipher_view.revision_date = now.clone();
        if cipher_view.creation_date.is_none() {
            cipher_view.creation_date = Some(now);
        }

        // 4. Encrypt using SDK
        let encrypted = self
            .cipher_service
            .encrypt_cipher(&cipher_view, &user_key)
            .await?;

        // 5. Convert to request format
        let request = CipherRequest::from(encrypted.clone());

        // 6. Send to API
        let created: Cipher = self
            .api_client
            .post_with_auth(endpoints::api::ciphers::BASE, &request)
            .await
            .map_err(|e| VaultError::ApiError(e.to_string()))?;

        // 7. Update local cache (atomic)
        self.add_cipher_to_cache(&created).await?;

        Ok(created)
    }

    /// Update existing cipher
    pub async fn update_cipher(
        &self,
        id: &str,
        mut cipher_view: CipherView,
        session: &str,
    ) -> Result<Cipher, VaultError> {
        // 1. Validate item exists first
        self.validate_cipher_exists(id).await?;

        // 2. Ensure ID matches
        cipher_view.id = id.to_string();

        // 3. Validate update structure BEFORE attempting key retrieval
        self.validation_service
            .validate_cipher_update(&cipher_view)?;

        // 4. Get user key for encryption (only after validation passes)
        let user_key = self.get_user_key(session).await?;

        // 5. Update timestamp
        cipher_view.revision_date = Utc::now().to_rfc3339();

        // 6. Encrypt using SDK
        let encrypted = self
            .cipher_service
            .encrypt_cipher(&cipher_view, &user_key)
            .await?;

        // 7. Convert to request format
        let request = CipherRequest::from(encrypted);

        // 8. Send to API
        let updated: Cipher = self
            .api_client
            .put_with_auth(&endpoints::api::ciphers::by_id(id), &request)
            .await
            .map_err(|e| VaultError::ApiError(e.to_string()))?;

        // 9. Update cache atomically
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
        // Soft delete uses PUT to /ciphers/{id}/delete (moves to trash)
        // Permanent delete uses DELETE to /ciphers/{id} (permanently removes)
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
            // Soft delete - mark with deletedDate
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
        // 1. Validate cipher exists first
        self.validate_cipher_exists(cipher_id).await?;

        // 2. Validate folder exists if specified
        if let Some(fid) = folder_id {
            self.validate_folder_exists(fid).await?;
        }

        // 3. Get user key for decryption/encryption (only after validation passes)
        let user_key = self.get_user_key(session).await?;

        // 4. Get current cipher
        let cipher = self.get_cipher(cipher_id).await?;

        // 5. Decrypt, update folder, re-encrypt
        let mut cipher_view = self
            .cipher_service
            .decrypt_cipher(&cipher, &user_key)
            .await?;
        cipher_view.folder_id = folder_id.map(String::from);

        // 6. Update via API
        self.update_cipher(cipher_id, cipher_view, session).await
    }

    // ========== Folder Operations ==========

    /// Create folder
    pub async fn create_folder(&self, name: String, session: &str) -> Result<Folder, VaultError> {
        // 1. Validate name BEFORE attempting key retrieval
        self.validation_service.validate_folder_name(&name)?;

        // 2. Get user key for encryption (only after validation passes)
        let user_key = self.get_user_key(session).await?;

        // 3. Encrypt folder name using SDK
        let encrypted_name = self.cipher_service.encrypt_string(&name, &user_key)?;

        // 4. Send to API
        let folder_request = FolderRequest {
            name: encrypted_name,
        };
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
        session: &str,
    ) -> Result<Folder, VaultError> {
        // 1. Validate folder exists first
        self.validate_folder_exists(id).await?;

        // 2. Validate name BEFORE attempting key retrieval
        self.validation_service.validate_folder_name(&name)?;

        // 3. Get user key for encryption (only after validation passes)
        let user_key = self.get_user_key(session).await?;

        // 4. Encrypt folder name
        let encrypted_name = self.cipher_service.encrypt_string(&name, &user_key)?;

        // 5. Send to API
        let folder_request = FolderRequest {
            name: encrypted_name,
        };
        let updated: Folder = self
            .api_client
            .put_with_auth(&endpoints::api::folders::by_id(id), &folder_request)
            .await
            .map_err(|e| VaultError::ApiError(e.to_string()))?;

        // 6. Update cache
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

        // Get current ciphers (HashMap keyed by ID)
        let mut ciphers: HashMap<String, Cipher> = storage
            .get(&StorageKey::UserCiphers.format(Some(&user_id)))
            .map_err(|e| VaultError::StorageError(e.to_string()))?
            .ok_or(VaultError::NotSynced)?;

        // Add new cipher by ID
        ciphers.insert(cipher.id.clone(), cipher.clone());

        // Write ciphers
        storage
            .set(&StorageKey::UserCiphers.format(Some(&user_id)), &ciphers)
            .await
            .map_err(|e| VaultError::StorageError(e.to_string()))?;

        // Update last sync timestamp
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

        // Check cipher exists, then update by ID
        if !ciphers.contains_key(&cipher.id) {
            return Err(VaultError::ItemNotFound);
        }
        ciphers.insert(cipher.id.clone(), cipher.clone());

        // Write ciphers
        storage
            .set(&StorageKey::UserCiphers.format(Some(&user_id)), &ciphers)
            .await
            .map_err(|e| VaultError::StorageError(e.to_string()))?;

        // Update last sync timestamp
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

        // Remove cipher by ID
        ciphers.remove(id);

        // Write ciphers
        storage
            .set(&StorageKey::UserCiphers.format(Some(&user_id)), &ciphers)
            .await
            .map_err(|e| VaultError::StorageError(e.to_string()))?;

        // Update last sync timestamp
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

        // Find and mark deleted by ID
        if let Some(cipher) = ciphers.get_mut(id) {
            cipher.deleted_date = Some(Utc::now().to_rfc3339());
        } else {
            return Err(VaultError::ItemNotFound);
        }

        // Write ciphers
        storage
            .set(&StorageKey::UserCiphers.format(Some(&user_id)), &ciphers)
            .await
            .map_err(|e| VaultError::StorageError(e.to_string()))?;

        // Update last sync timestamp
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
            .ok_or(VaultError::NotSynced)?;

        folders.insert(folder.id.clone(), folder.clone());

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

        if !folders.contains_key(&folder.id) {
            return Err(VaultError::FolderNotFound);
        }
        folders.insert(folder.id.clone(), folder.clone());

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
