---
slug: sdk-migration-vault
status: NEW
created: 2024-12-13
author: Migration Team
priority: high
---

# Enhancement: SDK Migration - Vault Encryption/Decryption

## Overview
**Goal:** Replace the custom cipher/folder/collection encryption/decryption implementation in `bw-core` with the SDK's `bitwarden-vault` crate to eliminate code duplication, gain access to advanced features (cipher keys, password history, FIDO2), and ensure cryptographic correctness.

**User Story:**
As a developer, I want the CLI to use the SDK's vault encryption/decryption so that we have a single source of truth for cryptographic operations and can benefit from SDK security improvements automatically.

## Context & Background

### Current CLI Implementation (~3,262 lines)
The CLI has custom vault services in `crates/bw-core/src/services/vault/`:

| File | Lines | Purpose |
|------|-------|---------|
| `cipher_service.rs` | 419 | Manual field-by-field encrypt/decrypt |
| `write_service.rs` | ~400 | CRUD operations with cache management |
| `sync_service.rs` | 161 | Sync from API to storage |
| `search_service.rs` | 154 | Filter/search operations |
| `validation_service.rs` | ~300 | Input validation |
| `totp_service.rs` | 111 | TOTP generation (already uses SDK!) |
| `errors.rs` | ~50 | Error types |
| `mod.rs` | 326 | Main VaultService coordinator |

Plus models in `crates/bw-core/src/models/vault/`:
| File | Lines | Purpose |
|------|-------|---------|
| `cipher.rs` | ~800 | Cipher/CipherView types |
| `folder.rs` | ~100 | Folder/FolderView types |
| `collection.rs` | ~80 | Collection types |

### SDK Implementation (`bitwarden-vault`, ~12,238 lines)
The SDK provides comprehensive vault functionality:

- **`VaultClient`** - Main entry point via `Client::vault()`
- **`CiphersClient`** - `encrypt()`, `decrypt()`, `decrypt_list()`, `move_to_organization()`
- **`FoldersClient`** - `encrypt()`, `decrypt()`, `decrypt_list()`, `create()`, `edit()`
- **`CollectionsClient`** - Collection operations
- **`TotpClient`** - TOTP generation
- **`AttachmentsClient`** - Attachment encrypt/decrypt
- **`PasswordHistoryClient`** - Password history tracking

### Key Differences

| Feature | CLI | SDK |
|---------|-----|-----|
| Key management | Direct `SymmetricCryptoKey` | `KeyStoreContext` with key hierarchy |
| Cipher keys | Not supported | Full support via `decrypt_cipher_key()` |
| Password history | Not tracked | `update_password_history()` |
| FIDO2 credentials | Not implemented | Full support |
| Organization keys | Basic org_id only | Proper key lookup |
| Attachment re-encryption | Pass-through | `reencrypt_attachment_keys()` |
| Move to organization | Not implemented | `move_to_organization()` |
| Field checksums | Not implemented | `generate_checksums()`, `remove_invalid_checksums()` |
| Type safety | String IDs | UUID newtypes (`CipherId`, `FolderId`, etc.) |
| TOTP | Already uses SDK | Already migrated |

## Requirements

### Functional Requirements
1. Replace `CipherService` encrypt/decrypt with SDK's `CiphersClient`
2. Replace custom folder encrypt/decrypt with SDK's `FoldersClient`
3. Replace custom collection decrypt with SDK's collection decryption
4. Maintain CLI interface compatibility (all existing commands work)
5. Support cipher keys for newer ciphers
6. Preserve TypeScript CLI storage compatibility

### Non-Functional Requirements
- **Security:** Use SDK crypto exclusively - no custom AES implementations
- **Performance:** No perceivable difference in list/get operations
- **Compatibility:** Continue to work with TypeScript CLI storage format

### Must Have (MVP)
- [ ] Use `VaultClientExt` to get `VaultClient` from SDK `Client`
- [ ] Replace `CipherService::decrypt_cipher` with `CiphersClient::decrypt`
- [ ] Replace `CipherService::decrypt_ciphers` with `CiphersClient::decrypt_list`
- [ ] Replace `CipherService::encrypt_cipher` with `CiphersClient::encrypt`
- [ ] Use SDK `Cipher`/`CipherView` types instead of custom models
- [ ] Use SDK `Folder`/`FolderView` types instead of custom models
- [ ] Initialize SDK Client with proper key store from session key
- [ ] Update storage layer to serialize/deserialize SDK types
- [ ] Update all CLI commands to use SDK types

### Should Have (if time permits)
- [ ] Password history tracking on cipher edits
- [ ] Support for cipher keys (individual cipher encryption)
- [ ] FIDO2 credential handling

### Won't Have (out of scope)
- Attachment file operations (separate enhancement)
- Organization key management overhaul
- Full Send support

## Technical Analysis

### Challenge 1: Key Store Initialization
The SDK uses `KeyStoreContext` for key management. The CLI currently uses raw `SymmetricCryptoKey`.

**Current CLI:**
```rust
let user_key = self.key_service.get_user_key(session).await?;
let cipher_view = cipher_service.decrypt_cipher(&cipher, &user_key).await?;
```

**SDK Approach:**
```rust
// SDK Client needs to be initialized with user key
let client = Client::new(None);
// Need to set up key store with user key from session
client.internal.initialize_user_crypto(...);

// Then use VaultClient
let cipher_view = client.vault().ciphers().decrypt(cipher)?;
```

