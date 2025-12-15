---
enhancement: 14-sdk-migration-vault
agent: architect
task_id: task_1765657124_66337
timestamp: 2025-12-13T12:45:00Z
status: READY_FOR_IMPLEMENTATION
---

# Implementation Plan: Vault Services SDK Migration

## Executive Summary

This plan outlines the migration of vault encryption/decryption services from the current custom implementation to the official Bitwarden SDK. The migration follows the established pattern from the TOTP service migration (enhancement #13) and leverages the `bitwarden-vault` crate's `VaultClientExt` trait.

## Architecture Overview

### Current State

The CLI currently has a custom implementation in `crates/bw-core/src/services/vault/`:
- `cipher_service.rs` - Custom cipher encryption/decryption
- `totp_service.rs` - Already migrated to SDK
- `write_service.rs` - Cipher create/edit operations
- `mod.rs` - Module organization

The custom implementation duplicates functionality that the SDK provides:
- Manual key derivation and management
- Custom encryption/decryption logic
- Custom cipher type conversion

### Target State

Leverage the SDK's vault operations through `VaultClientExt`:
- `client.vault().ciphers()` - CiphersClient for encrypt/decrypt
- `client.vault().folders()` - FoldersClient for folder operations
- `client.vault().collections()` - CollectionsClient for collection operations
- `client.vault().attachments()` - AttachmentsClient for attachment operations

### Key SDK Types

```rust
// bitwarden-vault crate types
bitwarden_vault::Cipher           // Encrypted cipher
bitwarden_vault::CipherView       // Decrypted cipher view
bitwarden_vault::CipherListView   // Lightweight list view
bitwarden_vault::EncryptionContext // Result of encryption with user ID
bitwarden_vault::Folder / FolderView
bitwarden_collections::Collection / CollectionView
```

## Implementation Phases

### Phase 1: SDK Client Initialization Enhancement

**Files to modify:**
- `crates/bw-core/src/services/sdk.rs`
- `crates/bw-core/src/services/key_service.rs`

**Tasks:**

1.1. **Create SDK Crypto Initialization Helper** (`key_service.rs`)
```rust
/// Initialize SDK client with user crypto state using decrypted user key
pub async fn initialize_sdk_crypto(
    client: &bitwarden_core::Client,
    user_key: &SymmetricCryptoKey,
    user_id: &str,
    private_key: &EncString,  // From stored account data
) -> Result<(), CryptoInitError> {
    // Use InitUserCryptoMethod::DecryptedKey
    client.crypto().initialize_user_crypto(InitUserCryptoRequest {
        user_id: Some(user_id.parse()?),
        kdf_params: Kdf::default(), // Not needed for DecryptedKey method
        email: String::new(),        // Not needed for DecryptedKey method
        private_key: private_key.clone(),
        signing_key: None,           // Optional
        security_state: None,        // Optional
        method: InitUserCryptoMethod::DecryptedKey {
            decrypted_user_key: user_key.to_base64(),
        },
    }).await?;
    Ok(())
}
```

1.2. **Store Required Account Data** (during login/unlock)
- Ensure `encrypted_private_key` is stored in account data
- Ensure `user_id` is available for crypto initialization

**Estimated effort:** 1 implementation task

---

### Phase 2: Cipher Service Migration

**Files to modify:**
- `crates/bw-core/src/services/vault/cipher_service.rs`

**SDK API Reference:**
```rust
// CiphersClient methods (from bitwarden-vault)
impl CiphersClient {
    pub fn encrypt(&self, cipher_view: CipherView) -> Result<EncryptionContext, EncryptError>;
    pub fn decrypt(&self, cipher: Cipher) -> Result<CipherView, DecryptError>;
    pub fn decrypt_list(&self, ciphers: Vec<Cipher>) -> Result<Vec<CipherListView>, DecryptError>;
    pub fn decrypt_list_with_failures(&self, ciphers: Vec<Cipher>) -> DecryptCipherListResult;
    pub fn move_to_organization(&self, cipher_view: CipherView, org_id: OrganizationId) -> Result<CipherView, CipherError>;
}
```

**Tasks:**

2.1. **Create SDK-based CipherService**
```rust
pub struct CipherService {
    client: Client,  // SDK Client
}

impl CipherService {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    /// Decrypt a single cipher to full view
    pub fn decrypt_cipher(&self, cipher: Cipher) -> Result<CipherView, VaultError> {
        self.client.vault().ciphers().decrypt(cipher)
            .map_err(Into::into)
    }

    /// Decrypt multiple ciphers to list views (efficient)
    pub fn decrypt_cipher_list(&self, ciphers: Vec<Cipher>) -> Result<Vec<CipherListView>, VaultError> {
        self.client.vault().ciphers().decrypt_list(ciphers)
            .map_err(Into::into)
    }

    /// Encrypt a cipher view for storage/API
    pub fn encrypt_cipher(&self, cipher_view: CipherView) -> Result<EncryptionContext, VaultError> {
        self.client.vault().ciphers().encrypt(cipher_view)
            .map_err(Into::into)
    }
}
```

2.2. **Update Cipher Type Conversions**
- Replace custom `Cipher` / `CipherView` types with SDK types
- Update `models/vault/cipher.rs` to re-export SDK types
- Add thin conversion layer if needed for CLI-specific fields

2.3. **Update Existing Cipher Operations**
- `list` command: Use `decrypt_cipher_list()` for efficient listing
- `get` command: Use `decrypt_cipher()` for full cipher details
- Filter/search operations: Work with `CipherListView`

**Estimated effort:** 2 implementation tasks

---

### Phase 3: Folder Service Migration

**Files to modify:**
- `crates/bw-core/src/services/vault/folder_service.rs` (create or modify)

**SDK API Reference:**
```rust
// FoldersClient methods (from bitwarden-vault)
impl FoldersClient {
    pub fn encrypt(&self, folder_view: FolderView) -> Result<Folder, EncryptError>;
    pub fn decrypt(&self, folder: Folder) -> Result<FolderView, DecryptError>;
    pub fn decrypt_list(&self, folders: Vec<Folder>) -> Result<Vec<FolderView>, DecryptError>;
    pub async fn list(&self) -> Result<Vec<FolderView>, GetFolderError>;  // From state
    pub async fn get(&self, folder_id: FolderId) -> Result<FolderView, GetFolderError>;
    pub async fn create(&self, request: FolderAddEditRequest) -> Result<FolderView, CreateFolderError>;
    pub async fn edit(&self, folder_id: FolderId, request: FolderAddEditRequest) -> Result<FolderView, EditFolderError>;
}
```

**Tasks:**

3.1. **Create SDK-based FolderService**
```rust
pub struct FolderService {
    client: Client,
}

impl FolderService {
    pub fn decrypt_folders(&self, folders: Vec<Folder>) -> Result<Vec<FolderView>, VaultError> {
        self.client.vault().folders().decrypt_list(folders)
            .map_err(Into::into)
    }

    pub fn encrypt_folder(&self, folder_view: FolderView) -> Result<Folder, VaultError> {
        self.client.vault().folders().encrypt(folder_view)
            .map_err(Into::into)
    }
}
```

3.2. **Update Folder Type Exports**
- Re-export SDK types from `models/vault/`

**Estimated effort:** 1 implementation task

---

### Phase 4: Collection Service Migration

**Files to modify:**
- `crates/bw-core/src/services/vault/collection_service.rs` (create or modify)

**SDK API Reference:**
```rust
// CollectionsClient methods (from bitwarden-vault)
impl CollectionsClient {
    pub fn decrypt(&self, collection: Collection) -> Result<CollectionView, DecryptError>;
    pub fn decrypt_list(&self, collections: Vec<Collection>) -> Result<Vec<CollectionView>, DecryptError>;
    pub fn get_collection_tree(&self, collections: Vec<CollectionView>) -> CollectionViewTree;
}
```

**Tasks:**

4.1. **Create SDK-based CollectionService**
```rust
pub struct CollectionService {
    client: Client,
}

impl CollectionService {
    pub fn decrypt_collections(&self, collections: Vec<Collection>) -> Result<Vec<CollectionView>, VaultError> {
        self.client.vault().collections().decrypt_list(collections)
            .map_err(Into::into)
    }

    pub fn get_collection_tree(&self, collections: Vec<CollectionView>) -> CollectionViewTree {
        self.client.vault().collections().get_collection_tree(collections)
    }
}
```

**Estimated effort:** 1 implementation task

---

### Phase 5: Write Service Migration

**Files to modify:**
- `crates/bw-core/src/services/vault/write_service.rs`

**Tasks:**

5.1. **Update VaultWriteService to Use SDK Encryption**

Current signature pattern:
```rust
pub async fn create_cipher(&self, cipher_view: CipherView) -> Result<Cipher, VaultWriteError>;
pub async fn update_cipher(&self, cipher_view: CipherView) -> Result<Cipher, VaultWriteError>;
```

Updated implementation:
```rust
pub async fn create_cipher(&self, cipher_view: CipherView) -> Result<Cipher, VaultWriteError> {
    // Encrypt using SDK
    let encryption_context = self.client.vault().ciphers().encrypt(cipher_view)?;

    // Convert to API request model
    let request: CipherRequestModel = encryption_context.into();

    // Call API
    let response = self.api.create_cipher(request).await?;

    // Parse response
    Ok(Cipher::try_from(response)?)
}
```

5.2. **Update Move to Organization**
```rust
pub async fn move_cipher_to_organization(
    &self,
    cipher_view: CipherView,
    organization_id: OrganizationId,
) -> Result<CipherView, VaultWriteError> {
    self.client.vault().ciphers()
        .move_to_organization(cipher_view, organization_id)
        .map_err(Into::into)
}
```

**Estimated effort:** 1 implementation task

---

### Phase 6: CLI Command Updates

**Files to modify:**
- Commands in `src/commands/vault/` that use cipher operations

**Tasks:**

6.1. **Update List Commands**
- Use `CipherListView` for display (more efficient)
- Only fetch full `CipherView` when needed for detailed operations

6.2. **Update Create/Edit Commands**
- Ensure proper SDK encryption flow
- Handle `EncryptionContext` for API requests

6.3. **Update Get Commands**
- Use full decryption for detailed cipher view

**Estimated effort:** 1 implementation task

---

### Phase 7: Cleanup and Testing

**Tasks:**

7.1. **Remove Deprecated Code**
- Remove custom encryption/decryption logic that's now in SDK
- Remove duplicate type definitions
- Update module exports

7.2. **Update Tests**
- Update unit tests to use SDK types
- Add integration tests for SDK cipher operations
- Test encryption/decryption round-trips

7.3. **Documentation Updates**
- Update inline documentation
- Update `/docs` as needed

**Estimated effort:** 1 implementation task

---

## Dependency Graph

```
Phase 1 (SDK Crypto Init)
    │
    ├──► Phase 2 (Cipher Service)
    │         │
    │         └──► Phase 5 (Write Service)
    │                   │
    │                   └──► Phase 6 (CLI Commands)
    │
    ├──► Phase 3 (Folder Service)
    │
    └──► Phase 4 (Collection Service)
                │
                └──► Phase 7 (Cleanup & Testing)
```

## Risk Assessment

### Low Risk
- SDK types are well-tested and production-ready
- Similar migration pattern already successful (TOTP)
- SDK provides comprehensive error handling

### Medium Risk
- Need to ensure `encrypted_private_key` is available during unlock
- Need to handle organization keys for shared ciphers
- Potential breaking changes in CLI output formats

### Mitigation
- Add backward compatibility layer for type conversions if needed
- Comprehensive testing before removing old code
- Feature flag for gradual rollout (optional)

## Testing Strategy

1. **Unit Tests**
   - Test each service method with mock SDK client
   - Test error handling paths

2. **Integration Tests**
   - Test full encrypt/decrypt round-trips
   - Test with real vault data
   - Test organization cipher operations

3. **Regression Tests**
   - Ensure CLI output format unchanged
   - Verify existing commands work correctly

## File Changes Summary

| File | Change Type | Description |
|------|-------------|-------------|
| `services/sdk.rs` | Modify | Add crypto initialization helper |
| `services/key_service.rs` | Modify | Add SDK crypto init integration |
| `services/vault/cipher_service.rs` | Modify | Replace with SDK-based implementation |
| `services/vault/folder_service.rs` | Create/Modify | Add SDK-based folder operations |
| `services/vault/collection_service.rs` | Create/Modify | Add SDK-based collection operations |
| `services/vault/write_service.rs` | Modify | Use SDK encryption for write ops |
| `models/vault/cipher.rs` | Modify | Re-export SDK types |
| `models/vault/folder.rs` | Modify | Re-export SDK types |
| `models/vault/collection.rs` | Modify | Re-export SDK types |
| `commands/vault/*.rs` | Modify | Update to use new service methods |

## Implementation Order

1. Phase 1: SDK Client Initialization Enhancement
2. Phase 2: Cipher Service Migration
3. Phase 3: Folder Service Migration
4. Phase 4: Collection Service Migration
5. Phase 5: Write Service Migration
6. Phase 6: CLI Command Updates
7. Phase 7: Cleanup and Testing

## Estimated Total Effort

- **Implementation Tasks:** 8
- **Test Tasks:** 2-3
- **Documentation:** 1

Total: ~11-12 discrete implementation tasks

## Notes on SDK Integration Pattern

The SDK uses an extension trait pattern (`VaultClientExt`) that provides clean access to vault operations:

```rust
use bitwarden_vault::VaultClientExt;

// Access vault operations
let cipher = client.vault().ciphers().decrypt(encrypted_cipher)?;
let folder = client.vault().folders().decrypt(encrypted_folder)?;
let collection = client.vault().collections().decrypt(encrypted_collection)?;
```

Key observations:
1. The SDK `Client` must have crypto initialized before vault operations
2. Organization keys must be loaded for org-owned ciphers
3. `EncryptionContext` includes both the cipher and `encrypted_for` user ID
4. `CipherListView` is more efficient for list operations than full `CipherView`
