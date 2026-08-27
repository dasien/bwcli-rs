//! Write service for vault CRUD operations
//!
//! Everything here goes through the SDK: either its high-level
//! [`bitwarden_vault::CiphersClient`], or — where that client's request types
//! are unreachable — its generated `bitwarden-api-api` clients.
//!
//! ## Why this is split
//!
//! `CiphersClient::create`/`edit` and `FoldersClient::create`/`edit` take
//! request types (`CipherCreateRequest`, `CipherEditRequest`,
//! `FolderAddEditRequest`) that `bitwarden-vault` does not export — the
//! `cipher_client` module is `pub(crate)`. Nothing outside the crate calls them,
//! including the wasm bindings, uniffi and `crates/bw`. `FoldersClient` has no
//! `delete` at all. So:
//!
//! - **`CiphersClient`**: delete / soft-delete / restore / move. These take only
//!   ids and update the state repository themselves.
//! - **Generated `CiphersApi`/`FoldersApi`**: cipher create/edit and all folder
//!   writes, each followed by an explicit repository update so reads stay
//!   consistent. These are the same clients the SDK's own `CiphersClient` calls,
//!   so they share its authentication, token refresh and retry behaviour.
//!
//! Revisit the second half if the SDK exports those request types; the
//! difference would then be the repository bookkeeping, not the transport.

use super::{CipherService, ConfirmationService, ValidationService, VaultError};
use crate::models::vault::{CipherRequestModel, FolderRequestModel};
use bitwarden_api_api::models::{
    CipherDetailsResponseModel, CipherResponseModel, FolderResponseModel,
};
use bitwarden_collections::collection::CollectionId;
use bitwarden_core::client::persisted_state::OrganizationSharedKey;
use bitwarden_core::{Client, OrganizationId};
use bitwarden_state::repository::Repository;
use bitwarden_vault::{
    Cipher, CipherId, CipherView, Folder, FolderId, FolderView, VaultClientExt,
};
use std::sync::Arc;

/// Service for vault write operations (create, update, delete)
pub struct WriteService {
    sdk: Arc<Client>,
    cipher_service: Arc<CipherService>,
    validation_service: Arc<ValidationService>,
    confirmation_service: Arc<ConfirmationService>,
}

impl WriteService {
    pub fn new(
        sdk: Arc<Client>,
        cipher_service: Arc<CipherService>,
        validation_service: Arc<ValidationService>,
        confirmation_service: Arc<ConfirmationService>,
    ) -> Self {
        Self {
            sdk,
            cipher_service,
            validation_service,
            confirmation_service,
        }
    }

    /// The SDK's generated API clients, authenticated by its token handler.
    fn api(&self) -> Arc<bitwarden_core::client::ApiConfigurations> {
        self.sdk.internal.get_api_configurations()
    }

    /// Write a cipher into the SDK's state repository.
    ///
    /// `CiphersClient` does this for us; the generated clients are a layer below
    /// it and cannot, and reads come from the repository, so without this a
    /// create or edit would not show up until the next sync.
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

        let response = self
            .api()
            .api_client
            .ciphers_api()
            .post(Some(request))
            .await
            .map_err(|e| VaultError::ApiError(e.to_string()))?;

        let created = cipher_from_response(response, None)?;
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

        // `CipherResponseModel` carries no collection ids, so preserve the ones
        // we sent. Dropping them would unshare an organization item on every
        // edit.
        let collection_ids = cipher_view.collection_ids.clone();

        let encryption_context = self.cipher_service.encrypt_cipher(cipher_view).await?;
        let request: CipherRequestModel = encryption_context.into();

        let response = self
            .api()
            .api_client
            .ciphers_api()
            .put(cipher_id.into(), Some(request))
            .await
            .map_err(|e| VaultError::ApiError(e.to_string()))?;

