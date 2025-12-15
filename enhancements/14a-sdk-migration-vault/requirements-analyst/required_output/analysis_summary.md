---
enhancement: 14a-sdk-migration-vault
agent: requirements-analyst
task_id: task_1765666525_97417
timestamp: 2025-12-13T20:15:00Z
status: READY_FOR_ARCHITECTURE
---

# Requirements Analysis Summary: SDK Migration - Vault (Direct SDK Types)

## 1. Executive Summary

This enhancement replaces the CLI's custom vault types and encryption code with DIRECT usage of SDK types from `bitwarden-vault`. The goal is code deletion and simplification, NOT the creation of adapter layers or bridges.

**Critical Anti-Pattern Alert**: A previous attempt (enhancement 14) created an `SdkVaultBridge` adapter layer. This approach is explicitly prohibited. This enhancement MUST use SDK types directly throughout the codebase.

## 2. Business Requirements

### 2.1 Primary Goals

1. **Code Simplification**: Delete ~1000+ lines of duplicate type definitions and manual encryption code
2. **SDK Alignment**: Use official Bitwarden SDK types for vault operations
3. **Maintainability**: Reduce maintenance burden by leveraging SDK-provided functionality
4. **Consistency**: Ensure vault operations use the same implementation as other Bitwarden clients

### 2.2 Success Metrics

| Metric | Target |
|--------|--------|
| Net code reduction | 400+ lines removed |
| Build status | Passes with no errors |
| CLI commands functional | `list items`, `get item`, `list folders`, etc. work correctly |
| Unit tests | All existing tests pass |

## 3. Functional Requirements

### 3.1 Type Replacement Requirements

#### FR-1: Cipher Types (DELETE and REPLACE)

**Current State**: `crates/bw-core/src/models/vault/cipher.rs` (612 lines)
- Custom `Cipher`, `CipherView`, `CipherType` structs
- Custom `CipherLogin`, `CipherCard`, `CipherIdentity`, `CipherSshKey` structs
- Custom View equivalents for all cipher sub-types
- Manual serde configuration duplicating SDK behavior

**Target State**: Re-export SDK types directly
```rust
pub use bitwarden_vault::{
    Cipher, CipherView, CipherListView, CipherType, CipherRepromptType,
    Login, LoginView, Card, CardView, Identity, IdentityView,
    SecureNote, SecureNoteView, SshKey, SshKeyView,
    Field, FieldView, FieldType,
    Attachment, AttachmentView,
    PasswordHistory, PasswordHistoryView,
    LoginUri, LoginUriView, UriMatchType,
    CipherId,
};
```

**Acceptance Criteria**:
- [ ] `cipher.rs` file is deleted entirely
- [ ] No custom Cipher types exist in CLI codebase
- [ ] All imports use `bitwarden_vault::*` types

#### FR-2: Folder Types (DELETE and REPLACE)

**Current State**: `crates/bw-core/src/models/vault/folder.rs` (27 lines)
- Custom `Folder` and `FolderView` structs

**Target State**: Re-export SDK types
```rust
pub use bitwarden_vault::{Folder, FolderView, FolderId};
```

**Acceptance Criteria**:
- [ ] `folder.rs` file is deleted or reduced to re-exports
- [ ] All code uses SDK `Folder` and `FolderView` types

#### FR-3: Collection Types (DELETE and REPLACE)

**Current State**: `crates/bw-core/src/models/vault/collection.rs` (39 lines)
- Custom `Collection` and `CollectionView` structs

**Target State**: Re-export SDK types
```rust
pub use bitwarden_collections::collection::{Collection, CollectionView, CollectionId};
```

**Acceptance Criteria**:
- [ ] `collection.rs` file is deleted or reduced to re-exports
- [ ] `bitwarden-collections` dependency added to Cargo.toml
- [ ] All code uses SDK Collection types

### 3.2 Service Simplification Requirements

#### FR-4: CipherService Simplification

**Current State**: `crates/bw-core/src/services/vault/cipher_service.rs` (418 lines)
- Manual field-by-field encryption/decryption
- Direct use of `bitwarden_crypto::EncString`
- Separate methods for each cipher type component

**Target State**: SDK VaultClient wrapper (~50 lines)
```rust
pub fn decrypt_cipher(&self, cipher: Cipher) -> Result<CipherView, VaultError> {
    self.client.vault().ciphers().decrypt(cipher)
}

pub fn encrypt_cipher(&self, cipher_view: CipherView) -> Result<Cipher, VaultError> {
    self.client.vault().ciphers().encrypt(cipher_view)
}
```

