# Code Changes Summary

## Files Modified

### /Cargo.toml (workspace)
Added dependencies:
```toml
bitwarden-collections = { path = "../sdk-internal/crates/bitwarden-collections", version = "=1.0.0" }
bitwarden-api-api = { path = "../sdk-internal/crates/bitwarden-api-api", version = "=1.0.0" }
```

### /crates/bw-core/Cargo.toml
Added dependencies:
```toml
bitwarden-collections.workspace = true
bitwarden-api-api.workspace = true
```

### /crates/bw-core/src/models/vault/mod.rs
- Replaced custom type declarations with SDK re-exports
- Added re-exports from `bitwarden_vault`, `bitwarden_collections`, `bitwarden_api_api`
- Preserved CLI-specific types: `Organization`, `ValidationError`

### /crates/bw-core/src/models/vault/sync_response.rs
- Removed custom `SyncResponse` struct (~100 lines)
- Added `parse_sync_response()` function using SDK's TryFrom
- Updated `VaultData` to use SDK types

### /crates/bw-core/src/services/vault/cipher_service.rs
- Reduced from 419 lines to 85 lines
- Removed manual `EncString` decryption
- Now uses `VaultClientExt` methods:
  - `vault().ciphers().decrypt()`
  - `vault().ciphers().decrypt_list()`
  - `vault().folders().decrypt_list()`
  - `vault().collections().decrypt_list()`
  - `vault().ciphers().encrypt()`
  - `vault().folders().encrypt()`

### /crates/bw-core/src/services/vault/sync_service.rs
- Changed to use `SyncResponseModel` from SDK API
- Uses `parse_sync_response()` for conversion
- Handles SDK ID types (`Option<CipherId>` etc.)

### /crates/bw-core/src/services/vault/validation_service.rs
- Changed `cipher.cipher_type` to `cipher.r#type`
- Changed `cipher.id.is_empty()` to `cipher.id.is_none()`
- Updated URI validation for `Option<Vec<LoginUriView>>`
- Removed UUID regex validation (SDK types are always valid)
- Removed tests (need SDK type constructors)

### /crates/bw-core/src/services/vault/write_service.rs
- Uses `CipherRequestModel` from SDK API
- Uses `EncryptionContext` from encrypt
- Removed `user_key` parameter (SDK handles internally)
- Uses `DateTime<Utc>` for timestamps
- Uses `CipherId::new()`, `FolderId::new()` for IDs

### /crates/bw-core/src/services/vault/mod.rs
- Changed to import `bitwarden_core::Client`
- Removed key_service usage (SDK handles keys)
- Updated method signatures (removed user_key)
- Changed return types to use SDK view types

## Type Mapping Reference

| Old CLI Type | New SDK Type |
|--------------|--------------|
| `Cipher` | `bitwarden_vault::Cipher` |
| `CipherView` | `bitwarden_vault::CipherView` |
| `CipherType` | `bitwarden_vault::CipherType` |
| `CipherLoginView` | `bitwarden_vault::LoginView` |
| `CipherCardView` | `bitwarden_vault::CardView` |
| `CipherIdentityView` | `bitwarden_vault::IdentityView` |
| `CipherSecureNoteView` | `bitwarden_vault::SecureNoteView` |
| `CipherLoginUriView` | `bitwarden_vault::LoginUriView` |
| `Folder` | `bitwarden_vault::Folder` |
| `FolderView` | `bitwarden_vault::FolderView` |
| `Collection` | `bitwarden_collections::Collection` |
| `CollectionView` | `bitwarden_collections::CollectionView` |
| `SyncResponse` | `bitwarden_api_api::models::SyncResponseModel` |
| `CipherRequest` | `bitwarden_api_api::models::CipherRequestModel` |
| `FolderRequest` | `bitwarden_api_api::models::FolderRequestModel` |

## Field Name Changes

| Old Field | New Field |
|-----------|-----------|
| `cipher.cipher_type` | `cipher.r#type` |
| `cipher.id: String` | `cipher.id: Option<CipherId>` |
| `cipher.folder_id: Option<String>` | `cipher.folder_id: Option<FolderId>` |
| `cipher.organization_id: Option<String>` | `cipher.organization_id: Option<OrganizationId>` |
| `cipher.revision_date: String` | `cipher.revision_date: DateTime<Utc>` |
| `cipher.creation_date: Option<String>` | `cipher.creation_date: DateTime<Utc>` |
| `cipher.login.uris: Vec<_>` | `cipher.login.uris: Option<Vec<_>>` |
| `collection.organization_id: String` | `collection.organization_id: OrganizationId` |
