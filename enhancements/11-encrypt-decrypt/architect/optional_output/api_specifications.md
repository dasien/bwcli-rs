# API Specifications: Vault Encryption/Decryption

## 1. Protected Storage Module API

### Module: `crates/bw-core/src/services/storage/protected_storage.rs`

```rust
//! Protected storage encryption/decryption using SDK crypto primitives
//!
//! This module provides encryption/decryption for sensitive data stored locally.
//! Uses the session key (from BW_SESSION) to encrypt/decrypt data with the
//! EncArrayBuffer binary format for TypeScript CLI compatibility.

use bitwarden_crypto::{
    BitwardenLegacyKeyBytes, EncString, KeyDecryptable, KeyEncryptable,
    OctetStreamBytes, SymmetricCryptoKey,
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use thiserror::Error;

/// Prefix for protected storage keys (TypeScript CLI compatible)
pub const PROTECTED_PREFIX: &str = "__PROTECTED__";

/// Error types for protected storage operations
#[derive(Debug, Error)]
pub enum ProtectedStorageError {
    #[error("Invalid session key: {0}")]
    InvalidSessionKey(String),

    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),

    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),

    #[error("Invalid base64 encoding: {0}")]
    InvalidBase64(String),

    #[error("Invalid key format: {0}")]
    InvalidKeyFormat(String),

    #[error("Crypto error: {0}")]
    CryptoError(String),
}

/// Create a protected storage key with the standard prefix
///
/// # Arguments
/// * `key` - The base key name (without prefix)
///
/// # Returns
/// Key name with __PROTECTED__ prefix
///
/// # Example
/// ```ignore
/// let key = make_protected_key("abc123_user_auto");
/// assert_eq!(key, "__PROTECTED__abc123_user_auto");
/// ```
pub fn make_protected_key(key: &str) -> String {
    format!("{}{}", PROTECTED_PREFIX, key)
}

/// Generate the protected storage key for a user's key
///
/// # Arguments
/// * `user_id` - The user's UUID
///
/// # Returns
/// Key suffix for user key storage (without __PROTECTED__ prefix)
///
/// # Example
/// ```ignore
/// let key = user_key_protected_storage_key("abc-123-def");
/// assert_eq!(key, "abc-123-def_user_auto");
/// ```
pub fn user_key_protected_storage_key(user_id: &str) -> String {
    format!("{}_user_auto", user_id)
}

/// Parse a session key from base64 string (BW_SESSION format)
///
/// # Arguments
/// * `session_str` - Base64-encoded 64-byte session key
///
/// # Returns
/// SymmetricCryptoKey ready for encryption/decryption
///
/// # Errors
/// - InvalidBase64 if decoding fails
/// - InvalidKeyFormat if wrong length or invalid key bytes
pub fn parse_session_key(session_str: &str) -> Result<SymmetricCryptoKey, ProtectedStorageError> {
    let key_bytes = STANDARD.decode(session_str)
        .map_err(|e| ProtectedStorageError::InvalidBase64(e.to_string()))?;

    let legacy_bytes = BitwardenLegacyKeyBytes::from(key_bytes);
    SymmetricCryptoKey::try_from(&legacy_bytes)
        .map_err(|e| ProtectedStorageError::InvalidKeyFormat(e.to_string()))
}

/// Format a session key as base64 for BW_SESSION export
///
/// # Arguments
/// * `key` - The session key to format
///
/// # Returns
/// Base64-encoded 64-byte key string
pub fn format_session_key(key: &SymmetricCryptoKey) -> String {
    let encoded = key.to_encoded();
    STANDARD.encode(encoded.as_ref())
}

/// Generate a new random session key
///
/// Uses SDK's cryptographically secure key generation.
///
/// # Returns
/// New 64-byte AES-256-CBC-HMAC session key
pub fn generate_session_key() -> SymmetricCryptoKey {
    SymmetricCryptoKey::make_aes256_cbc_hmac_key()
}

