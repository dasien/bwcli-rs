//! SDK-backed cryptographic operations
//!
//! This module provides thin wrappers around the Bitwarden SDK's crypto operations.
//! All cryptographic functions delegate to the SDK - no custom crypto is implemented here.

use bitwarden_crypto::{CryptoError, EncString, HashPurpose, Kdf, MasterKey, SymmetricCryptoKey};

/// Derive a master key from password, email, and KDF configuration
///
/// This wraps SDK's MasterKey::derive() for consistency with CLI patterns.
/// The email is automatically trimmed and converted to lowercase by the SDK.
///
/// # Arguments
/// * `password` - The user's master password
/// * `email` - The user's email address (used as salt)
/// * `kdf` - The KDF configuration (PBKDF2 or Argon2id)
///
/// # Returns
/// The derived master key, or an error if derivation fails
pub fn derive_master_key(password: &str, email: &str, kdf: &Kdf) -> Result<MasterKey, CryptoError> {
    MasterKey::derive(password, email, kdf)
}

/// Hash password for server authentication
///
/// Returns base64-encoded hash suitable for the login API.
/// Uses a single additional PBKDF2 iteration with the purpose flag.
///
/// # Arguments
/// * `master_key` - The derived master key
/// * `password` - The user's master password
///
/// # Returns
/// Base64-encoded password hash for server authentication
pub fn hash_password_for_auth(master_key: &MasterKey, password: &str) -> String {
    master_key
        .derive_master_key_hash(password.as_bytes(), HashPurpose::ServerAuthorization)
        .to_string()
}

/// Decrypt the user's symmetric key using their master key
///
/// The encrypted_key should be in EncString format (type.iv.ct.mac).
///
/// # Arguments
/// * `master_key` - The derived master key
/// * `encrypted_key` - The encrypted user key in EncString format
///
/// # Returns
/// The decrypted symmetric crypto key, or an error if decryption fails
pub fn decrypt_user_key(
    master_key: &MasterKey,
    encrypted_key: &str,
) -> Result<SymmetricCryptoKey, CryptoError> {
    let enc_string: EncString = encrypted_key.parse().map_err(|_| CryptoError::InvalidKey)?;

    master_key.decrypt_user_key(enc_string)
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use super::*;

    #[test]
    fn test_derive_master_key_pbkdf2() {
        // Test vector from SDK
        let password = "asdfasdf";
        let email = "test@bitwarden.com";
        let kdf = Kdf::PBKDF2 {
            iterations: NonZeroU32::new(100_000).unwrap(),
        };

        let master_key = derive_master_key(password, email, &kdf).expect("Should derive key");
        let hash = hash_password_for_auth(&master_key, password);

        // Expected hash from SDK test vectors
        assert_eq!(hash, "wmyadRMyBZOH7P/a/ucTCbSghKgdzDpPqUnu/DAVtSw=");
    }

    #[test]
    fn test_derive_master_key_argon2id() {
        // Test vector from SDK
        let password = "asdfasdf";
        let salt = "test_salt";
        let kdf = Kdf::Argon2id {
            iterations: NonZeroU32::new(4).unwrap(),
            memory: NonZeroU32::new(32).unwrap(),
            parallelism: NonZeroU32::new(2).unwrap(),
        };

        let master_key = derive_master_key(password, salt, &kdf).expect("Should derive key");
        let hash = hash_password_for_auth(&master_key, password);

        // Expected hash from SDK test vectors
        assert_eq!(hash, "PR6UjYmjmppTYcdyTiNbAhPJuQQOmynKbdEl1oyi/iQ=");
    }

    #[test]
    fn test_email_normalization() {
        // SDK normalizes email by trimming and lowercasing
        let password = "asdfasdf";
        let kdf = Kdf::PBKDF2 {
            iterations: NonZeroU32::new(100_000).unwrap(),
        };

        // All these should produce the same key
        let key1 = derive_master_key(password, "test@bitwarden.com", &kdf)
            .expect("Should derive key")
            .derive_master_key_hash(password.as_bytes(), HashPurpose::ServerAuthorization);

        let key2 = derive_master_key(password, "TEST@bitwarden.com", &kdf)
            .expect("Should derive key")
            .derive_master_key_hash(password.as_bytes(), HashPurpose::ServerAuthorization);

        let key3 = derive_master_key(password, " test@bitwarden.com", &kdf)
            .expect("Should derive key")
            .derive_master_key_hash(password.as_bytes(), HashPurpose::ServerAuthorization);

        assert_eq!(key1.to_string(), key2.to_string());
        assert_eq!(key2.to_string(), key3.to_string());
    }

    #[test]
    fn test_decrypt_user_key_invalid_format() {
        let password = "test";
        let email = "test@test.com";
        let kdf = Kdf::PBKDF2 {
            iterations: NonZeroU32::new(100_000).unwrap(),
        };

        let master_key = derive_master_key(password, email, &kdf).expect("Should derive key");

        // Invalid format should fail
        let result = decrypt_user_key(&master_key, "invalid");
        assert!(result.is_err());
    }
}
