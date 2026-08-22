//! Key service for managing user encryption keys
//!
//! This module provides functions for retrieving and managing the user key
//! for vault decryption operations.

use crate::models::state::KdfConfig;
use crate::services::storage::{
    AccountManager, JsonFileStorage, Storage, StorageKey, make_protected_key, parse_session_key,
    user_key_protected_storage_key,
};
use bitwarden_core::key_management::account_cryptographic_state::WrappedAccountCryptographicState;
use bitwarden_core::key_management::crypto::{InitUserCryptoMethod, InitUserCryptoRequest};
use bitwarden_core::{Client, UserId};
use bitwarden_crypto::{EncString, Kdf, SymmetricCryptoKey};
use std::str::FromStr;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::Mutex;
use tracing::debug;

/// Key service errors
#[derive(Debug, Error)]
pub enum KeyServiceError {
    #[error("No active user. Run 'bw login' first.")]
    NoActiveUser,

    #[error("User key not found. Run 'bw unlock' first.")]
    UserKeyNotFound,

    #[error("Invalid session key: {0}")]
    InvalidSessionKey(String),

    #[error("Failed to decrypt user key: {0}")]
    DecryptionFailed(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("KDF configuration not found. Run 'bw login' again.")]
    KdfConfigNotFound,

    #[error(
        "Account private key not found in storage. Run 'bw login' again to \
         re-fetch account keys."
    )]
    PrivateKeyNotFound,

    #[error("Failed to initialize SDK crypto: {0}")]
    CryptoInitFailed(String),
}

impl From<anyhow::Error> for KeyServiceError {
    fn from(e: anyhow::Error) -> Self {
        KeyServiceError::StorageError(e.to_string())
    }
}

/// Service for retrieving and managing user encryption keys
pub struct KeyService {
    storage: Arc<Mutex<JsonFileStorage>>,
    account_manager: Arc<AccountManager>,
}

impl KeyService {
    /// Create a new KeyService
    ///
    /// # Arguments
    /// * `storage` - The JSON file storage instance
    /// * `account_manager` - The account manager for user info
    pub fn new(storage: Arc<Mutex<JsonFileStorage>>, account_manager: Arc<AccountManager>) -> Self {
        Self {
            storage,
            account_manager,
        }
    }

    /// Get the user key from protected storage using the session key
    ///
    /// # Arguments
    /// * `session_str` - Base64-encoded session key (from BW_SESSION or --session)
    ///
    /// # Returns
    /// The user's symmetric key ready for vault decryption
    pub async fn get_user_key(
        &self,
        session_str: &str,
    ) -> Result<SymmetricCryptoKey, KeyServiceError> {
        // Parse the session key
        let session_key = parse_session_key(session_str)
            .map_err(|e| KeyServiceError::InvalidSessionKey(e.to_string()))?;

        // Get the active user ID
        let user_id = self
            .account_manager
            .get_active_user_id()
            .await?
            .ok_or(KeyServiceError::NoActiveUser)?;

        // Get the protected user key from storage
        let storage = self.storage.lock().await;
        let protected_key = make_protected_key(&user_key_protected_storage_key(&user_id));

        let encrypted_user_key: Option<String> = storage
            .get(&protected_key)
            .map_err(|e| KeyServiceError::StorageError(e.to_string()))?;

        drop(storage);

        let encrypted_user_key = encrypted_user_key.ok_or(KeyServiceError::UserKeyNotFound)?;

        // Decrypt the user key
        crate::services::storage::decrypt_user_key(&encrypted_user_key, &session_key)
            .map_err(|e| KeyServiceError::DecryptionFailed(e.to_string()))
    }

