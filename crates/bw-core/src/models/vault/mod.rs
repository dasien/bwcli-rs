//! Vault data models
//!
//! This module re-exports vault types from the Bitwarden SDK for use throughout the CLI.
//! Custom types are only defined where the SDK doesn't provide suitable types.

mod organization;
mod sync_response;
mod validation_error;

// Re-export SDK vault types (only those actually exported from bitwarden-vault)
pub use bitwarden_vault::{
    // Core cipher types
    Cipher, CipherId, CipherListView, CipherRepromptType, CipherType, CipherView,
    // Login types
    Login, LoginView, LoginUriView, LoginListView, UriMatchType,
    // Card types
    CardView, CardListView, CardBrand,
    // Identity types
    IdentityView,
    // Secure note types
    SecureNoteType, SecureNoteView,
    // SSH key types
    SshKeyView,
    // Folder types
    Folder, FolderId, FolderView,
    // Attachment types
    Attachment, AttachmentView,
    // Field types
    FieldType, FieldView,
    // Password history
    PasswordHistory, PasswordHistoryView,
    // FIDO2
    Fido2Credential, Fido2CredentialView, Fido2CredentialFullView,
    // Errors
    VaultParseError, DecryptError, EncryptError,
    // VaultClient extension
    VaultClientExt,
    // Encryption context
    EncryptionContext,
};

// Re-export SDK collection types from bitwarden-collections
pub use bitwarden_collections::collection::{Collection, CollectionId, CollectionView, CollectionType};

// Re-export OrganizationId from bitwarden-core
pub use bitwarden_core::OrganizationId;

// CLI-specific types
pub use organization::*;
pub use sync_response::{parse_sync_response, SyncData, VaultData};
pub use validation_error::*;

// Re-export SDK API models for API requests/responses
pub use bitwarden_api_api::models::{
    SyncResponseModel,
    CipherRequestModel,
    FolderRequestModel,
};
