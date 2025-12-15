---
enhancement: 14a-sdk-migration-vault
agent: architect
task_id: task_1765666701_99547
timestamp: 2025-12-13T21:15:00Z
status: READY_FOR_IMPLEMENTATION
---

# Implementation Plan: SDK Migration - Vault (Direct SDK Types)

## 1. Executive Summary

This implementation plan details how to replace CLI's custom vault types with direct SDK types from `bitwarden-vault` and `bitwarden-collections`. The key principle is **code deletion and simplification** - no adapter layers or bridges.

### Verification Complete

| Requirement | Status | Notes |
|-------------|--------|-------|
| SDK Cipher types exported | ✅ Verified | `bitwarden_vault::{Cipher, CipherView, CipherListView, CipherType, ...}` |
| SDK Folder types exported | ✅ Verified | `bitwarden_vault::{Folder, FolderView, FolderId}` |
| SDK Collection types exported | ✅ Verified | `bitwarden_collections::collection::{Collection, CollectionView, CollectionId}` |
| VaultClient extension trait | ✅ Verified | `bitwarden_vault::{VaultClient, VaultClientExt}` |
| CiphersClient methods | ✅ Verified | `encrypt()`, `decrypt()`, `decrypt_list()` available |
| JSON serialization compatible | ✅ Verified | SDK uses `#[serde(rename_all = "camelCase")]` - matches CLI format |

### Key Differences Found

| Field | CLI Type | SDK Type | Impact |
|-------|----------|----------|--------|
| IDs | `String` | UUID newtypes (`CipherId`, `FolderId`, `CollectionId`) | Storage key conversion needed |
| Dates | `String` (ISO 8601) | `DateTime<Utc>` | Serialization compatible |
| Encrypted fields | `String` | `EncString` | Serialization compatible |
| Collection has more fields | 5 fields | 9 fields (`hide_passwords`, `manage`, `type`, etc.) | May need defaults for storage |

## 2. Architecture Overview

### 2.1 Current Architecture (To Be Deleted)

```
CLI Custom Types                    Manual Crypto
─────────────────                   ─────────────
models/vault/cipher.rs (612 lines)  CipherService.decrypt_cipher()
models/vault/folder.rs (27 lines)   CipherService.encrypt_cipher()
models/vault/collection.rs (39 lines)
                    ↓
            Direct EncString operations via bitwarden-crypto
```

### 2.2 Target Architecture

```
SDK Types (Re-exported)             SDK VaultClient
───────────────────────             ────────────────
bitwarden_vault::Cipher             client.vault().ciphers().decrypt()
bitwarden_vault::Folder             client.vault().ciphers().encrypt()
bitwarden_collections::Collection   client.vault().folders().decrypt_list()
                    ↓
            SDK-managed crypto context (initialized once)
```

### 2.3 Integration Pattern

```rust
// Initialize SDK crypto once at unlock
client.crypto().initialize_user_crypto(InitUserCryptoRequest {
    kdf_params,
    email,
    private_key,
    signing_key: None,
    security_state: None,
    method: InitUserCryptoMethod::DecryptedKey {
        decrypted_user_key: user_key_base64,
    },
}).await?;

// All subsequent operations use SDK VaultClient
let cipher_view = client.vault().ciphers().decrypt(cipher)?;
let encrypted = client.vault().ciphers().encrypt(cipher_view)?;
```

## 3. Implementation Phases

### Phase 1: Add Dependencies and Re-exports (Low Risk)

**Goal:** Add `bitwarden-collections` dependency and create re-export module.

**Files to Modify:**

1. **`Cargo.toml` (workspace)**
   ```toml
   bitwarden-collections = { path = "../sdk-internal/crates/bitwarden-collections", version = "=1.0.0" }
   ```

2. **`crates/bw-core/Cargo.toml`**
   ```toml
   [dependencies]
   bitwarden-collections = { workspace = true }
   ```

3. **`crates/bw-core/src/models/vault/mod.rs`** - Create SDK re-exports
   ```rust
   //! Vault data models - re-exported from SDK

   // Re-export SDK vault types
   pub use bitwarden_vault::{
       // Cipher types
       Cipher, CipherView, CipherListView, CipherType, CipherRepromptType, CipherId,
       // Cipher sub-types
       Login, LoginView, LoginUriView, UriMatchType,
       CardView, IdentityView, SecureNoteView, SecureNoteType, SshKeyView,
       FieldView, FieldType,
       AttachmentView, Attachment,
       PasswordHistory, PasswordHistoryView,
       // Folder types
       Folder, FolderView, FolderId,
       // Encryption context
       EncryptionContext,
   };

   // Re-export collection types
   pub use bitwarden_collections::collection::{
       Collection, CollectionView, CollectionId, CollectionType,
   };

   // Keep custom types for API responses and organization
   mod cipher_request;
   mod organization;
   mod sync_response;
   mod validation_error;

   pub use cipher_request::*;
   pub use organization::*;
   pub use sync_response::*;
   pub use validation_error::*;
   ```