/// Encrypt arbitrary bytes for protected storage
///
/// Uses EncArrayBuffer binary format (not string format) for TypeScript compatibility.
///
/// # Arguments
/// * `plain` - Plaintext bytes to encrypt
/// * `key` - Session key for encryption
///
/// # Returns
/// Base64-encoded EncArrayBuffer
///
/// # Errors
/// - EncryptionFailed if SDK encryption fails
pub fn encrypt_protected_bytes(
    plain: &[u8],
    key: &SymmetricCryptoKey,
) -> Result<String, ProtectedStorageError> {
    // Wrap bytes in OctetStreamBytes for SDK encryption
    let enc_string: EncString = OctetStreamBytes::from(plain.to_vec())
        .encrypt_with_key(key)
        .map_err(|e| ProtectedStorageError::EncryptionFailed(e.to_string()))?;

    // Convert to binary format (EncArrayBuffer)
    let buffer = enc_string.to_buffer()
        .map_err(|e| ProtectedStorageError::EncryptionFailed(e.to_string()))?;

    // Base64 encode for storage
    Ok(STANDARD.encode(&buffer))
}

/// Decrypt bytes from protected storage
///
/// Expects base64-encoded EncArrayBuffer binary format.
///
/// # Arguments
/// * `encrypted_b64` - Base64-encoded encrypted data
/// * `key` - Session key for decryption
///
/// # Returns
/// Decrypted plaintext bytes
///
/// # Errors
/// - InvalidBase64 if decoding fails
/// - DecryptionFailed if SDK decryption fails
pub fn decrypt_protected_bytes(
    encrypted_b64: &str,
    key: &SymmetricCryptoKey,
) -> Result<Vec<u8>, ProtectedStorageError> {
    // Decode base64
    let buffer = STANDARD.decode(encrypted_b64)
        .map_err(|e| ProtectedStorageError::InvalidBase64(e.to_string()))?;

    // Parse binary format
    let enc_string = EncString::from_buffer(&buffer)
        .map_err(|e| ProtectedStorageError::DecryptionFailed(e.to_string()))?;

    // Decrypt
    enc_string.decrypt_with_key(key)
        .map_err(|e| ProtectedStorageError::DecryptionFailed(e.to_string()))
}

/// Encrypt a user key for protected storage
///
/// # Arguments
/// * `user_key` - The user's symmetric key to protect
/// * `session_key` - Session key for encryption
///
/// # Returns
/// Base64-encoded encrypted user key
pub fn encrypt_user_key(
    user_key: &SymmetricCryptoKey,
    session_key: &SymmetricCryptoKey,
) -> Result<String, ProtectedStorageError> {
    let encoded = user_key.to_encoded();
    encrypt_protected_bytes(encoded.as_ref(), session_key)
}

