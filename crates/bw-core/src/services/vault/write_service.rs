//! Write service for vault CRUD operations
//!
//! Coordinates validation, encryption, API calls, and cache updates for
//! creating, updating, and deleting vault items and folders.

use super::{CipherService, ConfirmationService, ValidationService, VaultError};
use crate::models::vault::{Cipher, CipherRequest, CipherView, Folder, FolderRequest, VaultData};
use crate::services::api::{ApiClient, BitwardenApiClient};
use crate::services::storage::{JsonFileStorage, Storage};
use chrono::Utc;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Service for vault write operations (create, update, delete)
pub struct WriteService {
    api_client: Arc<BitwardenApiClient>,
    storage: Arc<Mutex<JsonFileStorage>>,
    cipher_service: Arc<CipherService>,
    validation_service: Arc<ValidationService>,
    confirmation_service: Arc<ConfirmationService>,
}

impl WriteService {
    /// Create new write service
    pub fn new(
        api_client: Arc<BitwardenApiClient>,
        storage: Arc<Mutex<JsonFileStorage>>,
        cipher_service: Arc<CipherService>,
        validation_service: Arc<ValidationService>,
        confirmation_service: Arc<ConfirmationService>,
    ) -> Self {
        Self {
            api_client,
            storage,
            cipher_service,
            validation_service,
            confirmation_service,
        }
    }

    // ========== Cipher Operations ==========

    /// Create new cipher (item)
    pub async fn create_cipher(&self, mut cipher_view: CipherView) -> Result<Cipher, VaultError> {
        // 1. Validate input structure
        self.validation_service
            .validate_cipher_create(&cipher_view)?;

        // 2. Generate ID if not present
        if cipher_view.id.is_empty() {
            cipher_view.id = uuid::Uuid::new_v4().to_string();
        }

        // Set timestamps
        let now = Utc::now().to_rfc3339();
        cipher_view.revision_date = now.clone();
        if cipher_view.creation_date.is_none() {
            cipher_view.creation_date = Some(now);
        }

        // 3. Encrypt using SDK
        let encrypted = self.cipher_service.encrypt_cipher(&cipher_view).await?;

        // 4. Convert to request format
        let request = CipherRequest::from(encrypted.clone());

        // 5. Send to API
        let created: Cipher = self
            .api_client
            .post_with_auth("/api/ciphers", &request)
            .await
            .map_err(|e| VaultError::ApiError(e.to_string()))?;

        // 6. Update local cache (atomic)
        self.add_cipher_to_cache(&created).await?;

        Ok(created)
    }

    /// Update existing cipher
    pub async fn update_cipher(
        &self,
        id: &str,
        mut cipher_view: CipherView,
    ) -> Result<Cipher, VaultError> {
        // 1. Validate item exists
        self.validate_cipher_exists(id).await?;

        // 2. Ensure ID matches
        cipher_view.id = id.to_string();

        // 3. Validate update structure
        self.validation_service
            .validate_cipher_update(&cipher_view)?;

        // 4. Update timestamp
        cipher_view.revision_date = Utc::now().to_rfc3339();

        // 5. Encrypt using SDK
        let encrypted = self.cipher_service.encrypt_cipher(&cipher_view).await?;

        // 6. Convert to request format
        let request = CipherRequest::from(encrypted);

        // 7. Send to API
        let updated: Cipher = self
            .api_client
            .put_with_auth(&format!("/api/ciphers/{}", id), &request)
            .await
            .map_err(|e| VaultError::ApiError(e.to_string()))?;

        // 8. Update cache atomically
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
        let endpoint = if permanent {
            format!("/api/ciphers/{}/delete", id)
        } else {
            format!("/api/ciphers/{}", id)
        };

        self.api_client
            .delete_with_auth(&endpoint)
            .await
            .map_err(|e| VaultError::ApiError(e.to_string()))?;

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
                &format!("/api/ciphers/{}/restore", id),
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
    ) -> Result<Cipher, VaultError> {
        // 1. Validate cipher exists
        self.validate_cipher_exists(cipher_id).await?;

        // 2. Validate folder exists if specified
        if let Some(fid) = folder_id {
            self.validate_folder_exists(fid).await?;
        }

        // 3. Get current cipher
        let cipher = self.get_cipher(cipher_id).await?;

        // 4. Decrypt, update folder, re-encrypt
        let mut cipher_view = self.cipher_service.decrypt_cipher(&cipher).await?;
        cipher_view.folder_id = folder_id.map(String::from);

        // 5. Update via API
        self.update_cipher(cipher_id, cipher_view).await
    }

    // ========== Folder Operations ==========

