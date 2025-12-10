//! Protected storage for encrypting sensitive data with session keys
//!
//! This module provides encryption/decryption for sensitive data stored locally
//! using the session key. This is compatible with the TypeScript CLI's protected
//! storage mechanism.

use base64::{Engine, engine::general_purpose::STANDARD};
use bitwarden_crypto::{EncString, KeyDecryptable, KeyEncryptable, OctetStreamBytes, SymmetricCryptoKey};
use thiserror::Error;

/// Prefix for protected storage keys
pub const PROTECTED_PREFIX: &str = "__PROTECTED__";

/// Protected storage error types
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
}

/// Format a storage key as a protected key
///
/// # Arguments
/// * `key` - The base storage key
///
/// # Returns
/// The key prefixed with `__PROTECTED__`
pub fn make_protected_key(key: &str) -> String {
    format!("{}{}", PROTECTED_PREFIX, key)
}

/// Generate the protected storage key for the user key
///
/// # Arguments
/// * `user_id` - The user's ID
///
/// # Returns
/// Storage key in format `{userId}_user_auto`
pub fn user_key_protected_storage_key(user_id: &str) -> String {
    format!("{}_user_auto", user_id)
}

/// Parse a session key from a base64-encoded string
///
/// The session key is expected to be a 64-byte key encoded as base64.
///
/// # Arguments
/// * `session_str` - Base64-encoded session key string
///
/// # Returns
/// The parsed SymmetricCryptoKey, or an error if parsing fails
pub fn parse_session_key(session_str: &str) -> Result<SymmetricCryptoKey, ProtectedStorageError> {
    // The SDK's SymmetricCryptoKey can be created from a String (base64-encoded)
    SymmetricCryptoKey::try_from(session_str.to_string()).map_err(|e| {
        ProtectedStorageError::InvalidSessionKey(format!("Failed to parse session key: {}", e))
    })
}

/// Format a session key for export to BW_SESSION environment variable
///
/// # Arguments
/// * `key` - The session key to format
///
/// # Returns
/// Base64-encoded session key string
pub fn format_session_key(key: &SymmetricCryptoKey) -> String {
    // Export the key as base64-encoded string
    key.to_base64().to_string()
}

/// Generate a new session key
///
/// Uses the SDK's SymmetricCryptoKey generation which provides
/// a cryptographically secure 64-byte key.
///
/// # Returns
/// A new random SymmetricCryptoKey
pub fn generate_session_key() -> SymmetricCryptoKey {
    SymmetricCryptoKey::make_aes256_cbc_hmac_key()
}

/// Encrypt a string using a session key
///
/// # Arguments
/// * `plain` - The plaintext string to encrypt
/// * `key` - The session key to use for encryption
///
/// # Returns
/// Base64-encoded encrypted data string (using EncString buffer format)
pub fn encrypt_protected_string(
    plain: &str,
    key: &SymmetricCryptoKey,
) -> Result<String, ProtectedStorageError> {
    // Encrypt the string using the SDK
    let enc_string: EncString = plain.encrypt_with_key(key).map_err(|e| {
        ProtectedStorageError::EncryptionFailed(format!("Encryption failed: {}", e))
    })?;

    // Convert to buffer format and encode as base64
    let buffer = enc_string.to_buffer().map_err(|e| {
        ProtectedStorageError::EncryptionFailed(format!("Failed to serialize: {}", e))
    })?;

    Ok(STANDARD.encode(&buffer))
}

/// Decrypt a string using a session key
///
/// # Arguments
/// * `encrypted_b64` - Base64-encoded encrypted data string
/// * `key` - The session key to use for decryption
///
/// # Returns
/// The decrypted plaintext string
pub fn decrypt_protected_string(
    encrypted_b64: &str,
    key: &SymmetricCryptoKey,
) -> Result<String, ProtectedStorageError> {
    // Decode base64
    let buffer = STANDARD
        .decode(encrypted_b64)
        .map_err(|e| ProtectedStorageError::InvalidBase64(format!("Invalid base64: {}", e)))?;

    // Parse as EncString from buffer format
    let enc_string = EncString::from_buffer(&buffer).map_err(|e| {
        ProtectedStorageError::DecryptionFailed(format!("Failed to parse encrypted data: {}", e))
    })?;

    // Decrypt using the session key
    enc_string.decrypt_with_key(key).map_err(|e| {
        ProtectedStorageError::DecryptionFailed(format!("Decryption failed: {}", e))
    })
}

