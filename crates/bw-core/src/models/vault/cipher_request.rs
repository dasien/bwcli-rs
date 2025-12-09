//! API request models for cipher operations

use super::{
    Cipher, CipherCard, CipherField, CipherIdentity, CipherLogin, CipherSecureNote, CipherType,
    PasswordHistory,
};
use serde::{Deserialize, Serialize};

/// Request body for creating/updating ciphers
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CipherRequest {
    #[serde(rename = "type")]
    pub cipher_type: CipherType,
    pub name: String, // Encrypted
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>, // Encrypted
    pub favorite: bool,
    pub folder_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub login: Option<CipherLogin>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secure_note: Option<CipherSecureNote>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card: Option<CipherCard>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identity: Option<CipherIdentity>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<CipherField>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub password_history: Vec<PasswordHistory>,
}

impl From<Cipher> for CipherRequest {
    fn from(cipher: Cipher) -> Self {
        Self {
            cipher_type: cipher.cipher_type,
            name: cipher.name,
            notes: cipher.notes,
            favorite: cipher.favorite,
            folder_id: cipher.folder_id,
            organization_id: cipher.organization_id,
            login: cipher.login,
            secure_note: cipher.secure_note,
            card: cipher.card,
            identity: cipher.identity,
            fields: cipher.fields.unwrap_or_default(),
            password_history: cipher.password_history.unwrap_or_default(),
        }
    }
}

/// Request body for folder operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderRequest {
    pub name: String, // Encrypted
}