**Acceptance Criteria**:
- [ ] Manual decrypt_* methods deleted
- [ ] Manual encrypt_* methods deleted
- [ ] All operations use `client.vault().ciphers().*` methods
- [ ] Code reduced from 418 to ~50 lines

#### FR-5: SDK Crypto Initialization

**Current State**: No SDK crypto initialization

**Target State**: Crypto initialized before vault operations
```rust
client.crypto().initialize_user_crypto(InitUserCryptoRequest {
    kdf_params: kdf,
    method: InitUserCryptoMethod::DecryptedKey {
        decrypted_user_key: user_key.to_base64().to_string(),
    },
    ...
}).await
```

**Acceptance Criteria**:
- [ ] SDK crypto initialized when session is unlocked
- [ ] VaultClient methods work without passing explicit keys
- [ ] Crypto state managed by SDK internally

### 3.3 Storage Compatibility Requirements

#### FR-6: Storage Format Compatibility

**Current State**: Vault data stored as `HashMap<String, Cipher>` with camelCase JSON

**Target State**: Same format, but using SDK `Cipher` type

**Acceptance Criteria**:
- [ ] Existing stored vault data can be deserialized into SDK types
- [ ] SDK types serialize to same JSON format as before
- [ ] No data migration required for existing users

### 3.4 ID Type Handling Requirements

#### FR-7: UUID Newtype Handling

**Current State**: IDs stored as `String`

**Target State**: SDK uses UUID newtypes (`CipherId`, `FolderId`, etc.)

**Acceptance Criteria**:
- [ ] String-to-ID conversion handled at API boundaries
- [ ] Storage keys remain strings for HashMap compatibility
- [ ] ID parsing errors handled gracefully

## 4. Non-Functional Requirements

### NFR-1: No Adapter Layers

**Requirement**: No bridge classes, adapter layers, or type conversion utilities (except for ID string conversion).

**Rationale**: The previous attempt (enhancement 14) created an `SdkVaultBridge` that translated between CLI types and SDK types. This defeated the purpose of SDK migration.

**Validation**: Grep codebase should NOT find:
- `Bridge` classes for vault operations
- `Adapter` classes for vault operations
- Conversion functions between CLI and SDK cipher/folder/collection types

### NFR-2: Backward Compatibility

**Requirement**: CLI commands must function identically from user perspective.

**Validation**:
- `bw list items` produces same output format
- `bw get item <id>` works correctly
- `bw create item` creates items successfully
- `bw edit item` updates items successfully

### NFR-3: Performance

**Requirement**: No significant performance regression.

**Validation**: Operations complete in comparable time to current implementation.

## 5. Constraints and Assumptions

### 5.1 Technical Constraints

1. **SDK Type Visibility**: Some SDK internal types (e.g., `Login`, `Card`) may have limited visibility. The architect MUST verify export availability.

2. **Serde Compatibility**: SDK types use `#[serde(rename_all = "camelCase")]` - this matches existing CLI format and should be compatible.

3. **Date Handling**: SDK uses `DateTime<Utc>` for dates; CLI currently uses ISO 8601 strings. Serialization compatibility MUST be verified.

4. **EncString Type**: SDK uses proper `EncString` type for encrypted fields, not raw strings.

### 5.2 Dependencies

| Crate | Current Version | Notes |
|-------|-----------------|-------|
| bitwarden-vault | workspace | Already included |
| bitwarden-collections | workspace | Needs to be added |
| bitwarden-core | workspace | For Client, crypto |
| bitwarden-crypto | workspace | For EncString, Kdf |

### 5.3 Assumptions

1. SDK `Cipher` type serializes to JSON compatible with existing storage format
2. SDK `VaultClient` extension trait is publicly available
3. SDK crypto initialization works with pre-decrypted keys
4. Existing synced vault data can be deserialized into SDK types

## 6. Risk Assessment

### 6.1 High Risk Items

| Risk | Impact | Mitigation |
|------|--------|------------|
| SDK types don't serialize compatibly | Storage corruption | Verify JSON format before implementation |
| Some SDK types not exported | Implementation blocked | Architect to verify all type exports |
| Crypto initialization complexity | Auth flow changes | Document initialization sequence carefully |

### 6.2 Medium Risk Items

| Risk | Impact | Mitigation |
|------|--------|------------|
| ID type conversion overhead | Minor performance impact | Only convert at boundaries |
| Date format differences | Display inconsistencies | Test serialization round-trips |

### 6.3 Low Risk Items