/// Encrypt raw bytes using a session key
///
/// This is the general-purpose encryption function for binary data like
/// attachments. Uses OctetStreamBytes from the SDK.
///
/// # Arguments
/// * `plain` - The plaintext bytes to encrypt
/// * `key` - The session key to use for encryption
///
/// # Returns
/// Base64-encoded encrypted data string (using EncString buffer format)
pub fn encrypt_protected_bytes(
    plain: &[u8],
    key: &SymmetricCryptoKey,
) -> Result<String, ProtectedStorageError> {
    // Encrypt the bytes using OctetStreamBytes wrapper
    let enc_string: EncString = OctetStreamBytes::from(plain.to_vec())
        .encrypt_with_key(key)
        .map_err(|e| {
            ProtectedStorageError::EncryptionFailed(format!("Encryption failed: {}", e))
        })?;

    // Convert to buffer format and encode as base64
    let buffer = enc_string.to_buffer().map_err(|e| {
        ProtectedStorageError::EncryptionFailed(format!("Failed to serialize: {}", e))
    })?;

    Ok(STANDARD.encode(&buffer))
}

/// Decrypt raw bytes using a session key
///
/// This is the general-purpose decryption function for binary data like
/// attachments.
///
/// # Arguments
/// * `encrypted_b64` - Base64-encoded encrypted data string
/// * `key` - The session key to use for decryption
///
/// # Returns
/// The decrypted plaintext bytes
pub fn decrypt_protected_bytes(
    encrypted_b64: &str,
    key: &SymmetricCryptoKey,
) -> Result<Vec<u8>, ProtectedStorageError> {
    // Decode base64
    let buffer = STANDARD
        .decode(encrypted_b64)
        .map_err(|e| ProtectedStorageError::InvalidBase64(format!("Invalid base64: {}", e)))?;

    // Parse as EncString from buffer format
    let enc_string = EncString::from_buffer(&buffer).map_err(|e| {
        ProtectedStorageError::DecryptionFailed(format!("Failed to parse encrypted data: {}", e))
    })?;

    // Decrypt using the session key - returns Vec<u8>
    enc_string.decrypt_with_key(key).map_err(|e| {
        ProtectedStorageError::DecryptionFailed(format!("Decryption failed: {}", e))
    })
}

/// Encrypt a user key with a session key for protected storage
///
/// This matches the TypeScript CLI format:
/// - Get user key as base64 string
/// - Decode base64 to get raw key bytes
/// - Encrypt those raw bytes
///
/// # Arguments
/// * `user_key` - The user's symmetric key to encrypt
/// * `session_key` - The session key to use for encryption
///
/// # Returns
/// Base64-encoded encrypted user key string (EncArrayBuffer format)
pub fn encrypt_user_key(
    user_key: &SymmetricCryptoKey,
    session_key: &SymmetricCryptoKey,
) -> Result<String, ProtectedStorageError> {
    // Get the user key as base64 string
    let user_key_b64 = format_session_key(user_key);

    // Decode base64 to get raw key bytes (this is what TypeScript CLI does)
    let key_bytes = STANDARD.decode(&user_key_b64).map_err(|e| {
        ProtectedStorageError::InvalidBase64(format!("Failed to decode user key: {}", e))
    })?;

    // Encrypt the raw key bytes
    encrypt_protected_bytes(&key_bytes, session_key)
}