    /// Create folder
    pub async fn create_folder(&self, name: String) -> Result<Folder, VaultError> {
        // 1. Validate name
        self.validation_service.validate_folder_name(&name)?;

        // 2. Encrypt folder name using SDK
        let encrypted_name = self.cipher_service.encrypt_string(&name).await?;

        // 3. Send to API
        let folder_request = FolderRequest {
            name: encrypted_name,
        };
        let created: Folder = self
            .api_client
            .post_with_auth("/api/folders", &folder_request)
            .await
            .map_err(|e| VaultError::ApiError(e.to_string()))?;

        // 4. Update cache
        self.add_folder_to_cache(&created).await?;

        Ok(created)
    }

    /// Update folder name
    pub async fn update_folder(&self, id: &str, name: String) -> Result<Folder, VaultError> {
        // 1. Validate folder exists
        self.validate_folder_exists(id).await?;

        // 2. Validate name
        self.validation_service.validate_folder_name(&name)?;

        // 3. Encrypt folder name
        let encrypted_name = self.cipher_service.encrypt_string(&name).await?;

        // 4. Send to API
        let folder_request = FolderRequest {
            name: encrypted_name,
        };
        let updated: Folder = self
            .api_client
            .put_with_auth(&format!("/api/folders/{}", id), &folder_request)
            .await
            .map_err(|e| VaultError::ApiError(e.to_string()))?;

        // 5. Update cache
        self.update_folder_in_cache(&updated).await?;

        Ok(updated)
    }

    /// Delete folder
    pub async fn delete_folder(&self, id: &str) -> Result<(), VaultError> {
        // 1. Validate folder exists
        self.validate_folder_exists(id).await?;

        // 2. Send delete to API
        self.api_client
            .delete_with_auth(&format!("/api/folders/{}", id))
            .await
            .map_err(|e| VaultError::ApiError(e.to_string()))?;

        // 3. Remove from cache
        self.remove_folder_from_cache(id).await?;

        Ok(())
    }

    // ========== Cache Management (Private) ==========

    async fn add_cipher_to_cache(&self, cipher: &Cipher) -> Result<(), VaultError> {
        let mut storage = self.storage.lock().await;

        // Get current vault data
        let mut vault_data: VaultData = storage
            .get("vaultData")
            .map_err(|e| VaultError::StorageError(e.to_string()))?
            .ok_or(VaultError::NotSynced)?;

        // Add new cipher
        vault_data.ciphers.push(cipher.clone());

        // Update timestamp
        vault_data.last_sync = Utc::now().to_rfc3339();

        // Write atomically
        storage
            .set("vaultData", &vault_data)
            .await
            .map_err(|e| VaultError::StorageError(e.to_string()))?;

        Ok(())
    }

    async fn update_cipher_in_cache(&self, cipher: &Cipher) -> Result<(), VaultError> {
        let mut storage = self.storage.lock().await;

        let mut vault_data: VaultData = storage
            .get("vaultData")
            .map_err(|e| VaultError::StorageError(e.to_string()))?
            .ok_or(VaultError::NotSynced)?;

        // Find and replace cipher
        if let Some(index) = vault_data.ciphers.iter().position(|c| c.id == cipher.id) {
            vault_data.ciphers[index] = cipher.clone();
        } else {
            return Err(VaultError::ItemNotFound);
        }

        // Update timestamp
        vault_data.last_sync = Utc::now().to_rfc3339();

        // Write atomically
        storage
            .set("vaultData", &vault_data)
            .await
            .map_err(|e| VaultError::StorageError(e.to_string()))?;

        Ok(())
    }

    async fn remove_cipher_from_cache(&self, id: &str) -> Result<(), VaultError> {
        let mut storage = self.storage.lock().await;

        let mut vault_data: VaultData = storage
            .get("vaultData")
            .map_err(|e| VaultError::StorageError(e.to_string()))?
            .ok_or(VaultError::NotSynced)?;

        // Remove cipher
        vault_data.ciphers.retain(|c| c.id != id);

        // Update timestamp
        vault_data.last_sync = Utc::now().to_rfc3339();

        // Write atomically
        storage
            .set("vaultData", &vault_data)
            .await
            .map_err(|e| VaultError::StorageError(e.to_string()))?;

        Ok(())
    }

