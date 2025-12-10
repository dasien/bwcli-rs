//! Cipher encryption/decryption service
//!
//! Handles all SDK integration for encrypting and decrypting vault items.
//! Uses the bitwarden-crypto SDK for all cryptographic operations.

use super::errors::VaultError;
use crate::models::vault::{
    Cipher, CipherCard, CipherCardView, CipherField, CipherFieldView, CipherIdentity,
    CipherIdentityView, CipherLogin, CipherLoginUri, CipherLoginUriView, CipherLoginView,
    CipherView, Collection, CollectionView, Folder, FolderView,
};
use crate::services::sdk::Client;
use bitwarden_crypto::{EncString, KeyDecryptable, KeyEncryptable, SymmetricCryptoKey};
use std::sync::Arc;

/// Service for cipher decryption operations
///
/// Handles all SDK integration for decrypting vault items.
pub struct CipherService {
    #[allow(dead_code)]
    sdk_client: Arc<Client>,
}

impl CipherService {
    pub fn new(sdk_client: Arc<Client>) -> Self {
        Self { sdk_client }
    }

    /// Decrypt a single cipher using the provided user key
    pub async fn decrypt_cipher(
        &self,
        cipher: &Cipher,
        user_key: &SymmetricCryptoKey,
    ) -> Result<CipherView, VaultError> {
        Ok(CipherView {
            id: cipher.id.clone(),
            organization_id: cipher.organization_id.clone(),
            folder_id: cipher.folder_id.clone(),
            cipher_type: cipher.cipher_type,
            name: self.decrypt_string(&cipher.name, user_key)?,
            notes: self.decrypt_optional(&cipher.notes, user_key)?,
            favorite: cipher.favorite,
            collection_ids: cipher.collection_ids.clone(),
            revision_date: cipher.revision_date.clone(),
            creation_date: cipher.creation_date.clone(),
            deleted_date: cipher.deleted_date.clone(),
            login: cipher
                .login
                .as_ref()
                .map(|l| self.decrypt_login(l, user_key))
                .transpose()?,
            secure_note: cipher.secure_note.clone(),
            card: cipher
                .card
                .as_ref()
                .map(|c| self.decrypt_card(c, user_key))
                .transpose()?,
            identity: cipher
                .identity
                .as_ref()
                .map(|i| self.decrypt_identity(i, user_key))
                .transpose()?,
            attachments: cipher.attachments.clone().unwrap_or_default(),
            fields: self.decrypt_fields(&cipher.fields, user_key)?,
        })
    }

    /// Decrypt fields list
    fn decrypt_fields(
        &self,
        fields: &Option<Vec<CipherField>>,
        key: &SymmetricCryptoKey,
    ) -> Result<Vec<CipherFieldView>, VaultError> {
        let Some(cipher_fields) = fields else {
            return Ok(Vec::new());
        };

        cipher_fields
            .iter()
            .map(|field| {
                Ok(CipherFieldView {
                    name: self.decrypt_string(&field.name, key)?,
                    value: self.decrypt_optional(&field.value, key)?,
                    field_type: field.field_type,
                })
            })
            .collect()
    }

    /// Decrypt multiple ciphers using the provided user key
    pub async fn decrypt_ciphers(
        &self,
        ciphers: &[Cipher],
        user_key: &SymmetricCryptoKey,
    ) -> Result<Vec<CipherView>, VaultError> {
        let mut results = Vec::new();
        for cipher in ciphers {
            match self.decrypt_cipher(cipher, user_key).await {
                Ok(decrypted) => results.push(decrypted),
                Err(e) => {
                    // Log error but continue with other ciphers
                    tracing::warn!("Failed to decrypt cipher {}: {}", cipher.id, e);
                }
            }
        }
        Ok(results)
    }

    /// Decrypt folders using the provided user key
    pub async fn decrypt_folders(
        &self,
        folders: &[Folder],
        user_key: &SymmetricCryptoKey,
    ) -> Result<Vec<FolderView>, VaultError> {
        let mut results = Vec::new();
        for folder in folders {
            results.push(FolderView {
                id: folder.id.clone(),
                name: self.decrypt_string(&folder.name, user_key)?,
                revision_date: folder.revision_date.clone(),
            });
        }
        Ok(results)
    }

    /// Decrypt collections using the provided user key
    pub async fn decrypt_collections(
        &self,
        collections: &[Collection],
        user_key: &SymmetricCryptoKey,
    ) -> Result<Vec<CollectionView>, VaultError> {
        let mut results = Vec::new();
        for collection in collections {
            results.push(CollectionView {
                id: collection.id.clone(),
                organization_id: collection.organization_id.clone(),
                name: self.decrypt_string(&collection.name, user_key)?,
                external_id: collection.external_id.clone(),
                read_only: collection.read_only,
            });
        }
        Ok(results)
    }

    // Private helper methods for decryption

