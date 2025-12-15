# SDK Type Mapping Reference

This document provides a detailed mapping between current CLI types and their SDK equivalents.

## Cipher Types

| Current CLI Type | SDK Type | Notes |
|------------------|----------|-------|
| `models::Cipher` | `bitwarden_vault::Cipher` | Encrypted cipher data |
| `models::CipherView` | `bitwarden_vault::CipherView` | Decrypted cipher for editing |
| N/A | `bitwarden_vault::CipherListView` | Lightweight view for lists |
| `CipherType` | `bitwarden_vault::CipherType` | Login, SecureNote, Card, Identity, SshKey |

### CipherView Fields Comparison

```rust
// SDK CipherView structure
pub struct CipherView {
    pub id: Option<CipherId>,
    pub organization_id: Option<OrganizationId>,
    pub folder_id: Option<FolderId>,
    pub collection_ids: Vec<CollectionId>,
    pub key: Option<EncString>,          // Cipher-level encryption key
    pub name: String,
    pub notes: Option<String>,
    pub r#type: CipherType,
    pub login: Option<LoginView>,
    pub identity: Option<IdentityView>,
    pub card: Option<CardView>,
    pub secure_note: Option<SecureNoteView>,
    pub ssh_key: Option<SshKeyView>,
    pub favorite: bool,
    pub reprompt: CipherRepromptType,
    pub organization_use_totp: bool,
    pub edit: bool,
    pub permissions: Option<CipherPermissions>,
    pub view_password: bool,
    pub local_data: Option<LocalDataView>,
    pub attachments: Option<Vec<AttachmentView>>,
    pub fields: Option<Vec<FieldView>>,
    pub password_history: Option<Vec<PasswordHistoryView>>,
    pub creation_date: DateTime<Utc>,
    pub deleted_date: Option<DateTime<Utc>>,
    pub revision_date: DateTime<Utc>,
    pub archived_date: Option<DateTime<Utc>>,
}
```

### CipherListView (for efficient listing)

```rust
pub struct CipherListView {
    pub id: Option<CipherId>,
    pub organization_id: Option<OrganizationId>,
    pub folder_id: Option<FolderId>,
    pub collection_ids: Vec<CollectionId>,
    pub key: Option<EncString>,
    pub name: String,
    pub subtitle: String,              // Pre-computed subtitle
    pub r#type: CipherListViewType,    // Simplified type info
    pub favorite: bool,
    pub reprompt: CipherRepromptType,
    pub organization_use_totp: bool,
    pub edit: bool,
    pub permissions: Option<CipherPermissions>,
    pub view_password: bool,
    pub attachments: u32,              // Count only
    pub has_old_attachments: bool,
    pub creation_date: DateTime<Utc>,
    pub deleted_date: Option<DateTime<Utc>>,
    pub revision_date: DateTime<Utc>,
    pub archived_date: Option<DateTime<Utc>>,
    pub copyable_fields: Vec<CopyableCipherFields>,
    pub local_data: Option<LocalDataView>,
}
```

## Folder Types

| Current CLI Type | SDK Type | Notes |
|------------------|----------|-------|
| `models::Folder` | `bitwarden_vault::Folder` | Encrypted folder |
| `models::FolderView` | `bitwarden_vault::FolderView` | Decrypted folder |

```rust
// SDK Folder structure
pub struct Folder {
    pub id: Option<FolderId>,
    pub name: EncString,
    pub revision_date: DateTime<Utc>,
}

pub struct FolderView {
    pub id: Option<FolderId>,
    pub name: String,
    pub revision_date: DateTime<Utc>,
}
```

## Collection Types

| Current CLI Type | SDK Type | Notes |
|------------------|----------|-------|
| `models::Collection` | `bitwarden_collections::Collection` | Encrypted collection |
| `models::CollectionView` | `bitwarden_collections::CollectionView` | Decrypted collection |

```rust
// SDK Collection structure (from bitwarden-collections crate)
pub struct Collection {
    pub id: Option<CollectionId>,
    pub organization_id: OrganizationId,
    pub name: EncString,
    pub external_id: Option<String>,
    pub hide_passwords: bool,
    pub read_only: bool,
    pub manage: bool,
    pub default_user_collection_email: Option<String>,
    pub r#type: CollectionType,
}

pub struct CollectionView {
    pub id: Option<CollectionId>,
    pub organization_id: OrganizationId,
    pub name: String,
    pub external_id: Option<String>,
    pub hide_passwords: bool,
    pub read_only: bool,
    pub manage: bool,
    pub default_user_collection_email: Option<String>,
    pub r#type: CollectionType,
}
```

## Encryption Context

The SDK returns an `EncryptionContext` when encrypting ciphers:

```rust
pub struct EncryptionContext {
    /// The Id of the user that encrypted the cipher
    pub encrypted_for: UserId,
    pub cipher: Cipher,
}
```

This is important for API requests where the server needs to know who encrypted the data.

## API Model Conversions

SDK types implement `TryFrom` for API models:

```rust
// For creating/updating ciphers
impl From<EncryptionContext> for CipherRequestModel { ... }
impl TryFrom<EncryptionContext> for CipherWithIdRequestModel { ... }

// For parsing API responses
impl TryFrom<CipherDetailsResponseModel> for Cipher { ... }
impl TryFrom<CipherResponseModel> for Cipher { ... }
```

## Key IDs and Key Store

The SDK uses a key store pattern with typed key identifiers:

```rust
pub enum SymmetricKeyId {
    User,                           // User's master key
    Organization(OrganizationId),   // Org key
    // ... other key types
}

// Ciphers identify their key based on ownership
impl IdentifyKey<SymmetricKeyId> for Cipher {
    fn key_identifier(&self) -> SymmetricKeyId {
        match self.organization_id {
            Some(org_id) => SymmetricKeyId::Organization(org_id),
            None => SymmetricKeyId::User,
        }
    }
}
```

## Migration Helpers

Suggested helper functions for smooth migration:

```rust
// In models/vault/cipher.rs
pub use bitwarden_vault::{
    Cipher, CipherView, CipherListView, CipherType, CipherRepromptType,
    EncryptionContext, Login, LoginView, Card, CardView,
    Identity, IdentityView, SecureNote, SecureNoteView, SshKey, SshKeyView,
};

// In models/vault/folder.rs
pub use bitwarden_vault::{Folder, FolderView, FolderId};

// In models/vault/collection.rs
pub use bitwarden_collections::{Collection, CollectionView, CollectionId};
```