| Risk | Impact | Mitigation |
|------|--------|------------|
| Test updates required | Development time | Tests should pass with SDK types |

## 7. Files Affected

### 7.1 Files to DELETE

| File | Lines | Reason |
|------|-------|--------|
| `crates/bw-core/src/models/vault/cipher.rs` | 612 | Replace with SDK re-exports |
| `crates/bw-core/src/models/vault/folder.rs` | 27 | Replace with SDK re-exports |
| `crates/bw-core/src/models/vault/collection.rs` | 39 | Replace with SDK re-exports |

### 7.2 Files to SIMPLIFY

| File | Current Lines | Target Lines | Change |
|------|---------------|--------------|--------|
| `cipher_service.rs` | 418 | ~50 | -368 lines |

### 7.3 Files to MODIFY

| File | Change |
|------|--------|
| `Cargo.toml` (workspace) | Add `bitwarden-collections` |
| `crates/bw-core/Cargo.toml` | Add `bitwarden-collections` |
| `crates/bw-core/src/models/vault/mod.rs` | Re-export SDK types |
| `crates/bw-core/src/services/vault/mod.rs` | Update for SDK crypto |
| `crates/bw-core/src/services/vault/sync_service.rs` | Use SDK types |
| `crates/bw-core/src/services/vault/write_service.rs` | Use SDK types |
| `crates/bw-core/src/services/vault/search_service.rs` | Update for SDK types |
| `crates/bw-cli/src/commands/vault.rs` | Update for SDK types |

## 8. User Stories

### US-1: List Vault Items
**As a** CLI user
**I want to** run `bw list items`
**So that** I can see my decrypted vault items

**Acceptance Criteria**:
- [ ] Command returns list of items
- [ ] Items are properly decrypted
- [ ] Output format matches TypeScript CLI

### US-2: Get Single Item
**As a** CLI user
**I want to** run `bw get item <id>`
**So that** I can retrieve a specific vault item

**Acceptance Criteria**:
- [ ] Item is retrieved by ID
- [ ] All fields are properly decrypted
- [ ] Type-specific data (login, card, etc.) is included

### US-3: Create Vault Item
**As a** CLI user
**I want to** run `bw create item`
**So that** I can add new items to my vault

**Acceptance Criteria**:
- [ ] Item is encrypted using SDK
- [ ] Item is saved to API
- [ ] Item appears in subsequent list operations

### US-4: Edit Vault Item
**As a** CLI user
**I want to** run `bw edit item <id>`
**So that** I can modify existing vault items

**Acceptance Criteria**:
- [ ] Existing item is decrypted
- [ ] Modified item is re-encrypted
- [ ] Changes are saved to API

## 9. Phased Implementation Approach

### Phase 1: Type Migration
1. Add `bitwarden-collections` dependency
2. Update `models/vault/mod.rs` to re-export SDK types
3. Delete custom type files
4. Fix compilation errors in dependent code

### Phase 2: Service Migration
1. Add SDK crypto initialization
2. Replace CipherService encryption/decryption
3. Update WriteService for SDK types
4. Update SearchService for SDK types

### Phase 3: Validation
1. Run all existing tests
2. Manual testing of CLI commands
3. Verify storage compatibility

## 10. Verification Checklist

### Build Verification
- [ ] `cargo build` passes with no errors
- [ ] `cargo test` passes all tests

### Type Verification
- [ ] Grep finds `bitwarden_vault::Cipher` usage
- [ ] No `SdkVaultBridge` or similar adapter classes exist
- [ ] `cipher.rs` file is deleted

### Functional Verification
- [ ] `bw unlock '<password>'` works
- [ ] `bw list items --session "$BW_SESSION"` works
- [ ] `bw get item <id> --session "$BW_SESSION"` works
- [ ] `bw list folders --session "$BW_SESSION"` works

### Code Quality Verification
- [ ] Net code reduction of 400+ lines achieved
- [ ] No JSON conversion between CLI and SDK types

## 11. Conclusion

This enhancement is a **code deletion and simplification** exercise. The primary deliverable is the REMOVAL of custom vault types and encryption code, replacing them with direct SDK usage. Any implementation that adds adapter layers or bridges should be rejected as it violates the core requirement of this enhancement.

The architect should focus on:
1. Verifying SDK type export availability
2. Confirming JSON serialization compatibility
3. Designing the crypto initialization flow
4. Planning the file deletion and update sequence

The implementer should:
1. Delete files before adding code
2. Use SDK types directly at import sites
3. Resist the temptation to create abstractions
4. Verify code reduction targets are met