    /// Decrypt an optional encrypted string using the user key
    ///
    /// Returns Ok(None) if input is None, Ok(Some(decrypted)) if successful
    fn decrypt_optional(
        &self,
        val: &Option<String>,
        key: &SymmetricCryptoKey,
    ) -> Result<Option<String>, VaultError> {
        val.as_ref()
            .map(|v| self.decrypt_string(v, key))
            .transpose()
    }

    /// Encrypt an optional string using the user key
    ///
    /// Returns Ok(None) if input is None, Ok(Some(encrypted)) if successful
    fn encrypt_optional(
        &self,
        val: &Option<String>,
        key: &SymmetricCryptoKey,
    ) -> Result<Option<String>, VaultError> {
        val.as_ref()
            .map(|v| self.encrypt_string(v, key))
            .transpose()
    }

    /// Decrypt an encrypted string using the user key
    fn decrypt_string(
        &self,
        enc_string: &str,
        key: &SymmetricCryptoKey,
    ) -> Result<String, VaultError> {
        if enc_string.is_empty() {
            return Ok(String::new());
        }

        // Parse the encrypted string (format: "type.iv|ct|mac" or "type.iv|ct")
        let enc: EncString = enc_string
            .parse()
            .map_err(|e| VaultError::DecryptionError(format!("Invalid EncString format: {}", e)))?;

        // Decrypt using the SDK
        enc.decrypt_with_key(key)
            .map_err(|e| VaultError::DecryptionError(format!("Decryption failed: {}", e)))
    }

    fn decrypt_login(
        &self,
        login: &CipherLogin,
        key: &SymmetricCryptoKey,
    ) -> Result<CipherLoginView, VaultError> {
        let uris = login
            .uris
            .iter()
            .map(|uri| {
                Ok(CipherLoginUriView {
                    uri: self.decrypt_optional(&uri.uri, key)?,
                    match_type: uri.match_type,
                })
            })
            .collect::<Result<Vec<_>, VaultError>>()?;

        Ok(CipherLoginView {
            username: self.decrypt_optional(&login.username, key)?,
            password: self.decrypt_optional(&login.password, key)?,
            uris,
            totp: self.decrypt_optional(&login.totp, key)?,
        })
    }

    fn decrypt_card(
        &self,
        card: &CipherCard,
        key: &SymmetricCryptoKey,
    ) -> Result<CipherCardView, VaultError> {
        Ok(CipherCardView {
            cardholder_name: self.decrypt_optional(&card.cardholder_name, key)?,
            number: self.decrypt_optional(&card.number, key)?,
            brand: self.decrypt_optional(&card.brand, key)?,
            exp_month: self.decrypt_optional(&card.exp_month, key)?,
            exp_year: self.decrypt_optional(&card.exp_year, key)?,
            code: self.decrypt_optional(&card.code, key)?,
        })
    }

    fn decrypt_identity(
        &self,
        identity: &CipherIdentity,
        key: &SymmetricCryptoKey,
    ) -> Result<CipherIdentityView, VaultError> {
        Ok(CipherIdentityView {
            title: self.decrypt_optional(&identity.title, key)?,
            first_name: self.decrypt_optional(&identity.first_name, key)?,
            middle_name: self.decrypt_optional(&identity.middle_name, key)?,
            last_name: self.decrypt_optional(&identity.last_name, key)?,
            address1: self.decrypt_optional(&identity.address1, key)?,
            address2: self.decrypt_optional(&identity.address2, key)?,
            address3: self.decrypt_optional(&identity.address3, key)?,
            city: self.decrypt_optional(&identity.city, key)?,
            state: self.decrypt_optional(&identity.state, key)?,
            postal_code: self.decrypt_optional(&identity.postal_code, key)?,
            country: self.decrypt_optional(&identity.country, key)?,
            phone: self.decrypt_optional(&identity.phone, key)?,
            email: self.decrypt_optional(&identity.email, key)?,
            ssn: self.decrypt_optional(&identity.ssn, key)?,
            username: self.decrypt_optional(&identity.username, key)?,
            passport_number: self.decrypt_optional(&identity.passport_number, key)?,
            license_number: self.decrypt_optional(&identity.license_number, key)?,
        })
    }

    // ========== Encryption Methods (for write operations) ==========