**Validation:** `cargo check` should pass with re-exports active.

### Phase 2: Update SyncService for SDK Types (Medium Risk)

**Goal:** Modify sync to deserialize directly into SDK types and handle ID conversion.

**Key Challenge:** SDK types use UUID newtypes while storage uses string keys.

**Files to Modify:**

1. **`crates/bw-core/src/models/vault/sync_response.rs`**
   ```rust
   use crate::models::vault::{Cipher, Collection, Folder};  // Now SDK types
   use serde::{Deserialize, Serialize};
   use std::collections::HashMap;

   /// API sync endpoint response - uses SDK types directly
   #[derive(Debug, Clone, Serialize, Deserialize)]
   #[serde(rename_all = "camelCase")]
   pub struct SyncResponse {
       #[serde(default)]
       pub ciphers: Vec<Cipher>,
       #[serde(default)]
       pub folders: Vec<Folder>,
       #[serde(default)]
       pub collections: Vec<Collection>,
       // ... other fields unchanged
   }
   ```

2. **`crates/bw-core/src/services/vault/sync_service.rs`**

   Update to handle SDK ID types:
   ```rust
   // Convert ciphers Vec to HashMap keyed by ID string
   let ciphers_map: HashMap<String, _> = sync_response
       .ciphers
       .into_iter()
       .filter_map(|c| c.id.map(|id| (id.to_string(), c)))
       .collect();
   ```

**Validation:** `cargo test` should pass, sync should work.

### Phase 3: Initialize SDK Crypto at Unlock (Critical)

**Goal:** Initialize SDK crypto state when vault is unlocked so VaultClient operations work.

**Files to Modify/Create:**

1. **`crates/bw-core/src/services/vault/mod.rs`** - Add crypto initialization helper

   ```rust
   use bitwarden_core::Client;
   use bitwarden_crypto::Kdf;
   use crate::key_management::crypto::{InitUserCryptoRequest, InitUserCryptoMethod};

   /// Initialize SDK crypto context for vault operations
   pub async fn initialize_vault_crypto(
       client: &Client,
       email: &str,
       kdf_params: Kdf,
       user_key_base64: &str,
       private_key_enc: &str,
   ) -> Result<(), VaultError> {
       client.crypto().initialize_user_crypto(InitUserCryptoRequest {
           user_id: None,  // Set if available
           kdf_params,
           email: email.to_string(),
           private_key: private_key_enc.parse()
               .map_err(|e| VaultError::CryptoError(format!("Invalid private key: {}", e)))?,
           signing_key: None,
           security_state: None,
           method: InitUserCryptoMethod::DecryptedKey {
               decrypted_user_key: user_key_base64.to_string(),
           },
       }).await.map_err(|e| VaultError::CryptoError(e.to_string()))
   }
   ```

2. **Integrate at unlock time** - Call `initialize_vault_crypto` when user unlocks vault

**Validation:** After unlock, `client.vault().ciphers().decrypt()` should work.

### Phase 4: Replace CipherService with SDK VaultClient (High Impact)

**Goal:** Replace 418 lines of manual encryption/decryption with SDK calls.

**Files to Modify:**

1. **`crates/bw-core/src/services/vault/cipher_service.rs`** - Simplify to ~50 lines

   ```rust
   use bitwarden_core::Client;
   use bitwarden_vault::{Cipher, CipherView, CipherListView, VaultClientExt};
   use super::errors::VaultError;
   use std::sync::Arc;

   /// Service for cipher operations using SDK VaultClient
   pub struct CipherService {
       client: Arc<Client>,
   }

   impl CipherService {
       pub fn new(client: Arc<Client>) -> Self {
           Self { client }
       }

       /// Decrypt a single cipher
       pub fn decrypt_cipher(&self, cipher: Cipher) -> Result<CipherView, VaultError> {
           self.client.vault().ciphers().decrypt(cipher)
               .map_err(|e| VaultError::DecryptionError(e.to_string()))
       }

       /// Decrypt multiple ciphers to list view
       pub fn decrypt_ciphers(&self, ciphers: Vec<Cipher>) -> Result<Vec<CipherListView>, VaultError> {
           self.client.vault().ciphers().decrypt_list(ciphers)
               .map_err(|e| VaultError::DecryptionError(e.to_string()))
       }

       /// Encrypt a cipher view for API submission
       pub fn encrypt_cipher(&self, cipher_view: CipherView) -> Result<Cipher, VaultError> {
           let ctx = self.client.vault().ciphers().encrypt(cipher_view)
               .map_err(|e| VaultError::EncryptionError(e.to_string()))?;
           Ok(ctx.cipher)
       }
   }
   ```

