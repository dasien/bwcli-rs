---
slug: sdk-migration-vault
status: NEW
created: 2024-12-13
author: Migration Team
priority: high
---

# Enhancement: SDK Migration - Vault (Direct SDK Types)

## Overview

**Goal:** Replace CLI's custom vault types and encryption code with DIRECT usage of SDK types from `bitwarden-vault`. This is NOT about creating bridges or adapters - it's about DELETING duplicate code and using SDK types throughout.

**CRITICAL ANTI-PATTERN TO AVOID:**
Do NOT create "bridge" classes, adapter layers, or type conversion utilities. The previous attempt created an `SdkVaultBridge` that translated between CLI types and SDK types - this defeats the entire purpose. We want to USE SDK types directly, not wrap them.

## What We're Replacing

### CLI Types to DELETE (in `crates/bw-core/src/models/vault/cipher.rs`)
- `Cipher` → Use `bitwarden_vault::Cipher`
- `CipherView` → Use `bitwarden_vault::CipherView`
- `CipherType` → Use `bitwarden_vault::CipherType`
- `CipherLogin`, `CipherCard`, `CipherIdentity`, etc. → Use SDK equivalents
- `CipherLoginView`, `CipherCardView`, `CipherIdentityView`, etc. → Use SDK equivalents

### CLI Types to DELETE (in `crates/bw-core/src/models/vault/folder.rs`)
- `Folder` → Use `bitwarden_vault::Folder`
- `FolderView` → Use `bitwarden_vault::FolderView`

### CLI Types to DELETE (in `crates/bw-core/src/models/vault/collection.rs`)
- `Collection` → Use `bitwarden_collections::Collection`
- `CollectionView` → Use `bitwarden_collections::CollectionView`

### CLI Service to SIMPLIFY (in `crates/bw-core/src/services/vault/cipher_service.rs`)
The current `CipherService` has 418 lines of manual field-by-field encryption/decryption. Replace with:
```rust
// Instead of manual decryption:
client.vault().ciphers().decrypt(cipher)

// Instead of manual encryption:
client.vault().ciphers().encrypt(cipher_view)
```

## SDK Types Reference

### Key Imports
```rust
// Vault types
use bitwarden_vault::{
    Cipher, CipherView, CipherListView, CipherType, CipherRepromptType,
    Folder, FolderView, FolderId,
    Login, LoginView, Card, CardView, Identity, IdentityView,
    SecureNote, SecureNoteView, SshKey, SshKeyView,
    Field, FieldView, FieldType,
    Attachment, AttachmentView,
    PasswordHistory, PasswordHistoryView,
    LoginUri, LoginUriView, UriMatchType,
    VaultClientExt,  // Extension trait for client.vault()
};
use bitwarden_vault::CipherId;

// Collection types
use bitwarden_collections::collection::{Collection, CollectionView, CollectionId};

// Core types
use bitwarden_core::{Client, OrganizationId, UserId};
use bitwarden_core::key_management::{
    SymmetricKeyId,
    crypto::{InitUserCryptoRequest, InitUserCryptoMethod},
};

// Crypto
use bitwarden_crypto::{EncString, Kdf};
```

### SDK Type Characteristics
- IDs are UUID newtypes: `CipherId`, `FolderId`, `OrganizationId`, `CollectionId`
- Encrypted fields use `EncString` type (not raw `String`)
- Dates use `DateTime<Utc>` (not `String`)
- All use `#[serde(rename_all = "camelCase")]` - compatible with API responses
- SDK types implement `Serialize`/`Deserialize` - can be stored directly

## Implementation Steps

### Step 1: Update Cargo.toml Dependencies
Ensure these are in `crates/bw-core/Cargo.toml`:
```toml
bitwarden-vault = { workspace = true }
bitwarden-collections = { workspace = true }
bitwarden-core = { workspace = true, features = ["internal"] }
bitwarden-crypto = { workspace = true }
```

### Step 2: Update Model Re-exports
In `crates/bw-core/src/models/vault/mod.rs`, change from defining types to re-exporting SDK types:

