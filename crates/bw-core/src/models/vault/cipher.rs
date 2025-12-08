//! Cipher (vault item) data models
//!
//! Contains all cipher types (Login, SecureNote, Card, Identity) and their
//! encrypted/decrypted representations matching the Bitwarden API format.

use serde::{Deserialize, Serialize};

/// Encrypted vault cipher (item)
///
/// Matches Bitwarden API response format exactly.
/// All sensitive fields are encrypted using EncString format ("2.base64|base64|base64").
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cipher {
    /// Cipher ID (UUID)
    pub id: String,

    /// Organization ID (if shared)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_id: Option<String>,

    /// Folder ID (if in folder, null if no folder)
    pub folder_id: Option<String>,

    /// Cipher type: 1=Login, 2=SecureNote, 3=Card, 4=Identity
    #[serde(rename = "type")]
    pub cipher_type: CipherType,

    /// Encrypted name (EncString format)
    pub name: String,

    /// Encrypted notes (EncString format)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,

    /// Whether this is a favorite
    pub favorite: bool,

    /// Collection IDs this cipher belongs to
    #[serde(default)]
    pub collection_ids: Vec<String>,

    /// Revision date (ISO 8601)
    pub revision_date: String,

    /// Creation date (ISO 8601)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creation_date: Option<String>,

    /// Deleted date (ISO 8601, present if in trash)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted_date: Option<String>,

    /// Login-specific data (if type=1)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub login: Option<CipherLogin>,

    /// Secure note data (if type=2)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secure_note: Option<CipherSecureNote>,

    /// Card data (if type=3)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card: Option<CipherCard>,

    /// Identity data (if type=4)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identity: Option<CipherIdentity>,

    /// Attachments
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<CipherAttachment>,

    /// Custom fields
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<CipherField>,

    /// Password history
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub password_history: Vec<PasswordHistory>,
}

/// Cipher type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum CipherType {
    Login = 1,
    SecureNote = 2,
    Card = 3,
    Identity = 4,
}

/// Login cipher type data
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CipherLogin {
    /// Encrypted username (EncString)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,

    /// Encrypted password (EncString)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,

    /// URIs associated with login
    #[serde(default)]
    pub uris: Vec<CipherLoginUri>,

    /// Encrypted TOTP secret (EncString)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub totp: Option<String>,

    /// Whether password should be auto-filled on page load
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autofill_on_page_load: Option<bool>,
}

/// Login URI
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CipherLoginUri {
    /// Encrypted URI (EncString)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,

    /// URI match type (0=Domain, 1=Host, 2=StartsWith, 3=Exact, 4=RegEx, 5=Never)
    #[serde(rename = "match")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_type: Option<UriMatchType>,
}

/// URI match type for URL matching
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum UriMatchType {
    Domain = 0,
    Host = 1,
    StartsWith = 2,
    Exact = 3,
    RegularExpression = 4,
    Never = 5,
}

/// Secure note data
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CipherSecureNote {
    /// Note type (always 0 for generic note)
    #[serde(rename = "type")]
    pub note_type: u8,
}

/// Card data
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CipherCard {
    /// Encrypted cardholder name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cardholder_name: Option<String>,

    /// Encrypted card number
    #[serde(skip_serializing_if = "Option::is_none")]
    pub number: Option<String>,

    /// Encrypted brand (Visa, Mastercard, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brand: Option<String>,

    /// Encrypted expiration month (MM format)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exp_month: Option<String>,

    /// Encrypted expiration year (YYYY format)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exp_year: Option<String>,

    /// Encrypted CVV code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

/// Identity data
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CipherIdentity {
    /// Encrypted title (Mr, Mrs, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// Encrypted first name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_name: Option<String>,

    /// Encrypted middle name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub middle_name: Option<String>,

    /// Encrypted last name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_name: Option<String>,

    /// Encrypted address line 1
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address1: Option<String>,

    /// Encrypted address line 2
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address2: Option<String>,

    /// Encrypted address line 3
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address3: Option<String>,

    /// Encrypted city
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,

    /// Encrypted state/province
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,

    /// Encrypted postal code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub postal_code: Option<String>,

    /// Encrypted country
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,

    /// Encrypted phone
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,

    /// Encrypted email
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,

    /// Encrypted SSN
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssn: Option<String>,

    /// Encrypted username
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,

    /// Encrypted passport number
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passport_number: Option<String>,

    /// Encrypted license number
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license_number: Option<String>,
}

/// Cipher attachment
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CipherAttachment {
    /// Attachment ID
    pub id: String,

    /// Encrypted filename
    pub file_name: String,

    /// File size in bytes
    pub size: Option<u64>,

    /// Size string (e.g., "1.2 MB")
    pub size_name: Option<String>,

    /// Download URL
    pub url: Option<String>,
}

/// Custom field
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CipherField {
    /// Encrypted field name
    pub name: String,

    /// Encrypted field value
    pub value: Option<String>,

    /// Field type: 0=Text, 1=Hidden, 2=Boolean
    #[serde(rename = "type")]
    pub field_type: u8,
}

/// Password history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PasswordHistory {
    /// Encrypted password
    pub password: String,

    /// Last used date (ISO 8601)
    pub last_used_date: String,
}

/// Decrypted cipher view for display
///
/// Used after SDK decryption for list/get operations.
/// All fields are plain text (decrypted).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CipherView {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_id: Option<String>,
    pub folder_id: Option<String>,
    #[serde(rename = "type")]
    pub cipher_type: CipherType,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    pub favorite: bool,
    #[serde(default)]
    pub collection_ids: Vec<String>,
    pub revision_date: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creation_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted_date: Option<String>,

    // Decrypted type-specific data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub login: Option<CipherLoginView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secure_note: Option<CipherSecureNote>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card: Option<CipherCardView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identity: Option<CipherIdentityView>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<CipherAttachment>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<CipherFieldView>,
}

/// Decrypted login view
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CipherLoginView {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(default)]
    pub uris: Vec<CipherLoginUriView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub totp: Option<String>,
}

/// Decrypted URI view
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CipherLoginUriView {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    #[serde(rename = "match")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_type: Option<UriMatchType>,
}

/// Decrypted card view
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CipherCardView {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cardholder_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brand: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exp_month: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exp_year: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

/// Decrypted identity view
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CipherIdentityView {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub middle_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address2: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address3: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub postal_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssn: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passport_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license_number: Option<String>,
}

/// Decrypted field view
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CipherFieldView {
    pub name: String,
    pub value: Option<String>,
    #[serde(rename = "type")]
    pub field_type: u8,
}
