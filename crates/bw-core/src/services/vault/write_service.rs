//! Write service for vault CRUD operations
//!
//! Uses the SDK's `CiphersClient` where its API is actually reachable, and
//! hand-rolled HTTP where it is not.
//!
//! ## Why this is split
//!
//! `CiphersClient::create`/`edit` and `FoldersClient::create`/`edit` take
//! request types (`CipherCreateRequest`, `CipherEditRequest`,
//! `FolderAddEditRequest`) that `bitwarden-vault` does not export — the
//! `cipher_client` module is `pub(crate)`. Nothing outside the crate calls them,
//! including the wasm bindings, uniffi and upstream `bw`. `FoldersClient` has no
//! `delete` at all. So:
//!
//! - **SDK**: cipher delete / soft-delete / restore / move (these take only
//!   `CipherId`/`FolderId`, and update the state repository themselves).
//! - **Hand-rolled**: cipher create/edit and all folder writes, each followed by
//!   an explicit repository update so reads stay consistent.
//!
//! Revisit the hand-rolled half if the SDK exports those request types.

use super::{CipherService, ConfirmationService, ValidationService, VaultError};
use crate::models::vault::{CipherRequestModel, FolderRequestModel};
use crate::services::api::{ApiClient, BitwardenApiClient, endpoints};
use bitwarden_api_api::models::{CipherDetailsResponseModel, FolderResponseModel};
use bitwarden_core::Client;
use bitwarden_state::repository::Repository;
use bitwarden_vault::{
    Cipher, CipherId, CipherView, Folder, FolderId, FolderView, VaultClientExt,
};
use std::sync::Arc;

/// Service for vault write operations (create, update, delete)
pub struct WriteService {
    sdk: Arc<Client>,
    api_client: Arc<BitwardenApiClient>,
    cipher_service: Arc<CipherService>,
    validation_service: Arc<ValidationService>,
    confirmation_service: Arc<ConfirmationService>,
}

impl WriteService {
    pub fn new(
        sdk: Arc<Client>,
        api_client: Arc<BitwardenApiClient>,
        cipher_service: Arc<CipherService>,
        validation_service: Arc<ValidationService>,
        confirmation_service: Arc<ConfirmationService>,
    ) -> Self {
        Self {
            sdk,
            api_client,
            cipher_service,
            validation_service,
            confirmation_service,
        }
    }

    /// Write a cipher into the SDK's state repository.
    ///
    /// The SDK's own write methods do this for us; the hand-rolled ones cannot,
    /// and reads come from the repository, so without this a create or edit
    /// would not show up until the next sync.
    async fn store_cipher(&self, cipher: Cipher) -> Result<(), VaultError> {
        let Some(id) = cipher.id else {
            return Ok(());
        };

        self.sdk
            .platform()
            .state()
            .get::<Cipher>()
            .map_err(|e| VaultError::StorageError(e.to_string()))?
            .set(id, cipher)
            .await
            .map_err(|e| VaultError::StorageError(e.to_string()))
    }

    /// Write a folder into the SDK's state repository. See [`Self::store_cipher`].
    async fn store_folder(&self, folder: Folder) -> Result<(), VaultError> {
        let Some(id) = folder.id else {
            return Ok(());
        };

        self.sdk
            .platform()
            .state()
            .get::<Folder>()
            .map_err(|e| VaultError::StorageError(e.to_string()))?
            .set(id, folder)
            .await
            .map_err(|e| VaultError::StorageError(e.to_string()))
    }

    fn parse_cipher_id(id: &str) -> Result<CipherId, VaultError> {
        id.parse()
            .map_err(|_| VaultError::InvalidInput(format!("'{id}' is not a valid item id")))
    }

    fn parse_folder_id(id: &str) -> Result<FolderId, VaultError> {
        id.parse()
            .map_err(|_| VaultError::InvalidInput(format!("'{id}' is not a valid folder id")))
    }

    // ========== Cipher Operations ==========

    /// Create new cipher (item)
    pub async fn create_cipher(
        &self,
        cipher_view: CipherView,
        _session: &str,
    ) -> Result<CipherView, VaultError> {
        self.validation_service
            .validate_cipher_create(&cipher_view)?;

        let encryption_context = self.cipher_service.encrypt_cipher(cipher_view).await?;
        let request: CipherRequestModel = encryption_context.into();

        // Deserialize into the tolerant generated model, not `Cipher` — that type
        // is `deny_unknown_fields` and rejects the server's `object` field, which
        // made writes report failure after succeeding.
        let response: CipherDetailsResponseModel = self
            .api_client
            .post_with_auth(endpoints::api::ciphers::BASE, &request)
            .await
            .map_err(|e| VaultError::ApiError(e.to_string()))?;

        let created = cipher_from_response(response)?;
        self.store_cipher(created.clone()).await?;

        self.cipher_service.decrypt_cipher(created).await
    }