```rust
// BEFORE (custom types):
mod cipher;
pub use cipher::*;

// AFTER (SDK re-exports):
pub use bitwarden_vault::{
    Cipher, CipherView, CipherListView, CipherType, CipherRepromptType,
    Login, LoginView, LoginUri, LoginUriView, UriMatchType,
    Card, CardView, Identity, IdentityView,
    SecureNote, SecureNoteView, SshKey, SshKeyView,
    Field, FieldView, FieldType,
    Attachment, AttachmentView,
    PasswordHistory, PasswordHistoryView,
    Folder, FolderView, FolderId,
    CipherId,
};
pub use bitwarden_collections::collection::{Collection, CollectionView, CollectionId};
pub use bitwarden_core::OrganizationId;

// Keep Organization type if SDK doesn't have it
mod organization;
pub use organization::Organization;
```

### Step 3: Update Storage Layer
The sync service stores ciphers as `HashMap<String, Cipher>`. Since SDK types serialize the same way (camelCase), this should work with SDK types directly.

In `crates/bw-core/src/services/vault/sync_service.rs`:
```rust
// The HashMap key stays as String (the cipher ID as string)
// But the value type changes to SDK Cipher
use bitwarden_vault::Cipher;

let ciphers_map: HashMap<String, Cipher> = sync_response
    .ciphers
    .into_iter()
    .map(|c| (c.id.map(|id| id.to_string()).unwrap_or_default(), c))
    .collect();
```

### Step 4: Initialize SDK Crypto
The SDK's `VaultClient` requires crypto to be initialized. In `VaultService`:

```rust
use bitwarden_core::Client;
use bitwarden_core::key_management::crypto::{InitUserCryptoRequest, InitUserCryptoMethod};
use bitwarden_crypto::Kdf;

impl VaultService {
    /// Initialize SDK crypto with the user's key
    pub async fn initialize_crypto(
        &self,
        client: &Client,
        user_key: &SymmetricCryptoKey,
        email: &str,
        kdf: Kdf,
        private_key: Option<EncString>,
    ) -> Result<(), VaultError> {
        let request = InitUserCryptoRequest {
            user_id: None,
            kdf_params: kdf,
            email: email.to_string(),
            private_key: private_key.unwrap_or_else(|| "2.dummy|dummy|dummy".parse().unwrap()),
            signing_key: None,
            security_state: None,
            method: InitUserCryptoMethod::DecryptedKey {
                decrypted_user_key: user_key.to_base64().to_string(),
            },
        };

        client.crypto().initialize_user_crypto(request).await
            .map_err(|e| VaultError::CryptoInitFailed(e.to_string()))
    }
}
```

### Step 5: Replace CipherService with VaultClient
Delete the manual encryption/decryption in `cipher_service.rs`. Replace with:

```rust
use bitwarden_vault::VaultClientExt;

pub struct CipherService {
    client: Arc<Client>,
}

impl CipherService {
    pub fn decrypt_cipher(&self, cipher: Cipher) -> Result<CipherView, VaultError> {
        self.client
            .vault()
            .ciphers()
            .decrypt(cipher)
            .map_err(|e| VaultError::DecryptionError(e.to_string()))
    }

    pub fn decrypt_ciphers(&self, ciphers: Vec<Cipher>) -> Result<Vec<CipherListView>, VaultError> {
        self.client
            .vault()
            .ciphers()
            .decrypt_list(ciphers)
            .map_err(|e| VaultError::DecryptionError(e.to_string()))
    }

    pub fn encrypt_cipher(&self, cipher_view: CipherView) -> Result<Cipher, VaultError> {
        let ctx = self.client
            .vault()
            .ciphers()
            .encrypt(cipher_view)
            .map_err(|e| VaultError::EncryptionError(e.to_string()))?;
        Ok(ctx.cipher)
    }

    pub fn decrypt_folder(&self, folder: Folder) -> Result<FolderView, VaultError> {
        self.client
            .vault()
            .folders()
            .decrypt(folder)
            .map_err(|e| VaultError::DecryptionError(e.to_string()))
    }

    pub fn decrypt_folders(&self, folders: Vec<Folder>) -> Result<Vec<FolderView>, VaultError> {
        self.client
            .vault()
            .folders()
            .decrypt_list(folders)
            .map_err(|e| VaultError::DecryptionError(e.to_string()))
    }

    pub fn decrypt_collection(&self, collection: Collection) -> Result<CollectionView, VaultError> {
        self.client
            .vault()
            .collections()
            .decrypt(collection)
            .map_err(|e| VaultError::DecryptionError(e.to_string()))
    }
}
```

