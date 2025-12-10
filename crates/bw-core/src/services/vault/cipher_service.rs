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
            notes: if let Some(notes) = &cipher.notes {
                Some(self.decrypt_string(notes, user_key)?)
            } else {
                None
            },
            favorite: cipher.favorite,
            collection_ids: cipher.collection_ids.clone(),
            revision_date: cipher.revision_date.clone(),
            creation_date: cipher.creation_date.clone(),
            deleted_date: cipher.deleted_date.clone(),
            login: if let Some(login) = &cipher.login {
                Some(self.decrypt_login(login, user_key)?)
            } else {
                None
            },
            secure_note: cipher.secure_note.clone(),
            card: if let Some(card) = &cipher.card {
                Some(self.decrypt_card(card, user_key)?)
            } else {
                None
            },
            identity: if let Some(identity) = &cipher.identity {
                Some(self.decrypt_identity(identity, user_key)?)
            } else {
                None
            },
            attachments: cipher.attachments.clone().unwrap_or_default(),
            fields: {
                let mut fields = Vec::new();
                if let Some(cipher_fields) = &cipher.fields {
                    for field in cipher_fields {
                        fields.push(CipherFieldView {
                            name: self.decrypt_string(&field.name, user_key)?,
                            value: if let Some(v) = &field.value {
                                Some(self.decrypt_string(v, user_key)?)
                            } else {
                                None
                            },
                            field_type: field.field_type,
                        });
                    }
                }
                fields
            },
        })
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
        Ok(CipherLoginView {
            username: if let Some(u) = &login.username {
                Some(self.decrypt_string(u, key)?)
            } else {
                None
            },
            password: if let Some(p) = &login.password {
                Some(self.decrypt_string(p, key)?)
            } else {
                None
            },
            uris: {
                let mut uris = Vec::new();
                for uri in &login.uris {
                    uris.push(CipherLoginUriView {
                        uri: if let Some(u) = &uri.uri {
                            Some(self.decrypt_string(u, key)?)
                        } else {
                            None
                        },
                        match_type: uri.match_type,
                    });
                }
                uris
            },
            totp: if let Some(t) = &login.totp {
                Some(self.decrypt_string(t, key)?)
            } else {
                None
            },
        })
    }

    fn decrypt_card(
        &self,
        card: &CipherCard,
        key: &SymmetricCryptoKey,
    ) -> Result<CipherCardView, VaultError> {
        Ok(CipherCardView {
            cardholder_name: if let Some(n) = &card.cardholder_name {
                Some(self.decrypt_string(n, key)?)
            } else {
                None
            },
            number: if let Some(n) = &card.number {
                Some(self.decrypt_string(n, key)?)
            } else {
                None
            },
            brand: if let Some(b) = &card.brand {
                Some(self.decrypt_string(b, key)?)
            } else {
                None
            },
            exp_month: if let Some(m) = &card.exp_month {
                Some(self.decrypt_string(m, key)?)
            } else {
                None
            },
            exp_year: if let Some(y) = &card.exp_year {
                Some(self.decrypt_string(y, key)?)
            } else {
                None
            },
            code: if let Some(c) = &card.code {
                Some(self.decrypt_string(c, key)?)
            } else {
                None
            },
        })
    }

    fn decrypt_identity(
        &self,
        identity: &CipherIdentity,
        key: &SymmetricCryptoKey,
    ) -> Result<CipherIdentityView, VaultError> {
        Ok(CipherIdentityView {
            title: if let Some(t) = &identity.title {
                Some(self.decrypt_string(t, key)?)
            } else {
                None
            },
            first_name: if let Some(f) = &identity.first_name {
                Some(self.decrypt_string(f, key)?)
            } else {
                None
            },
            middle_name: if let Some(m) = &identity.middle_name {
                Some(self.decrypt_string(m, key)?)
            } else {
                None
            },
            last_name: if let Some(l) = &identity.last_name {
                Some(self.decrypt_string(l, key)?)
            } else {
                None
            },
            address1: if let Some(a) = &identity.address1 {
                Some(self.decrypt_string(a, key)?)
            } else {
                None
            },
            address2: if let Some(a) = &identity.address2 {
                Some(self.decrypt_string(a, key)?)
            } else {
                None
            },
            address3: if let Some(a) = &identity.address3 {
                Some(self.decrypt_string(a, key)?)
            } else {
                None
            },
            city: if let Some(c) = &identity.city {
                Some(self.decrypt_string(c, key)?)
            } else {
                None
            },
            state: if let Some(s) = &identity.state {
                Some(self.decrypt_string(s, key)?)
            } else {
                None
            },
            postal_code: if let Some(p) = &identity.postal_code {
                Some(self.decrypt_string(p, key)?)
            } else {
                None
            },
            country: if let Some(c) = &identity.country {
                Some(self.decrypt_string(c, key)?)
            } else {
                None
            },
            phone: if let Some(p) = &identity.phone {
                Some(self.decrypt_string(p, key)?)
            } else {
                None
            },
            email: if let Some(e) = &identity.email {
                Some(self.decrypt_string(e, key)?)
            } else {
                None
            },
            ssn: if let Some(s) = &identity.ssn {
                Some(self.decrypt_string(s, key)?)
            } else {
                None
            },
            username: if let Some(u) = &identity.username {
                Some(self.decrypt_string(u, key)?)
            } else {
                None
            },
            passport_number: if let Some(p) = &identity.passport_number {
                Some(self.decrypt_string(p, key)?)
            } else {
                None
            },
            license_number: if let Some(l) = &identity.license_number {
                Some(self.decrypt_string(l, key)?)
            } else {
                None
            },
        })
    }

    // ========== Encryption Methods (for write operations) ==========

    /// Encrypt cipher view to cipher (for API submission)
    pub async fn encrypt_cipher(
        &self,
        cipher_view: &CipherView,
        user_key: &SymmetricCryptoKey,
    ) -> Result<Cipher, VaultError> {
        let encrypted_name = self.encrypt_string(&cipher_view.name, user_key)?;

        let encrypted_notes = if let Some(notes) = &cipher_view.notes {
            Some(self.encrypt_string(notes, user_key)?)
        } else {
            None
        };

        let encrypted_login = if let Some(login) = &cipher_view.login {
            Some(self.encrypt_login(login, user_key)?)
        } else {
            None
        };

        let encrypted_secure_note = cipher_view.secure_note.clone();

        let encrypted_card = if let Some(card) = &cipher_view.card {
            Some(self.encrypt_card(card, user_key)?)
        } else {
            None
        };

        let encrypted_identity = if let Some(identity) = &cipher_view.identity {
            Some(self.encrypt_identity(identity, user_key)?)
        } else {
            None
        };

        let encrypted_fields = self.encrypt_fields(&cipher_view.fields, user_key)?;

        // Build encrypted cipher
        Ok(Cipher {
            id: cipher_view.id.clone(),
            organization_id: cipher_view.organization_id.clone(),
            folder_id: cipher_view.folder_id.clone(),
            cipher_type: cipher_view.cipher_type,
            name: encrypted_name,
            notes: encrypted_notes,
            favorite: cipher_view.favorite,
            edit: true,
            view_password: true,
            permissions: None,
            collection_ids: cipher_view.collection_ids.clone(),
            revision_date: cipher_view.revision_date.clone(),
            creation_date: cipher_view.creation_date.clone(),
            deleted_date: cipher_view.deleted_date.clone(),
            login: encrypted_login,
            secure_note: encrypted_secure_note,
            card: encrypted_card,
            identity: encrypted_identity,
            ssh_key: None,
            attachments: Some(cipher_view.attachments.clone()),
            fields: Some(encrypted_fields),
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
        let encrypted_username = if let Some(username) = &login.username {
            Some(self.encrypt_string(username, key)?)
        } else {
            None
        };

        let encrypted_password = if let Some(password) = &login.password {
            Some(self.encrypt_string(password, key)?)
        } else {
            None
        };

        let encrypted_totp = if let Some(totp) = &login.totp {
            Some(self.encrypt_string(totp, key)?)
        } else {
            None
        };

        let encrypted_uris = self.encrypt_uris(&login.uris, key)?;

        Ok(CipherLogin {
            username: encrypted_username,
            password: encrypted_password,
            totp: encrypted_totp,
            uris: encrypted_uris,
            uri: None,
            autofill_on_page_load: None,
            password_revision_date: None,
            fido2_credentials: None,
        })
    }

    /// Encrypt URI list
    fn encrypt_uris(
        &self,
        uris: &[CipherLoginUriView],
        key: &SymmetricCryptoKey,
    ) -> Result<Vec<CipherLoginUri>, VaultError> {
        let mut encrypted_uris = Vec::new();

        for uri_view in uris {
            let encrypted_uri = if let Some(uri) = &uri_view.uri {
                Some(self.encrypt_string(uri, key)?)
            } else {
                None
            };

            encrypted_uris.push(CipherLoginUri {
                uri: encrypted_uri,
                match_type: uri_view.match_type,
            });
        }

        Ok(encrypted_uris)
    }

    /// Encrypt card data
    fn encrypt_card(
        &self,
        card: &CipherCardView,
        key: &SymmetricCryptoKey,
    ) -> Result<CipherCard, VaultError> {
        Ok(CipherCard {
            cardholder_name: if let Some(n) = &card.cardholder_name {
                Some(self.encrypt_string(n, key)?)
            } else {
                None
            },
            number: if let Some(n) = &card.number {
                Some(self.encrypt_string(n, key)?)
            } else {
                None
            },
            brand: if let Some(b) = &card.brand {
                Some(self.encrypt_string(b, key)?)
            } else {
                None
            },
            exp_month: if let Some(m) = &card.exp_month {
                Some(self.encrypt_string(m, key)?)
            } else {
                None
            },
            exp_year: if let Some(y) = &card.exp_year {
                Some(self.encrypt_string(y, key)?)
            } else {
                None
            },
            code: if let Some(c) = &card.code {
                Some(self.encrypt_string(c, key)?)
            } else {
                None
            },
        })
    }

    /// Encrypt identity data
    fn encrypt_identity(
        &self,
        identity: &CipherIdentityView,
        key: &SymmetricCryptoKey,
    ) -> Result<CipherIdentity, VaultError> {
        Ok(CipherIdentity {
            title: if let Some(t) = &identity.title {
                Some(self.encrypt_string(t, key)?)
            } else {
                None
            },
            first_name: if let Some(f) = &identity.first_name {
                Some(self.encrypt_string(f, key)?)
            } else {
                None
            },
            middle_name: if let Some(m) = &identity.middle_name {
                Some(self.encrypt_string(m, key)?)
            } else {
                None
            },
            last_name: if let Some(l) = &identity.last_name {
                Some(self.encrypt_string(l, key)?)
            } else {
                None
            },
            address1: if let Some(a) = &identity.address1 {
                Some(self.encrypt_string(a, key)?)
            } else {
                None
            },
            address2: if let Some(a) = &identity.address2 {
                Some(self.encrypt_string(a, key)?)
            } else {
                None
            },
            address3: if let Some(a) = &identity.address3 {
                Some(self.encrypt_string(a, key)?)
            } else {
                None
            },
            city: if let Some(c) = &identity.city {
                Some(self.encrypt_string(c, key)?)
            } else {
                None
            },
            state: if let Some(s) = &identity.state {
                Some(self.encrypt_string(s, key)?)
            } else {
                None
            },
            postal_code: if let Some(p) = &identity.postal_code {
                Some(self.encrypt_string(p, key)?)
            } else {
                None
            },
            country: if let Some(c) = &identity.country {
                Some(self.encrypt_string(c, key)?)
            } else {
                None
            },
            phone: if let Some(p) = &identity.phone {
                Some(self.encrypt_string(p, key)?)
            } else {
                None
            },
            email: if let Some(e) = &identity.email {
                Some(self.encrypt_string(e, key)?)
            } else {
                None
            },
            ssn: if let Some(s) = &identity.ssn {
                Some(self.encrypt_string(s, key)?)
            } else {
                None
            },
            username: if let Some(u) = &identity.username {
                Some(self.encrypt_string(u, key)?)
            } else {
                None
            },
            passport_number: if let Some(p) = &identity.passport_number {
                Some(self.encrypt_string(p, key)?)
            } else {
                None
            },
            license_number: if let Some(l) = &identity.license_number {
                Some(self.encrypt_string(l, key)?)
            } else {
                None
            },
        })
    }

    /// Encrypt custom fields
    fn encrypt_fields(
        &self,
        fields: &[CipherFieldView],
        key: &SymmetricCryptoKey,
    ) -> Result<Vec<CipherField>, VaultError> {
        let mut encrypted_fields = Vec::new();

        for field_view in fields {
            encrypted_fields.push(CipherField {
                name: self.encrypt_string(&field_view.name, key)?,
                value: if let Some(v) = &field_view.value {
                    Some(self.encrypt_string(v, key)?)
                } else {
                    None
                },
                field_type: field_view.field_type,
            });
        }

        Ok(encrypted_fields)
    }
}