    /// Update existing cipher
    ///
    /// `revision_date` is deliberately left as-is: the SDK derives
    /// `lastKnownRevisionDate` from it for optimistic concurrency, and the
    /// server rejects the write if it does not match.
    pub async fn update_cipher(
        &self,
        id: &str,
        mut cipher_view: CipherView,
        _session: &str,
    ) -> Result<CipherView, VaultError> {
        let cipher_id = Self::parse_cipher_id(id)?;
        cipher_view.id = Some(cipher_id);

        self.validation_service
            .validate_cipher_update(&cipher_view)?;

        let encryption_context = self.cipher_service.encrypt_cipher(cipher_view).await?;
        let request: CipherRequestModel = encryption_context.into();

        let response: CipherDetailsResponseModel = self
            .api_client
            .put_with_auth(&endpoints::api::ciphers::by_id(id), &request)
            .await
            .map_err(|e| VaultError::ApiError(e.to_string()))?;

        let updated = cipher_from_response(response)?;
        self.store_cipher(updated.clone()).await?;

        self.cipher_service.decrypt_cipher(updated).await
    }

    /// Delete cipher (soft or permanent)
    pub async fn delete_cipher(
        &self,
        id: &str,
        permanent: bool,
        no_interaction: bool,
    ) -> Result<(), VaultError> {
        let cipher_id = Self::parse_cipher_id(id)?;

        if permanent && !no_interaction && !self.confirmation_service.confirm_permanent_delete()? {
            return Err(VaultError::OperationCancelled);
        }

        let ciphers = self.sdk.vault().ciphers();

        if permanent {
            ciphers.delete(cipher_id).await
        } else {
            ciphers.soft_delete(cipher_id).await
        }
        .map_err(|e| VaultError::ApiError(e.to_string()))
    }

    /// Restore cipher from trash
    pub async fn restore_cipher(&self, id: &str) -> Result<CipherView, VaultError> {
        let cipher_id = Self::parse_cipher_id(id)?;

        self.sdk
            .vault()
            .ciphers()
            .restore(cipher_id)
            .await
            .map_err(|e| VaultError::ApiError(e.to_string()))
    }

    /// Move cipher to a different folder (or out of all folders)
    pub async fn move_cipher(
        &self,
        cipher_id: &str,
        folder_id: Option<&str>,
        _session: &str,
    ) -> Result<(), VaultError> {
        let cipher_id = Self::parse_cipher_id(cipher_id)?;
        let folder_id = folder_id.map(Self::parse_folder_id).transpose()?;

        // Bulk endpoint: no decrypt/re-encrypt round trip, unlike the old
        // implementation.
        self.sdk
            .vault()
            .ciphers()
            .move_many(vec![cipher_id], folder_id)
            .await
            .map_err(|e| VaultError::ApiError(e.to_string()))
    }

    // ========== Folder Operations ==========

    /// Create folder
    pub async fn create_folder(
        &self,
        name: String,
        _session: &str,
    ) -> Result<FolderView, VaultError> {
        self.validation_service.validate_folder_name(&name)?;

        let encrypted = self.cipher_service.encrypt_folder(FolderView {
            id: None,
            name,
            revision_date: chrono::Utc::now(),
        })?;
        let request = FolderRequestModel {
            name: encrypted.name.to_string(),
        };

        let response: FolderResponseModel = self
            .api_client
            .post_with_auth(endpoints::api::folders::BASE, &request)
            .await
            .map_err(|e| VaultError::ApiError(e.to_string()))?;

        let created = folder_from_response(response)?;
        self.store_folder(created.clone()).await?;

        self.cipher_service
            .decrypt_folders(vec![created])?
            .pop()
            .ok_or(VaultError::FolderNotFound)
    }

    /// Update folder name
    pub async fn update_folder(
        &self,
        id: &str,
        name: String,
        _session: &str,
    ) -> Result<FolderView, VaultError> {
        self.validation_service.validate_folder_name(&name)?;
        let folder_id = Self::parse_folder_id(id)?;

        let encrypted = self.cipher_service.encrypt_folder(FolderView {
            id: Some(folder_id),
            name,
            revision_date: chrono::Utc::now(),
        })?;
        let request = FolderRequestModel {
            name: encrypted.name.to_string(),
        };

        let response: FolderResponseModel = self
            .api_client
            .put_with_auth(&endpoints::api::folders::by_id(id), &request)
            .await
            .map_err(|e| VaultError::ApiError(e.to_string()))?;

        let updated = folder_from_response(response)?;
        self.store_folder(updated.clone()).await?;

        self.cipher_service
            .decrypt_folders(vec![updated])?
            .pop()
            .ok_or(VaultError::FolderNotFound)
    }

    /// Delete folder
    pub async fn delete_folder(&self, id: &str) -> Result<(), VaultError> {
        // `FoldersClient` has no delete method at all.
        let folder_id = Self::parse_folder_id(id)?;

        self.api_client
            .delete_with_auth(&endpoints::api::folders::by_id(id))
            .await
            .map_err(|e| VaultError::ApiError(e.to_string()))?;

        self.sdk
            .platform()
            .state()
            .get::<Folder>()
            .map_err(|e| VaultError::StorageError(e.to_string()))?
            .remove(folder_id)
            .await
            .map_err(|e| VaultError::StorageError(e.to_string()))
    }

}

/// Convert a cipher write response into a domain `Cipher`.
fn cipher_from_response(response: CipherDetailsResponseModel) -> Result<Cipher, VaultError> {
    Cipher::try_from(response).map_err(|e| {
        VaultError::ApiError(format!("could not read the cipher the server returned: {e}"))
    })
}

/// Convert a folder write response into a domain `Folder`.
fn folder_from_response(response: FolderResponseModel) -> Result<Folder, VaultError> {
    Folder::try_from(response).map_err(|e| {
        VaultError::ApiError(format!("could not read the folder the server returned: {e}"))
    })
}