### Step 6: Update VaultService
Remove the `user_key` parameter from methods - the SDK client manages keys internally once initialized.

```rust
// BEFORE:
pub async fn list_items(&self, filters: &ItemFilters, session: &str) -> Result<Vec<CipherView>, VaultError> {
    let user_key = self.get_user_key(session).await?;
    let ciphers = self.get_ciphers().await?;
    self.cipher_service.decrypt_ciphers(&ciphers, &user_key).await
}

// AFTER:
pub async fn list_items(&self, filters: &ItemFilters) -> Result<Vec<CipherListView>, VaultError> {
    // Crypto must be initialized before calling this
    let ciphers = self.get_ciphers().await?;
    let filtered = self.search_service.filter_ciphers(&ciphers, filters);
    self.cipher_service.decrypt_ciphers(filtered.into_values().collect())
}
```

### Step 7: Handle ID Type Changes
SDK uses UUID newtypes. For storage keys and API compatibility:

```rust
// Getting ID as string for HashMap key:
let id_string = cipher.id.map(|id| id.to_string()).unwrap_or_default();

// Parsing string to CipherId:
let cipher_id: CipherId = id_string.parse().map_err(|_| VaultError::InvalidId)?;
```

### Step 8: Delete Unused Files
After migration, delete:
- `crates/bw-core/src/models/vault/cipher.rs` (612 lines)
- Most of `crates/bw-core/src/services/vault/cipher_service.rs` (reduce from 418 to ~50 lines)

## Files to Modify

| File | Action |
|------|--------|
| `crates/bw-core/Cargo.toml` | Add `bitwarden-collections` if missing |
| `crates/bw-core/src/models/vault/mod.rs` | Re-export SDK types instead of custom types |
| `crates/bw-core/src/models/vault/cipher.rs` | DELETE entirely |
| `crates/bw-core/src/models/vault/folder.rs` | DELETE or reduce to re-exports |
| `crates/bw-core/src/models/vault/collection.rs` | DELETE or reduce to re-exports |
| `crates/bw-core/src/services/vault/cipher_service.rs` | Replace with SDK VaultClient calls |
| `crates/bw-core/src/services/vault/mod.rs` | Update VaultService to use SDK crypto |
| `crates/bw-core/src/services/vault/sync_service.rs` | Use SDK Cipher type |
| `crates/bw-core/src/services/vault/write_service.rs` | Use SDK types |
| `crates/bw-core/src/services/vault/search_service.rs` | Update for SDK types |
| `crates/bw-cli/src/commands/vault.rs` | Update for SDK types |

## Success Criteria

1. **Build passes** with no errors
2. **SDK types used directly** - grep should find `bitwarden_vault::Cipher` in vault code
3. **No bridge/adapter classes** - no `SdkVaultBridge` or similar
4. **Custom cipher.rs deleted** - file should not exist or be drastically reduced
5. **VaultClient methods used** - `client.vault().ciphers().decrypt()` in use
6. **CLI commands work** - `bw list items`, `bw get item`, etc. function correctly
7. **Code reduction** - Net reduction of 400+ lines (not addition!)
8. **Unit tests pass** - All existing tests still pass

## What NOT to Do

1. **DO NOT** create bridge classes or adapter layers
2. **DO NOT** keep both CLI types and SDK types
3. **DO NOT** create type conversion functions (except for ID string conversion)
4. **DO NOT** add more code than you remove
5. **DO NOT** wrap SDK methods in unnecessary abstractions

## Testing

After implementation:
```bash
# Build
cargo build

# Run tests
cargo test --lib

# Test CLI commands
./target/debug/bw unlock '<password>'
export BW_SESSION="<session>"
./target/debug/bw list items --session "$BW_SESSION"
./target/debug/bw get item <id> --session "$BW_SESSION"
./target/debug/bw list folders --session "$BW_SESSION"
```