2. **Remove now-unused methods:**
   - `decrypt_string()`, `encrypt_string()`
   - `decrypt_login()`, `encrypt_login()`
   - `decrypt_card()`, `encrypt_card()`
   - `decrypt_identity()`, `encrypt_identity()`
   - `decrypt_fields()`, `encrypt_fields()`
   - `decrypt_optional()`, `encrypt_optional()`

**Note:** The SDK `CiphersClient` does NOT have separate folder/collection decrypt methods. Folders and collections use the generic `KeyStore::decrypt_list()` pattern. We need to:

```rust
// For folders and collections, use KeyStore directly
pub fn decrypt_folders(&self, folders: Vec<Folder>) -> Result<Vec<FolderView>, VaultError> {
    let key_store = self.client.internal.get_key_store();
    key_store.decrypt_list(&folders)
        .map_err(|e| VaultError::DecryptionError(e.to_string()))
}
```

### Phase 5: Delete Custom Type Files (Cleanup)

**Goal:** Remove replaced files, reducing codebase by 600+ lines.

**Files to DELETE:**

| File | Lines | Status After Phase 1-4 |
|------|-------|------------------------|
| `models/vault/cipher.rs` | 612 | No longer imported |
| `models/vault/folder.rs` | 27 | No longer imported |
| `models/vault/collection.rs` | 39 | No longer imported |

**Process:**
1. After Phase 4, run `cargo build` - should not reference deleted types
2. Delete files
3. Update `mod.rs` to remove mod declarations
4. Final `cargo build` and `cargo test`

### Phase 6: Update Dependent Code (Adaptation)

**Goal:** Fix any code that used CLI-specific type fields or patterns.

**Areas to Check:**

1. **Commands using vault types:**
   - `crates/bw-cli/src/commands/vault.rs`
   - Update to use SDK type patterns

2. **Search/filter services:**
   - `crates/bw-core/src/services/vault/search_service.rs`
   - Update field access to match SDK types

3. **Write service:**
   - `crates/bw-core/src/services/vault/write_service.rs`
   - Update for SDK encryption

**SDK Type Field Differences to Handle:**

| CLI Field | SDK Field | Notes |
|-----------|-----------|-------|
| `cipher.cipher_type` | `cipher.r#type` | Reserved keyword, use raw identifier |
| `cipher.id` (String) | `cipher.id` (Option<CipherId>) | Need `.map(|id| id.to_string())` for output |
| `folder.id` (String) | `folder.id` (Option<FolderId>) | Same pattern |

## 4. Technical Decisions

### 4.1 ID Type Handling Strategy

**Decision:** Convert ID types at storage and output boundaries only.

```rust
// Storage: Convert UUID to String for HashMap key
let id_string = cipher.id.map(|id| id.to_string()).unwrap_or_default();

// Output: Format for JSON response
#[derive(Serialize)]
struct CipherOutput {
    id: String,  // Convert from CipherId
    // ... other fields
}
```

**Rationale:** Minimizes changes to storage format while allowing internal SDK type usage.

### 4.2 Crypto Initialization Timing

**Decision:** Initialize SDK crypto at unlock time, store in session state.

**Flow:**
```
User runs `bw unlock`
    → Derive user key from password
    → Call initialize_vault_crypto()
    → Store SDK Client in session
    → All subsequent commands use initialized Client
```

**Rationale:** SDK requires crypto initialization before any decrypt operations. This is a one-time cost per session.

### 4.3 Error Type Mapping

**Decision:** Map SDK errors to VaultError at service boundaries.

```rust
// SDK errors to CLI errors
bitwarden_vault::DecryptError → VaultError::DecryptionError
bitwarden_vault::EncryptError → VaultError::EncryptionError
bitwarden_crypto::CryptoError → VaultError::CryptoError
```

### 4.4 Backwards Compatibility

**Decision:** Maintain storage format compatibility via serde configuration.

SDK types use `#[serde(rename_all = "camelCase")]` which matches CLI's existing storage format. No migration needed for:
- Cipher JSON structure
- Folder JSON structure
- Date serialization (DateTime<Utc> serializes to ISO 8601)

**Potential Issue:** Collection type has additional fields in SDK. Handle with:
```rust
#[serde(default)]  // Already present on most fields
```

## 5. Risk Mitigation

### 5.1 High Risk: Storage Deserialization

**Risk:** Existing stored vault data may not deserialize into SDK types.

**Mitigation:**
1. Write test that loads existing vault.json with SDK types
2. Verify all field mappings before Phase 2
3. Keep backup of original storage during testing