/// Decrypt a user key from protected storage
///
/// # Arguments
/// * `encrypted_b64` - Base64-encoded encrypted user key
/// * `session_key` - Session key for decryption
///
/// # Returns
/// Decrypted user symmetric key
pub fn decrypt_user_key(
    encrypted_b64: &str,
    session_key: &SymmetricCryptoKey,
) -> Result<SymmetricCryptoKey, ProtectedStorageError> {
    let bytes = decrypt_protected_bytes(encrypted_b64, session_key)?;
    let legacy_bytes = BitwardenLegacyKeyBytes::from(bytes);
    SymmetricCryptoKey::try_from(&legacy_bytes)
        .map_err(|e| ProtectedStorageError::InvalidKeyFormat(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_make_protected_key() {
        assert_eq!(
            make_protected_key("abc123_user_auto"),
            "__PROTECTED__abc123_user_auto"
        );
    }

    #[test]
    fn test_user_key_protected_storage_key() {
        assert_eq!(
            user_key_protected_storage_key("abc-123-def"),
            "abc-123-def_user_auto"
        );
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = generate_session_key();
        let plaintext = b"test data for encryption";

        let encrypted = encrypt_protected_bytes(plaintext, &key).unwrap();
        let decrypted = decrypt_protected_bytes(&encrypted, &key).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_user_key_roundtrip() {
        let session_key = generate_session_key();
        let user_key = generate_session_key(); // Use another key as user key

        let encrypted = encrypt_user_key(&user_key, &session_key).unwrap();
        let decrypted = decrypt_user_key(&encrypted, &session_key).unwrap();

        // Compare by encoding both keys
        assert_eq!(
            format_session_key(&user_key),
            format_session_key(&decrypted)
        );
    }

    #[test]
    fn test_session_key_roundtrip() {
        let key = generate_session_key();
        let formatted = format_session_key(&key);
        let parsed = parse_session_key(&formatted).unwrap();

        assert_eq!(format_session_key(&key), format_session_key(&parsed));
    }

    #[test]
    fn test_invalid_session_key() {
        let result = parse_session_key("not-valid-base64!");
        assert!(result.is_err());
    }

    #[test]
    fn test_wrong_key_fails_decryption() {
        let key1 = generate_session_key();
        let key2 = generate_session_key();
        let plaintext = b"secret data";

        let encrypted = encrypt_protected_bytes(plaintext, &key1).unwrap();
        let result = decrypt_protected_bytes(&encrypted, &key2);

        assert!(result.is_err());
    }
}
```

---

## 2. Key Service API

### Module: `crates/bw-core/src/services/key_service.rs`

```rust
//! Key service for user key management
//!
//! Provides retrieval and storage of user keys from/to protected storage.

use crate::services::storage::{
    AccountManager, JsonFileStorage, Storage,
    protected_storage::{
        self, ProtectedStorageError, make_protected_key, user_key_protected_storage_key,
    },
};
use bitwarden_crypto::SymmetricCryptoKey;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::Mutex;

/// Error types for key service operations
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

    #[error("Failed to encrypt user key: {0}")]
    EncryptionFailed(String),

    #[error("Storage error: {0}")]
    StorageError(String),
}

impl From<ProtectedStorageError> for KeyServiceError {
    fn from(err: ProtectedStorageError) -> Self {
        match err {
            ProtectedStorageError::InvalidSessionKey(msg) => KeyServiceError::InvalidSessionKey(msg),
            ProtectedStorageError::DecryptionFailed(msg) => KeyServiceError::DecryptionFailed(msg),
            ProtectedStorageError::EncryptionFailed(msg) => KeyServiceError::EncryptionFailed(msg),
            _ => KeyServiceError::DecryptionFailed(err.to_string()),
        }
    }
}

/// Service for managing user encryption keys
pub struct KeyService {
    storage: Arc<Mutex<JsonFileStorage>>,
    account_manager: Arc<AccountManager>,
}

impl KeyService {
    /// Create a new key service
    ///
    /// # Arguments
    /// * `storage` - Storage service for reading/writing protected keys
    /// * `account_manager` - Account manager for getting active user
    pub fn new(
        storage: Arc<Mutex<JsonFileStorage>>,
        account_manager: Arc<AccountManager>,
    ) -> Self {
        Self {
            storage,
            account_manager,
        }
    }

    /// Get user key from protected storage using session key
    ///
    /// # Arguments
    /// * `session_str` - Base64-encoded session key (from BW_SESSION or --session)
    ///
    /// # Returns
    /// User symmetric key ready for vault decryption
    ///
    /// # Errors
    /// - NoActiveUser if not logged in
    /// - InvalidSessionKey if session format is wrong
    /// - UserKeyNotFound if protected key not stored
    /// - DecryptionFailed if decryption fails
    pub async fn get_user_key(
        &self,
        session_str: &str,
    ) -> Result<SymmetricCryptoKey, KeyServiceError> {
        // Parse session key
        let session_key = protected_storage::parse_session_key(session_str)
            .map_err(|e| KeyServiceError::InvalidSessionKey(e.to_string()))?;

        // Get active user ID
        let user_id = self.account_manager
            .get_active_user_id()
            .await
            .map_err(|e| KeyServiceError::StorageError(e.to_string()))?
            .ok_or(KeyServiceError::NoActiveUser)?;

        // Build protected storage key
        let storage_key = make_protected_key(&user_key_protected_storage_key(&user_id));

        // Read encrypted user key from storage
        let storage = self.storage.lock().await;
        let encrypted_user_key: Option<String> = storage
            .get(&storage_key)
            .map_err(|e| KeyServiceError::StorageError(e.to_string()))?;
        drop(storage);

        let encrypted_user_key = encrypted_user_key
            .ok_or(KeyServiceError::UserKeyNotFound)?;

        // Decrypt and return user key
        protected_storage::decrypt_user_key(&encrypted_user_key, &session_key)
            .map_err(KeyServiceError::from)
    }

    /// Store user key in protected storage
    ///
    /// Called during login/unlock after decrypting user key with master key.
    ///
    /// # Arguments
    /// * `user_id` - User's UUID
    /// * `user_key` - User's symmetric key to store
    /// * `session_key` - Session key to encrypt with
    ///
    /// # Errors
    /// - EncryptionFailed if encryption fails
    /// - StorageError if storage write fails
    pub async fn store_user_key(
        &self,
        user_id: &str,
        user_key: &SymmetricCryptoKey,
        session_key: &SymmetricCryptoKey,
    ) -> Result<(), KeyServiceError> {
        // Encrypt user key with session key
        let encrypted_user_key = protected_storage::encrypt_user_key(user_key, session_key)
            .map_err(|e| KeyServiceError::EncryptionFailed(e.to_string()))?;

        // Build protected storage key
        let storage_key = make_protected_key(&user_key_protected_storage_key(user_id));

        // Store encrypted user key
        let mut storage = self.storage.lock().await;
        storage
            .set(&storage_key, &encrypted_user_key)
            .await
            .map_err(|e| KeyServiceError::StorageError(e.to_string()))?;
        storage
            .flush()
            .await
            .map_err(|e| KeyServiceError::StorageError(e.to_string()))?;

        Ok(())
    }

    /// Check if user key is stored for the active user
    ///
    /// # Returns
    /// true if protected key exists for active user
    pub async fn has_user_key(&self) -> Result<bool, KeyServiceError> {
        let user_id = match self.account_manager.get_active_user_id().await {
            Ok(Some(id)) => id,
            _ => return Ok(false),
        };

        let storage_key = make_protected_key(&user_key_protected_storage_key(&user_id));
        let storage = self.storage.lock().await;
        let value: Option<String> = storage
            .get(&storage_key)
            .map_err(|e| KeyServiceError::StorageError(e.to_string()))?;

        Ok(value.is_some())
    }
}
```

---

## 3. Updated CipherService Decryption API

### Module: `crates/bw-core/src/services/vault/cipher_service.rs`

Key method signatures with user_key parameter:

```rust
impl CipherService {
    /// Decrypt a single cipher
    ///
    /// # Arguments
    /// * `cipher` - Encrypted cipher from storage
    /// * `user_key` - User's symmetric key for decryption
    ///
    /// # Returns
    /// Decrypted cipher view
    pub async fn decrypt_cipher(
        &self,
        cipher: &Cipher,
        user_key: &SymmetricCryptoKey,
    ) -> Result<CipherView, VaultError>;

    /// Decrypt multiple ciphers
    ///
    /// Continues on individual failures (logs warning, skips item).
    ///
    /// # Arguments
    /// * `ciphers` - List of encrypted ciphers
    /// * `user_key` - User's symmetric key for decryption
    ///
    /// # Returns
    /// List of successfully decrypted cipher views
    pub async fn decrypt_ciphers(
        &self,
        ciphers: &[Cipher],
        user_key: &SymmetricCryptoKey,
    ) -> Result<Vec<CipherView>, VaultError>;

    /// Decrypt folders
    ///
    /// # Arguments
    /// * `folders` - List of encrypted folders
    /// * `user_key` - User's symmetric key for decryption
    ///
    /// # Returns
    /// List of decrypted folder views
    pub async fn decrypt_folders(
        &self,
        folders: &[Folder],
        user_key: &SymmetricCryptoKey,
    ) -> Result<Vec<FolderView>, VaultError>;

    /// Decrypt collections
    ///
    /// # Arguments
    /// * `collections` - List of encrypted collections
    /// * `user_key` - User's symmetric key for decryption
    ///
    /// # Returns
    /// List of decrypted collection views
    pub async fn decrypt_collections(
        &self,
        collections: &[Collection],
        user_key: &SymmetricCryptoKey,
    ) -> Result<Vec<CollectionView>, VaultError>;

    // Private helper
    fn decrypt_string(
        &self,
        enc_string: &str,
        key: &SymmetricCryptoKey,
    ) -> Result<String, VaultError> {
        if enc_string.is_empty() {
            return Ok(String::new());
        }

        let enc: EncString = enc_string.parse()
            .map_err(|e| VaultError::DecryptionError(format!("Invalid EncString: {}", e)))?;

        enc.decrypt_with_key(key)
            .map_err(|e| VaultError::DecryptionError(format!("Decryption failed: {}", e)))
    }

    fn decrypt_optional_string(
        &self,
        enc_string: &Option<String>,
        key: &SymmetricCryptoKey,
    ) -> Result<Option<String>, VaultError> {
        match enc_string {
            Some(s) if !s.is_empty() => Ok(Some(self.decrypt_string(s, key)?)),
            _ => Ok(None),
        }
    }
}
```

---

## 4. Updated VaultService API

### Module: `crates/bw-core/src/services/vault/mod.rs`

```rust
impl VaultService {
    /// List all items with optional filters
    ///
    /// # Arguments
    /// * `filters` - Optional filters (org, collection, folder, search, etc.)
    /// * `session` - Session key string (from BW_SESSION or --session)
    ///
    /// # Returns
    /// List of decrypted cipher views matching filters
    pub async fn list_items(
        &self,
        filters: &ItemFilters,
        session: &str,
    ) -> Result<Vec<CipherView>, VaultError>;

    /// Get specific item by ID or search term
    ///
    /// # Arguments
    /// * `id_or_search` - UUID or search string
    /// * `session` - Session key string
    ///
    /// # Returns
    /// Decrypted cipher view
    pub async fn get_item(
        &self,
        id_or_search: &str,
        session: &str,
    ) -> Result<CipherView, VaultError>;

    /// Get specific field from item
    ///
    /// # Arguments
    /// * `id_or_search` - UUID or search string
    /// * `field` - Field type (Username, Password, Uri, Notes)
    /// * `session` - Session key string
    ///
    /// # Returns
    /// Decrypted field value
    pub async fn get_field(
        &self,
        id_or_search: &str,
        field: FieldType,
        session: &str,
    ) -> Result<String, VaultError>;

    /// Generate TOTP code for item
    ///
    /// # Arguments
    /// * `id_or_search` - UUID or search string
    /// * `session` - Session key string
    ///
    /// # Returns
    /// 6-digit TOTP code
    pub async fn get_totp(
        &self,
        id_or_search: &str,
        session: &str,
    ) -> Result<String, VaultError>;

    /// List all folders
    ///
    /// # Arguments
    /// * `search` - Optional search filter
    /// * `session` - Session key string
    ///
    /// # Returns
    /// List of decrypted folder views
    pub async fn list_folders(
        &self,
        search: Option<&str>,
        session: &str,
    ) -> Result<Vec<FolderView>, VaultError>;

    /// List all collections
    ///
    /// # Arguments
    /// * `organization_id` - Optional org filter
    /// * `search` - Optional search filter
    /// * `session` - Session key string
    ///
    /// # Returns
    /// List of decrypted collection views
    pub async fn list_collections(
        &self,
        organization_id: Option<&str>,
        search: Option<&str>,
        session: &str,
    ) -> Result<Vec<CollectionView>, VaultError>;
}
```

---

## 5. CLI Session Handling

### Helper function in `crates/bw-cli/src/commands/vault.rs`

```rust
/// Get session key from --session flag or BW_SESSION environment variable
///
/// # Arguments
/// * `global_args` - Global CLI arguments (may contain --session)
///
/// # Returns
/// Session key string
///
/// # Errors
/// - If neither --session nor BW_SESSION is provided
fn get_session(global_args: &GlobalArgs) -> Result<String, anyhow::Error> {
    // Priority 1: --session flag
    if let Some(ref session) = global_args.session {
        if !session.is_empty() {
            return Ok(session.clone());
        }
    }

    // Priority 2: BW_SESSION environment variable
    match std::env::var("BW_SESSION") {
        Ok(session) if !session.is_empty() => Ok(session),
        _ => Err(anyhow::anyhow!(
            "Session key required. Export BW_SESSION or use --session flag.\n\
            Run 'bw unlock' to get a new session key."
        )),
    }
}
```

---

## 6. Storage Format Specification

### Protected Storage Key Format

```
__PROTECTED__{userId}_user_auto

Example:
__PROTECTED__a1b2c3d4-e5f6-7890-abcd-ef1234567890_user_auto
```

### Session Key Format (BW_SESSION)

```
Base64-encoded 64 bytes:
- Bytes 0-31: AES-256 encryption key
- Bytes 32-63: HMAC-SHA256 MAC key

Example:
SGVsbG8gV29ybGQhIFRoaXMgaXMgYSB0ZXN0IGtleSBmb3IgZGVtb25zdHJhdGlvbiBwdXJwb3Nlcy4=
```

### EncArrayBuffer Binary Format

```
[1 byte: encryption type][16 bytes: IV][32 bytes: MAC][variable: ciphertext]

Type 2 = AES-256-CBC with HMAC-SHA256
```

### EncString String Format (vault fields)

```
{type}.{base64_iv}|{base64_ciphertext}|{base64_mac}

Example:
2.abc123==|def456==|ghi789==

Where:
- 2 = AES-256-CBC with HMAC-SHA256
- abc123== = base64-encoded 16-byte IV
- def456== = base64-encoded ciphertext
- ghi789== = base64-encoded 32-byte MAC
```