    /// Load the user key from protected storage into the SDK client's key store.
    ///
    /// The CLI derives and stores the user key itself, so the SDK client starts
    /// every process with an empty key store. Without this call,
    /// `client.vault()` encrypt/decrypt operations have no user key to work
    /// with and fail. Call this once per invocation, before any vault command
    /// touches ciphers.
    ///
    /// # Arguments
    /// * `client` - The SDK client to initialize
    /// * `session_str` - Base64-encoded session key (from BW_SESSION or --session)
    pub async fn initialize_client_crypto(
        &self,
        client: &Client,
        session_str: &str,
    ) -> Result<(), KeyServiceError> {
        let user_key = self.get_user_key(session_str).await?;

        let user_id = self
            .account_manager
            .get_active_user_id()
            .await?
            .ok_or(KeyServiceError::NoActiveUser)?;

        let account = self
            .account_manager
            .get_account(&user_id)
            .await?
            .ok_or(KeyServiceError::NoActiveUser)?;

        let storage = self.storage.lock().await;

        let kdf_config: KdfConfig = storage
            .get(&StorageKey::UserKdfConfig.format(Some(&user_id)))
            .map_err(|e| KeyServiceError::StorageError(e.to_string()))?
            .ok_or(KeyServiceError::KdfConfigNotFound)?;

        let private_key: String = storage
            .get(&StorageKey::UserPrivateKey.format(Some(&user_id)))
            .map_err(|e| KeyServiceError::StorageError(e.to_string()))?
            .ok_or(KeyServiceError::PrivateKeyNotFound)?;

        drop(storage);

        let kdf: Kdf = (&kdf_config)
            .try_into()
            .map_err(|e: anyhow::Error| KeyServiceError::CryptoInitFailed(e.to_string()))?;

        let private_key = EncString::from_str(&private_key).map_err(|e| {
            KeyServiceError::CryptoInitFailed(format!("Invalid account private key: {}", e))
        })?;

        debug!("Initializing SDK crypto from session key");

        client
            .crypto()
            .initialize_user_crypto(InitUserCryptoRequest {
                user_id: UserId::from_str(&user_id).ok(),
                kdf_params: kdf,
                email: account.email,
                account_cryptographic_state: WrappedAccountCryptographicState::V1 { private_key },
                method: InitUserCryptoMethod::DecryptedKey {
                    decrypted_user_key: user_key.to_base64().to_string(),
                },
                // V1 -> V2 account key rotation is not something the CLI drives.
                upgrade_token: None,
            })
            .await
            .map_err(|e| KeyServiceError::CryptoInitFailed(e.to_string()))
    }

    /// Store the user key in protected storage
    ///
    /// Called during login/unlock after decrypting user key with master key.
    ///
    /// # Arguments
    /// * `user_id` - The user's ID
    /// * `user_key` - The user's symmetric key to store
    /// * `session_key` - The session key to encrypt with
    pub async fn store_user_key(
        &self,
        user_id: &str,
        user_key: &SymmetricCryptoKey,
        session_key: &SymmetricCryptoKey,
    ) -> Result<(), KeyServiceError> {
        // Encrypt the user key with the session key
        let encrypted_user_key = crate::services::storage::encrypt_user_key(user_key, session_key)
            .map_err(|e| KeyServiceError::StorageError(e.to_string()))?;

        // Store in protected storage
        let protected_key = make_protected_key(&user_key_protected_storage_key(user_id));

        let mut storage = self.storage.lock().await;
        storage
            .set(&protected_key, &encrypted_user_key)
            .await
            .map_err(|e| KeyServiceError::StorageError(e.to_string()))?;
        storage
            .flush()
            .await
            .map_err(|e| KeyServiceError::StorageError(e.to_string()))?;

        Ok(())
    }

    /// Check if a user key is stored for the active user
    ///
    /// # Returns
    /// `true` if a protected user key exists, `false` otherwise
    pub async fn has_user_key(&self) -> Result<bool, KeyServiceError> {
        let user_id = match self.account_manager.get_active_user_id().await? {
            Some(id) => id,
            None => return Ok(false),
        };

        let storage = self.storage.lock().await;
        let protected_key = make_protected_key(&user_key_protected_storage_key(&user_id));

        let value: Option<String> = storage
            .get(&protected_key)
            .map_err(|e| KeyServiceError::StorageError(e.to_string()))?;

        Ok(value.is_some())
    }