        let updated = cipher_from_response(response, Some(collection_ids))?;
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
    pub async fn move_cipher_to_folder(
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

    /// Share a cipher into an organization, i.e. the TypeScript CLI's `bw move`.
    ///
    /// This is a re-encryption, not a metadata change: the item is decrypted
    /// under the user key and re-encrypted under the organization's key, so the
    /// organization key has to be in the key store. `sync` puts it there.
    ///
    /// `CiphersClient::share_cipher` does the whole job — reassign, refresh
    /// password history, re-encrypt, `PUT`, update the repository.
    pub async fn share_cipher(
        &self,
        cipher_id: &str,
        organization_id: &str,
        collection_ids: Vec<String>,
    ) -> Result<CipherView, VaultError> {
        let cipher_id = Self::parse_cipher_id(cipher_id)?;
        let organization_id: OrganizationId = organization_id.parse().map_err(|_| {
            VaultError::InvalidInput(format!(
                "'{organization_id}' is not a valid organization id"
            ))
        })?;

        // The server requires at least one collection: an organization item with
        // no collection would be invisible to everyone, including its owner.
        if collection_ids.is_empty() {
            return Err(VaultError::InvalidInput(
                "at least one collection id is required to share an item".to_string(),
            ));
        }

        let collection_ids = collection_ids
            .iter()
            .map(|id| {
                id.parse::<CollectionId>().map_err(|_| {
                    VaultError::InvalidInput(format!("'{id}' is not a valid collection id"))
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        // Sharing re-encrypts under the organization's key, so check we have it
        // before doing any work. `sync` loads these best-effort, so a missing one
        // is a plausible state with an actionable remedy rather than a bug.
        let has_org_key = self
            .sdk
            .platform()
            .state()
            .get::<OrganizationSharedKey>()
            .map_err(|e| VaultError::StorageError(e.to_string()))?
            .get(organization_id)
            .await
            .map_err(|e| VaultError::StorageError(e.to_string()))?
            .is_some();

        if !has_org_key {
            return Err(VaultError::InvalidInput(format!(
                "no key for organization {organization_id}. Run 'bw sync' with the \
                 vault unlocked; if you are not a confirmed member of that \
                 organization, sharing into it is not possible."
            )));
        }

        let ciphers = self.sdk.vault().ciphers();

        let cipher_view = ciphers
            .get(&cipher_id.to_string())
            .await
            .map_err(|_| VaultError::ItemNotFound)?;

        if cipher_view.organization_id.is_some() {
            return Err(VaultError::InvalidInput(
                "this item already belongs to an organization".to_string(),
            ));
        }

        // The original is passed so the SDK can carry password history across the
        // re-encryption rather than losing it.
        ciphers
            .share_cipher(
                cipher_view.clone(),
                organization_id,
                collection_ids,
                Some(cipher_view),
            )
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

        let response = self
            .api()
            .api_client
            .folders_api()
            .post(Some(request))
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

        let response = self
            .api()
            .api_client
            .folders_api()
            .put(id, Some(request))
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

        self.api()
            .api_client
            .folders_api()
            .delete(id)
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
///
/// `Cipher` only has a `TryFrom` for `CipherDetailsResponseModel`, and the write
/// endpoints answer with `CipherResponseModel` — the same type minus
/// `collectionIds`. The SDK bridges the two with a `pub(crate)` trait
/// (`PartialCipher::merge_with_cipher`), so widen it here instead. Every other
/// field is identical, and the compiler will flag this if that stops being true.
fn cipher_from_response(
    response: CipherResponseModel,
    collection_ids: Option<Vec<bitwarden_collections::collection::CollectionId>>,
) -> Result<Cipher, VaultError> {
    let CipherResponseModel {
        object,
        id,
        organization_id,
        r#type,
        data,
        partial_data,
        name,
        notes,
        login,
        card,
        identity,
        secure_note,
        ssh_key,
        bank_account,
        drivers_license,
        passport,
        fields,
        password_history,
        attachments,
        organization_use_totp,
        revision_date,
        creation_date,
        deleted_date,
        reprompt,
        key,
        folder_id,
        favorite,
        edit,
        view_password,
        archived_date,
        permissions,
    } = response;

    let details = CipherDetailsResponseModel {
        object,
        id,
        organization_id,
        r#type,
        data,
        partial_data,
        name,
        notes,
        login,
        card,
        identity,
        secure_note,
        ssh_key,
        bank_account,
        drivers_license,
        passport,
        fields,
        password_history,
        attachments,
        organization_use_totp,
        revision_date,
        creation_date,
        deleted_date,
        reprompt,
        key,
        folder_id,
        favorite,
        edit,
        view_password,
        archived_date,
        permissions,
        collection_ids: collection_ids
            .map(|ids| ids.into_iter().map(Into::into).collect()),
    };

    Cipher::try_from(details).map_err(|e| {
        VaultError::ApiError(format!("could not read the cipher the server returned: {e}"))
    })
}

/// Convert a folder write response into a domain `Folder`.
fn folder_from_response(response: FolderResponseModel) -> Result<Folder, VaultError> {
    Folder::try_from(response).map_err(|e| {
        VaultError::ApiError(format!("could not read the folder the server returned: {e}"))
    })
}