### Challenge 2: Storage Type Compatibility
The CLI stores vault data as `HashMap<String, Cipher>` where `Cipher` is a custom type. Need to ensure SDK types serialize to same JSON format.

**Investigation needed:**
- Compare `serde` attributes on CLI vs SDK Cipher types
- Verify field names match (`rename_all = "camelCase"`)
- Test round-trip serialization

### Challenge 3: Model Type Differences
CLI uses `String` for IDs, SDK uses UUID newtypes:
- CLI: `id: String`
- SDK: `id: Option<CipherId>` where `CipherId` is a UUID newtype

CLI uses `CipherType` enum, SDK uses same but might have different serde attributes.

## Migration Strategy

### Phase 1: Add SDK Dependency
```toml
# In bw-core/Cargo.toml
bitwarden-vault = { path = "../../sdk-internal/crates/bitwarden-vault" }
```

### Phase 2: Create Adapter Layer
Create adapters to convert between CLI storage format and SDK types:
```rust
// src/services/vault/adapters.rs
impl From<StorageCipher> for bitwarden_vault::Cipher { ... }
impl From<bitwarden_vault::CipherView> for CipherViewOutput { ... }
```

### Phase 3: Replace CipherService Methods
One method at a time, replace CLI implementation with SDK calls.

### Phase 4: Update Models
Either:
- Option A: Re-export SDK types directly (cleaner but may need storage migration)
- Option B: Keep CLI types for storage, convert at boundaries (safer)

### Phase 5: Remove Duplicated Code

## Files to Modify

### Core Changes
1. `crates/bw-core/src/services/vault/cipher_service.rs` - Replace with SDK calls
2. `crates/bw-core/src/services/vault/mod.rs` - Update VaultService to use SDK Client
3. `crates/bw-core/src/services/vault/write_service.rs` - Use SDK encrypt
4. `crates/bw-core/src/models/vault/cipher.rs` - Possibly remove or convert to SDK re-exports
5. `crates/bw-core/src/models/vault/folder.rs` - Possibly remove or convert

### Potential Files to Delete (after migration)
- `crates/bw-core/src/services/vault/cipher_service.rs` (if fully replaced)
- Custom cipher model structs (if using SDK types)

## Notes for Implementer Subagent

### Key SDK Types
```rust
use bitwarden_vault::{
    VaultClientExt,  // Extension trait for Client::vault()
    Cipher, CipherView, CipherListView,
    Folder, FolderView,
    CipherType, CipherRepromptType,
    Login, LoginView, Card, CardView, Identity, IdentityView,
    generate_totp,  // Already used by TotpService
};
use bitwarden_core::Client;
```

### SDK Client Initialization
The SDK's `Client` manages key stores internally. To use it:
```rust
use bitwarden_core::Client;
use bitwarden_vault::VaultClientExt;

let client = Client::new(None);

// Need to initialize crypto with user key
// This is the tricky part - need to investigate how to set up
// the key store from our session key

// Then can use:
let vault = client.vault();
let cipher_view = vault.ciphers().decrypt(cipher)?;
```

### Storage Compatibility
The SDK types use `#[serde(rename_all = "camelCase")]` which should match TypeScript CLI.
Need to verify:
- `DateTime<Utc>` serialization matches string format
- UUID types serialize as strings
- Optional fields serialize as null when None

### Crypto Initialization Pattern
Look at how the SDK tests initialize crypto:
```rust
// From SDK tests:
let client = Client::init_test_account(test_bitwarden_com_account()).await;
```

The CLI needs equivalent initialization using our stored user key.

## Notes for Testing Subagent

### Critical Tests
1. Decrypt cipher created by TypeScript CLI
2. Decrypt cipher created by Rust CLI
3. Encrypt cipher in Rust, decrypt in TypeScript CLI
4. Encrypt cipher in TypeScript CLI, decrypt in Rust
5. List items shows decrypted names
6. Get item shows all decrypted fields
7. Edit item preserves all fields
8. Create item is readable by both CLIs

### Validation Tests
1. All cipher types work: Login, SecureNote, Card, Identity
2. Custom fields decrypt correctly
3. URIs with match types work
4. TOTP generation still works
5. Folder encrypt/decrypt works
6. Collection decrypt works

### Edge Cases
1. Cipher with cipher key (newer format)
2. Cipher without cipher key (legacy)
3. Organization cipher with org key
4. Empty optional fields
5. Unicode in field values

## Open Questions

1. **Key Store Initialization:** How to properly initialize SDK Client with our session-derived user key? May need to investigate `Client::internal` methods.

2. **Storage Format:** Do SDK types serialize identically to our current format? Need to test.

3. **Migration Path:** Should we keep both implementations during transition or do a full switch?

4. **ID Types:** Keep string IDs at API boundary and convert, or update storage to use UUID types?

## Success Criteria

**Definition of Done:**
- [ ] `bw list items` uses SDK decryption
- [ ] `bw get item` uses SDK decryption
- [ ] `bw create item` uses SDK encryption
- [ ] `bw edit item` uses SDK encryption
- [ ] TypeScript CLI compatibility verified
- [ ] All existing vault tests pass
- [ ] No custom AES/crypto code in CipherService
- [ ] Code reduction of 500+ lines

**Acceptance Tests:**
1. Login with TypeScript CLI, list items with Rust CLI - works
2. Create item with Rust CLI, view in TypeScript CLI - works
3. Create item with TypeScript CLI, view in Rust CLI - works
4. Edit item in Rust CLI, verify changes in TypeScript CLI - works
5. All cipher types (Login, SecureNote, Card, Identity) work
6. TOTP generation still works