    /// Clear the user key from protected storage
    ///
    /// Called during lock/logout operations.
    ///
    /// # Arguments
    /// * `user_id` - The user's ID
    pub async fn clear_user_key(&self, user_id: &str) -> Result<(), KeyServiceError> {
        let protected_key = make_protected_key(&user_key_protected_storage_key(user_id));

        let mut storage = self.storage.lock().await;
        storage
            .remove(&protected_key)
            .await
            .map_err(|e| KeyServiceError::StorageError(e.to_string()))?;
        storage
            .flush()
            .await
            .map_err(|e| KeyServiceError::StorageError(e.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::storage::generate_session_key;
    use tempfile::TempDir;

    async fn create_test_key_service() -> (KeyService, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage = Arc::new(Mutex::new(
            JsonFileStorage::new(Some(temp_dir.path().to_path_buf())).unwrap(),
        ));
        let account_manager = Arc::new(AccountManager::new(Arc::clone(&storage)));
        (KeyService::new(storage, account_manager), temp_dir)
    }

    #[tokio::test]
    async fn test_no_active_user() {
        let (service, _temp) = create_test_key_service().await;

        // Create a valid session key
        let session_key = generate_session_key();
        let session_str = crate::services::storage::format_session_key(&session_key);

        let result = service.get_user_key(&session_str).await;
        assert!(matches!(result, Err(KeyServiceError::NoActiveUser)));
    }

    #[tokio::test]
    async fn test_invalid_session_key() {
        let (service, _temp) = create_test_key_service().await;

        let result = service.get_user_key("invalid-session-key").await;
        assert!(matches!(result, Err(KeyServiceError::InvalidSessionKey(_))));
    }

    #[tokio::test]
    async fn test_store_and_retrieve_user_key() {
        let (service, _temp) = create_test_key_service().await;

        // Set up an active user
        service
            .account_manager
            .register_account("user-123", "test@example.com")
            .await
            .unwrap();
        service
            .account_manager
            .set_active_user_id("user-123")
            .await
            .unwrap();

        // Generate keys
        let session_key = generate_session_key();
        let user_key = generate_session_key(); // Using as user key

        // Store the user key
        service
            .store_user_key("user-123", &user_key, &session_key)
            .await
            .unwrap();

        // Verify has_user_key returns true
        let has_key = service.has_user_key().await.unwrap();
        assert!(has_key);

        // Retrieve the user key
        let session_str = crate::services::storage::format_session_key(&session_key);
        let retrieved_key = service.get_user_key(&session_str).await.unwrap();

        // Verify the keys match by comparing their formatted output
        assert_eq!(
            crate::services::storage::format_session_key(&user_key),
            crate::services::storage::format_session_key(&retrieved_key)
        );
    }

    #[tokio::test]
    async fn test_user_key_not_found() {
        let (service, _temp) = create_test_key_service().await;

        // Set up an active user but don't store a user key
        service
            .account_manager
            .register_account("user-123", "test@example.com")
            .await
            .unwrap();
        service
            .account_manager
            .set_active_user_id("user-123")
            .await
            .unwrap();

        // Create a valid session key
        let session_key = generate_session_key();
        let session_str = crate::services::storage::format_session_key(&session_key);

        let result = service.get_user_key(&session_str).await;
        assert!(matches!(result, Err(KeyServiceError::UserKeyNotFound)));
    }

    #[tokio::test]
    async fn test_clear_user_key() {
        let (service, _temp) = create_test_key_service().await;

        // Set up an active user
        service
            .account_manager
            .register_account("user-123", "test@example.com")
            .await
            .unwrap();
        service
            .account_manager
            .set_active_user_id("user-123")
            .await
            .unwrap();

        // Generate and store keys
        let session_key = generate_session_key();
        let user_key = generate_session_key();

        service
            .store_user_key("user-123", &user_key, &session_key)
            .await
            .unwrap();

        // Verify key exists
        assert!(service.has_user_key().await.unwrap());

        // Clear the key
        service.clear_user_key("user-123").await.unwrap();

        // Verify key no longer exists
        assert!(!service.has_user_key().await.unwrap());
    }
}