    /// Encrypt cipher view to cipher (for API submission)
    pub async fn encrypt_cipher(
        &self,
        cipher_view: &CipherView,
        user_key: &SymmetricCryptoKey,
    ) -> Result<Cipher, VaultError> {
        // Build encrypted cipher
        Ok(Cipher {
            id: cipher_view.id.clone(),
            organization_id: cipher_view.organization_id.clone(),
            folder_id: cipher_view.folder_id.clone(),
            cipher_type: cipher_view.cipher_type,
            name: self.encrypt_string(&cipher_view.name, user_key)?,
            notes: self.encrypt_optional(&cipher_view.notes, user_key)?,
            favorite: cipher_view.favorite,
            edit: true,
            view_password: true,
            permissions: None,
            collection_ids: cipher_view.collection_ids.clone(),
            revision_date: cipher_view.revision_date.clone(),
            creation_date: cipher_view.creation_date.clone(),
            deleted_date: cipher_view.deleted_date.clone(),
            login: cipher_view
                .login
                .as_ref()
                .map(|l| self.encrypt_login(l, user_key))
                .transpose()?,
            secure_note: cipher_view.secure_note.clone(),
            card: cipher_view
                .card
                .as_ref()
                .map(|c| self.encrypt_card(c, user_key))
                .transpose()?,
            identity: cipher_view
                .identity
                .as_ref()
                .map(|i| self.encrypt_identity(i, user_key))
                .transpose()?,
            ssh_key: None,
            attachments: Some(cipher_view.attachments.clone()),
            fields: Some(self.encrypt_fields(&cipher_view.fields, user_key)?),
            password_history: None,
            organization_use_totp: false,
            reprompt: 0,
            key: None,
            object: Some("cipher".to_string()),
            archived_date: None,
            data: None,
        })
    }

    /// Encrypt a single string field using the user key
    pub fn encrypt_string(
        &self,
        plain_text: &str,
        key: &SymmetricCryptoKey,
    ) -> Result<String, VaultError> {
        if plain_text.is_empty() {
            return Ok(String::new());
        }

        // Encrypt using the SDK
        let enc_string: EncString = plain_text
            .encrypt_with_key(key)
            .map_err(|e| VaultError::EncryptionError(format!("Encryption failed: {}", e)))?;

        Ok(enc_string.to_string())
    }

    /// Encrypt login data
    fn encrypt_login(
        &self,
        login: &CipherLoginView,
        key: &SymmetricCryptoKey,
    ) -> Result<CipherLogin, VaultError> {
        let encrypted_uris = login
            .uris
            .iter()
            .map(|uri| {
                Ok(CipherLoginUri {
                    uri: self.encrypt_optional(&uri.uri, key)?,
                    match_type: uri.match_type,
                })
            })
            .collect::<Result<Vec<_>, VaultError>>()?;

        Ok(CipherLogin {
            username: self.encrypt_optional(&login.username, key)?,
            password: self.encrypt_optional(&login.password, key)?,
            totp: self.encrypt_optional(&login.totp, key)?,
            uris: encrypted_uris,
            uri: None,
            autofill_on_page_load: None,
            password_revision_date: None,
            fido2_credentials: None,
        })
    }

    /// Encrypt card data
    fn encrypt_card(
        &self,
        card: &CipherCardView,
        key: &SymmetricCryptoKey,
    ) -> Result<CipherCard, VaultError> {
        Ok(CipherCard {
            cardholder_name: self.encrypt_optional(&card.cardholder_name, key)?,
            number: self.encrypt_optional(&card.number, key)?,
            brand: self.encrypt_optional(&card.brand, key)?,
            exp_month: self.encrypt_optional(&card.exp_month, key)?,
            exp_year: self.encrypt_optional(&card.exp_year, key)?,
            code: self.encrypt_optional(&card.code, key)?,
        })
    }

    /// Encrypt identity data
    fn encrypt_identity(
        &self,
        identity: &CipherIdentityView,
        key: &SymmetricCryptoKey,
    ) -> Result<CipherIdentity, VaultError> {
        Ok(CipherIdentity {
            title: self.encrypt_optional(&identity.title, key)?,
            first_name: self.encrypt_optional(&identity.first_name, key)?,
            middle_name: self.encrypt_optional(&identity.middle_name, key)?,
            last_name: self.encrypt_optional(&identity.last_name, key)?,
            address1: self.encrypt_optional(&identity.address1, key)?,
            address2: self.encrypt_optional(&identity.address2, key)?,
            address3: self.encrypt_optional(&identity.address3, key)?,
            city: self.encrypt_optional(&identity.city, key)?,
            state: self.encrypt_optional(&identity.state, key)?,
            postal_code: self.encrypt_optional(&identity.postal_code, key)?,
            country: self.encrypt_optional(&identity.country, key)?,
            phone: self.encrypt_optional(&identity.phone, key)?,
            email: self.encrypt_optional(&identity.email, key)?,
            ssn: self.encrypt_optional(&identity.ssn, key)?,
            username: self.encrypt_optional(&identity.username, key)?,
            passport_number: self.encrypt_optional(&identity.passport_number, key)?,
            license_number: self.encrypt_optional(&identity.license_number, key)?,
        })
    }

    /// Encrypt custom fields
    fn encrypt_fields(
        &self,
        fields: &[CipherFieldView],
        key: &SymmetricCryptoKey,
    ) -> Result<Vec<CipherField>, VaultError> {
        fields
            .iter()
            .map(|field_view| {
                Ok(CipherField {
                    name: self.encrypt_string(&field_view.name, key)?,
                    value: self.encrypt_optional(&field_view.value, key)?,
                    field_type: field_view.field_type,
                })
            })
            .collect()
    }
}
