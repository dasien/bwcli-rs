//! Cipher encryption/decryption service
//!
//! Thin wrapper around SDK's VaultClient for encrypting and decrypting vault items.

use super::errors::VaultError;
use bitwarden_collections::collection::{Collection, CollectionView};
use bitwarden_core::Client;
use bitwarden_vault::{
    Cipher, CipherListView, CipherView, EncryptionContext, Folder, FolderView, VaultClientExt,
};
use std::sync::Arc;

/// Service for cipher decryption operations
///
/// Delegates to SDK's VaultClient for all crypto operations.
pub struct CipherService {
    sdk_client: Arc<Client>,
}

impl CipherService {
    pub fn new(sdk_client: Arc<Client>) -> Self {
        Self { sdk_client }
    }

    /// Decrypt a single cipher using the SDK
    pub fn decrypt_cipher(&self, cipher: Cipher) -> Result<CipherView, VaultError> {
        self.sdk_client
            .vault()
            .ciphers()
            .decrypt(cipher)
            .map_err(|e| VaultError::DecryptionError(e.to_string()))
    }

    /// Decrypt multiple ciphers using the SDK
    /// Returns list views for efficiency
    pub fn decrypt_ciphers(&self, ciphers: Vec<Cipher>) -> Result<Vec<CipherListView>, VaultError> {
        self.sdk_client
            .vault()
            .ciphers()
            .decrypt_list(ciphers)
            .map_err(|e| VaultError::DecryptionError(e.to_string()))
    }

    /// Decrypt folders using the SDK
    pub fn decrypt_folders(&self, folders: Vec<Folder>) -> Result<Vec<FolderView>, VaultError> {
        self.sdk_client
            .vault()
            .folders()
            .decrypt_list(folders)
            .map_err(|e| VaultError::DecryptionError(e.to_string()))
    }

    /// Decrypt collections using the SDK
    pub fn decrypt_collections(
        &self,
        collections: Vec<Collection>,
    ) -> Result<Vec<CollectionView>, VaultError> {
        self.sdk_client
            .vault()
            .collections()
            .decrypt_list(collections)
            .map_err(|e| VaultError::DecryptionError(e.to_string()))
    }

    /// Encrypt a cipher view for API submission
    pub fn encrypt_cipher(&self, cipher_view: CipherView) -> Result<EncryptionContext, VaultError> {
        self.sdk_client
            .vault()
            .ciphers()
            .encrypt(cipher_view)
            .map_err(|e| VaultError::EncryptionError(e.to_string()))
    }

    /// Encrypt a folder view for API submission
    pub fn encrypt_folder(&self, folder_view: FolderView) -> Result<Folder, VaultError> {
        self.sdk_client
            .vault()
            .folders()
            .encrypt(folder_view)
            .map_err(|e| VaultError::EncryptionError(e.to_string()))
    }
}