    async fn mark_cipher_deleted(&self, id: &str) -> Result<(), VaultError> {
        let mut storage = self.storage.lock().await;

        let mut vault_data: VaultData = storage
            .get("vaultData")
            .map_err(|e| VaultError::StorageError(e.to_string()))?
            .ok_or(VaultError::NotSynced)?;

        // Find and mark deleted
        if let Some(cipher) = vault_data.ciphers.iter_mut().find(|c| c.id == id) {
            cipher.deleted_date = Some(Utc::now().to_rfc3339());
        } else {
            return Err(VaultError::ItemNotFound);
        }

        // Update timestamp
        vault_data.last_sync = Utc::now().to_rfc3339();

        // Write atomically
        storage
            .set("vaultData", &vault_data)
            .await
            .map_err(|e| VaultError::StorageError(e.to_string()))?;

        Ok(())
    }

    async fn add_folder_to_cache(&self, folder: &Folder) -> Result<(), VaultError> {
        let mut storage = self.storage.lock().await;

        let mut vault_data: VaultData = storage
            .get("vaultData")
            .map_err(|e| VaultError::StorageError(e.to_string()))?
            .ok_or(VaultError::NotSynced)?;

        vault_data.folders.push(folder.clone());
        vault_data.last_sync = Utc::now().to_rfc3339();

        storage
            .set("vaultData", &vault_data)
            .await
            .map_err(|e| VaultError::StorageError(e.to_string()))?;

        Ok(())
    }

    async fn update_folder_in_cache(&self, folder: &Folder) -> Result<(), VaultError> {
        let mut storage = self.storage.lock().await;

        let mut vault_data: VaultData = storage
            .get("vaultData")
            .map_err(|e| VaultError::StorageError(e.to_string()))?
            .ok_or(VaultError::NotSynced)?;

        if let Some(index) = vault_data.folders.iter().position(|f| f.id == folder.id) {
            vault_data.folders[index] = folder.clone();
        } else {
            return Err(VaultError::FolderNotFound);
        }

        vault_data.last_sync = Utc::now().to_rfc3339();

        storage
            .set("vaultData", &vault_data)
            .await
            .map_err(|e| VaultError::StorageError(e.to_string()))?;

        Ok(())
    }

    async fn remove_folder_from_cache(&self, id: &str) -> Result<(), VaultError> {
        let mut storage = self.storage.lock().await;

        let mut vault_data: VaultData = storage
            .get("vaultData")
            .map_err(|e| VaultError::StorageError(e.to_string()))?
            .ok_or(VaultError::NotSynced)?;

        vault_data.folders.retain(|f| f.id != id);
        vault_data.last_sync = Utc::now().to_rfc3339();

        storage
            .set("vaultData", &vault_data)
            .await
            .map_err(|e| VaultError::StorageError(e.to_string()))?;

        Ok(())
    }

    // ========== Validation Helpers ==========

    async fn validate_cipher_exists(&self, id: &str) -> Result<(), VaultError> {
        let storage = self.storage.lock().await;
        let vault_data: VaultData = storage
            .get("vaultData")
            .map_err(|e| VaultError::StorageError(e.to_string()))?
            .ok_or(VaultError::NotSynced)?;

        vault_data
            .ciphers
            .iter()
            .find(|c| c.id == id)
            .ok_or(VaultError::ItemNotFound)?;

        Ok(())
    }

    async fn validate_cipher_deleted(&self, id: &str) -> Result<(), VaultError> {
        let storage = self.storage.lock().await;
        let vault_data: VaultData = storage
            .get("vaultData")
            .map_err(|e| VaultError::StorageError(e.to_string()))?
            .ok_or(VaultError::NotSynced)?;

        let cipher = vault_data
            .ciphers
            .iter()
            .find(|c| c.id == id)
            .ok_or(VaultError::ItemNotFound)?;

        if cipher.deleted_date.is_none() {
            return Err(VaultError::ItemNotDeleted);
        }

        Ok(())
    }

    async fn validate_folder_exists(&self, id: &str) -> Result<(), VaultError> {
        let storage = self.storage.lock().await;
        let vault_data: VaultData = storage
            .get("vaultData")
            .map_err(|e| VaultError::StorageError(e.to_string()))?
            .ok_or(VaultError::NotSynced)?;

        vault_data
            .folders
            .iter()
            .find(|f| f.id == id)
            .ok_or(VaultError::FolderNotFound)?;

        Ok(())
    }

    async fn get_cipher(&self, id: &str) -> Result<Cipher, VaultError> {
        let storage = self.storage.lock().await;
        let vault_data: VaultData = storage
            .get("vaultData")
            .map_err(|e| VaultError::StorageError(e.to_string()))?
            .ok_or(VaultError::NotSynced)?;

        vault_data
            .ciphers
            .iter()
            .find(|c| c.id == id)
            .cloned()
            .ok_or(VaultError::ItemNotFound)
    }
}