**Test:**
```rust
#[test]
fn test_existing_storage_compatibility() {
    let json = include_str!("../testdata/vault_sample.json");
    let ciphers: HashMap<String, Cipher> = serde_json::from_str(json)
        .expect("Should deserialize existing vault data");
}
```

### 5.2 Medium Risk: SDK Crypto State

**Risk:** SDK crypto may not be properly initialized before operations.

**Mitigation:**
1. Add explicit crypto initialization check before operations
2. Clear error message if crypto not initialized
3. Document initialization requirement

```rust
fn require_crypto(&self) -> Result<(), VaultError> {
    if !self.client.internal.is_crypto_initialized() {
        return Err(VaultError::CryptoNotInitialized);
    }
    Ok(())
}
```

### 5.3 Low Risk: Missing Type Exports

**Risk:** Some SDK types may not be publicly exported.

**Status:** Verified all required types are exported:
- `bitwarden_vault` exports all cipher, folder types
- `bitwarden_collections::collection` exports Collection types

## 6. File Change Summary

### Files to DELETE (678 lines)

| File | Lines |
|------|-------|
| `crates/bw-core/src/models/vault/cipher.rs` | 612 |
| `crates/bw-core/src/models/vault/folder.rs` | 27 |
| `crates/bw-core/src/models/vault/collection.rs` | 39 |

### Files to SIMPLIFY

| File | Before | After | Reduction |
|------|--------|-------|-----------|
| `cipher_service.rs` | 418 | ~50 | -368 lines |

### Files to MODIFY

| File | Change Type |
|------|-------------|
| `Cargo.toml` (workspace) | Add bitwarden-collections |
| `crates/bw-core/Cargo.toml` | Add bitwarden-collections |
| `crates/bw-core/src/models/vault/mod.rs` | SDK re-exports |
| `crates/bw-core/src/models/vault/sync_response.rs` | Use SDK types |
| `crates/bw-core/src/services/vault/mod.rs` | Add crypto init helper |
| `crates/bw-core/src/services/vault/sync_service.rs` | ID conversion |
| `crates/bw-core/src/services/vault/cipher_service.rs` | SDK VaultClient |
| `crates/bw-core/src/services/vault/search_service.rs` | SDK type fields |
| `crates/bw-core/src/services/vault/write_service.rs` | SDK encryption |
| `crates/bw-cli/src/commands/vault.rs` | SDK type patterns |

### Net Code Change

**Target:** -400+ lines (678 deleted - 250 modified + 50 simplified service)

## 7. Testing Strategy

### 7.1 Unit Tests

1. **Storage Compatibility Test**
   - Load existing vault JSON with SDK types
   - Verify round-trip serialization

2. **Crypto Initialization Test**
   - Initialize SDK crypto with test keys
   - Verify decrypt/encrypt operations work

3. **ID Conversion Test**
   - Convert SDK UUIDs to storage strings
   - Convert back for operations

### 7.2 Integration Tests

1. **Sync Test**
   - Mock API response
   - Verify storage with SDK types

2. **List Items Test**
   - Load ciphers
   - Decrypt with SDK
   - Verify output format

3. **Create/Edit Test**
   - Create cipher view
   - Encrypt with SDK
   - Verify API format

### 7.3 Manual Tests

```bash
# After implementation, verify:
bw unlock '<password>'
bw list items --session "$BW_SESSION"
bw get item <id> --session "$BW_SESSION"
bw list folders --session "$BW_SESSION"
bw list collections --session "$BW_SESSION"
```

## 8. Implementation Order

1. **Phase 1:** Add dependencies and re-exports
2. **Phase 2:** Update SyncResponse to use SDK types
3. **Phase 3:** Add crypto initialization at unlock
4. **Phase 4:** Replace CipherService with SDK VaultClient
5. **Phase 5:** Delete custom type files
6. **Phase 6:** Update dependent code
7. **Final:** Run full test suite, verify CLI commands

## 9. Success Criteria

- [ ] `cargo build` passes with no errors
- [ ] `cargo test` passes all tests
- [ ] Net code reduction of 400+ lines
- [ ] No `SdkVaultBridge` or adapter classes
- [ ] `cipher.rs`, `folder.rs`, `collection.rs` files deleted
- [ ] `bw list items` command works correctly
- [ ] `bw get item <id>` command works correctly
- [ ] Existing stored vault data loads correctly

## 10. Anti-Patterns to Avoid

1. **NO adapter/bridge classes** - Use SDK types directly
2. **NO type conversion layers** - Only convert IDs at boundaries
3. **NO duplicate type definitions** - Re-export from SDK
4. **NO manual encryption** - Use SDK VaultClient methods