/// Decrypt a user key from protected storage
///
/// The TypeScript CLI stores user keys as raw bytes encrypted with the session key:
/// - Takes user key base64 string
/// - Decodes base64 to raw key bytes
/// - Encrypts those raw bytes
/// - Stores as base64
///
/// When decrypting, we get the raw key bytes back directly.
///
/// # Arguments
/// * `encrypted_b64` - Base64-encoded encrypted user key (EncArrayBuffer format)
/// * `session_key` - The session key to use for decryption
///
/// # Returns
/// The decrypted user key
pub fn decrypt_user_key(
    encrypted_b64: &str,
    session_key: &SymmetricCryptoKey,
) -> Result<SymmetricCryptoKey, ProtectedStorageError> {
    // Decrypt to raw bytes (the actual user key bytes)
    let key_bytes = decrypt_protected_bytes(encrypted_b64, session_key)?;

    // The decrypted bytes ARE the raw user key bytes (64 bytes for AES256-CBC-HMAC)
    // Re-encode as base64 to construct the key (SDK expects base64 input)
    let key_b64 = STANDARD.encode(&key_bytes);

    // Reconstruct the SymmetricCryptoKey from the base64 string
    SymmetricCryptoKey::try_from(key_b64).map_err(|e| {
        ProtectedStorageError::InvalidKeyFormat(format!("Failed to reconstruct key: {}", e))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_make_protected_key() {
        let key = make_protected_key("test_key");
        assert_eq!(key, "__PROTECTED__test_key");
    }

    #[test]
    fn test_user_key_protected_storage_key() {
        let key = user_key_protected_storage_key("user-123-abc");
        assert_eq!(key, "user-123-abc_user_auto");
    }

    #[test]
    fn test_session_key_roundtrip() {
        // Generate a new session key
        let original = generate_session_key();

        // Format for export
        let formatted = format_session_key(&original);

        // Parse back
        let parsed = parse_session_key(&formatted).expect("Should parse session key");

        // Verify by re-formatting (should produce same base64)
        let re_formatted = format_session_key(&parsed);
        assert_eq!(formatted, re_formatted);
    }

    #[test]
    fn test_invalid_session_key() {
        let result = parse_session_key("not-a-valid-key!");
        assert!(result.is_err());
    }

    #[test]
    fn test_encrypt_decrypt_string_roundtrip() {
        let session_key = generate_session_key();
        let plain_text = "Hello, World! This is secret data.";

        // Encrypt
        let encrypted = encrypt_protected_string(plain_text, &session_key)
            .expect("Should encrypt successfully");

        // Decrypt
        let decrypted = decrypt_protected_string(&encrypted, &session_key)
            .expect("Should decrypt successfully");

        assert_eq!(plain_text, decrypted);
    }

    #[test]
    fn test_encrypt_decrypt_user_key_roundtrip() {
        let session_key = generate_session_key();
        let user_key = generate_session_key(); // Using another key as the "user key"

        // Encrypt user key
        let encrypted = encrypt_user_key(&user_key, &session_key).expect("Should encrypt user key");

        // Decrypt user key
        let decrypted =
            decrypt_user_key(&encrypted, &session_key).expect("Should decrypt user key");

        // Verify the keys match by comparing their formatted output
        assert_eq!(
            format_session_key(&user_key),
            format_session_key(&decrypted)
        );
    }

    #[test]
    fn test_wrong_key_fails_decryption() {
        let session_key = generate_session_key();
        let wrong_key = generate_session_key();
        let plain_text = "Secret data";

        // Encrypt with session_key
        let encrypted =
            encrypt_protected_string(plain_text, &session_key).expect("Should encrypt successfully");

        // Try to decrypt with wrong_key
        let result = decrypt_protected_string(&encrypted, &wrong_key);
        assert!(result.is_err());
    }

    #[test]
    fn test_protected_storage_key_format() {
        // Verify key format matches TypeScript CLI expectations
        let user_id = "abc-123-def";
        let protected_key = make_protected_key(&user_key_protected_storage_key(user_id));
        assert_eq!(protected_key, "__PROTECTED__abc-123-def_user_auto");
    }

    #[test]
    fn test_encrypt_decrypt_bytes_roundtrip() {
        let session_key = generate_session_key();
        let plain_bytes: &[u8] = b"\x00\x01\x02\x03\xff\xfe\xfd binary data with null bytes";

        // Encrypt
        let encrypted = encrypt_protected_bytes(plain_bytes, &session_key)
            .expect("Should encrypt bytes successfully");

        // Decrypt
        let decrypted = decrypt_protected_bytes(&encrypted, &session_key)
            .expect("Should decrypt bytes successfully");

        assert_eq!(plain_bytes, decrypted.as_slice());
    }

    #[test]
    fn test_encrypt_decrypt_bytes_empty() {
        let session_key = generate_session_key();
        let plain_bytes: &[u8] = b"";

        // Encrypt
        let encrypted = encrypt_protected_bytes(plain_bytes, &session_key)
            .expect("Should encrypt empty bytes");

        // Decrypt
        let decrypted = decrypt_protected_bytes(&encrypted, &session_key)
            .expect("Should decrypt empty bytes");

        assert_eq!(plain_bytes, decrypted.as_slice());
    }

    #[test]
    fn test_bytes_wrong_key_fails_decryption() {
        let session_key = generate_session_key();
        let wrong_key = generate_session_key();
        let plain_bytes: &[u8] = b"Secret binary data";

        // Encrypt with session_key
        let encrypted = encrypt_protected_bytes(plain_bytes, &session_key)
            .expect("Should encrypt successfully");

        // Try to decrypt with wrong_key
        let result = decrypt_protected_bytes(&encrypted, &wrong_key);
        assert!(result.is_err());
    }
}
