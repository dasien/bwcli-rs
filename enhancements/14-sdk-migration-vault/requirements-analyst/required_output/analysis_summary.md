---
enhancement: 14-sdk-migration-vault
agent: requirements-analyst
task_id: task_1765656920_64057
timestamp: 2025-12-13T21:45:00Z
status: READY_FOR_ARCHITECTURE
---

# Requirements Analysis Summary: SDK Migration - Vault Encryption/Decryption

## Executive Summary

This enhancement migrates the custom cipher/folder/collection encryption/decryption implementation in `bw-core` to use the SDK's `bitwarden-vault` crate. The goal is to eliminate code duplication (~3,262 lines), gain access to advanced features (cipher keys, password history, FIDO2), and ensure cryptographic correctness through a single source of truth.

## Scope Definition

### In Scope

1. **Cipher Operations**
   - Replace `CipherService::decrypt_cipher` with `CiphersClient::decrypt`
   - Replace `CipherService::decrypt_ciphers` with `CiphersClient::decrypt_list`
   - Replace `CipherService::encrypt_cipher` with `CiphersClient::encrypt`
   - Replace custom Login/Card/Identity/SecureNote encryption/decryption

2. **Folder Operations**
   - Replace custom folder encryption with `FoldersClient::encrypt`
   - Replace custom folder decryption with `FoldersClient::decrypt`/`decrypt_list`

3. **Collection Operations**
   - Replace custom collection decryption with SDK collection decryption

4. **Model Migration**
   - Transition from CLI custom types to SDK `Cipher`/`CipherView` types
   - Transition from CLI custom types to SDK `Folder`/`FolderView` types
   - Handle type conversions for storage compatibility

5. **Key Management**
   - Initialize SDK `Client` with proper key store from session key
   - Support existing `KeyStoreContext` patterns

### Out of Scope

- Attachment file operations (separate enhancement)
- Organization key management overhaul
- Full Send support
- SSH key cipher type support
- Migration of existing stored data (format will remain compatible)

## Functional Requirements

### FR-1: Cipher Decryption
**Description**: The system shall decrypt vault ciphers using the SDK's `CiphersClient`.

**Acceptance Criteria**:
- [ ] Single cipher decryption returns identical results to current implementation
- [ ] Batch cipher decryption returns identical results to current implementation
- [ ] All cipher types (Login, SecureNote, Card, Identity) decrypt correctly
- [ ] Custom fields decrypt correctly
- [ ] URIs with match types work
- [ ] Decryption errors are handled gracefully with appropriate error messages

### FR-2: Cipher Encryption
**Description**: The system shall encrypt vault ciphers using the SDK's `CiphersClient`.

**Acceptance Criteria**:
- [ ] Encrypted ciphers are readable by TypeScript CLI
- [ ] All cipher types encrypt correctly
- [ ] Custom fields encrypt correctly
- [ ] Empty/null optional fields are handled correctly

### FR-3: Folder Operations
**Description**: The system shall use SDK for folder encryption/decryption.

**Acceptance Criteria**:
- [ ] Folder name encryption uses `FoldersClient::encrypt`
- [ ] Folder name decryption uses `FoldersClient::decrypt`
- [ ] Encrypted folders are compatible with TypeScript CLI

### FR-4: Collection Operations
**Description**: The system shall use SDK for collection decryption.

**Acceptance Criteria**:
- [ ] Collection name decryption uses SDK methods
- [ ] Organization-scoped collections decrypt correctly

### FR-5: CLI Interface Compatibility
**Description**: All existing CLI commands shall continue to work unchanged.

**Acceptance Criteria**:
- [ ] `bw list items` produces same output structure
- [ ] `bw get item <id>` produces same output structure
- [ ] `bw create item` accepts same input format
- [ ] `bw edit item` accepts same input format
- [ ] `bw get username/password/uri/totp` work identically
- [ ] All existing command-line flags work identically

### FR-6: Storage Compatibility
**Description**: The system shall maintain compatibility with TypeScript CLI storage format.

**Acceptance Criteria**:
- [ ] SDK types serialize to identical JSON format as current types
- [ ] TypeScript CLI can read data written by Rust CLI
- [ ] Rust CLI can read data written by TypeScript CLI
- [ ] DateTime serialization matches string format
- [ ] UUID types serialize as strings
- [ ] Optional fields serialize correctly (null when None)

## Non-Functional Requirements

### NFR-1: Security
**Description**: Use SDK crypto exclusively - no custom AES implementations.

**Acceptance Criteria**:
- [ ] All encryption uses SDK's `CiphersClient::encrypt` or equivalent
- [ ] All decryption uses SDK's `CiphersClient::decrypt` or equivalent
- [ ] No direct use of `bitwarden_crypto::EncString` for manual encryption/decryption
- [ ] Key material is handled according to SDK patterns

### NFR-2: Performance
**Description**: No perceivable difference in list/get operations.

**Acceptance Criteria**:
- [ ] `bw list items` completes within same time bounds (+/- 10%)
- [ ] `bw get item` completes within same time bounds (+/- 10%)
- [ ] Batch decryption scales similarly to current implementation

### NFR-3: Code Quality
**Description**: Reduce code duplication and improve maintainability.

**Acceptance Criteria**:
- [ ] Code reduction of 500+ lines in vault services
- [ ] Removal of custom AES/crypto code from `CipherService`
- [ ] Clear separation between SDK calls and business logic

## User Stories

### US-1: Developer Experience
**As a** developer maintaining the CLI,
**I want** the vault encryption/decryption to use the SDK,
**So that** I have a single source of truth for cryptographic operations and benefit from SDK security improvements automatically.

**Acceptance Criteria**:
- [ ] `CipherService` delegates to SDK for all crypto operations
- [ ] New SDK features (cipher keys, password history) are accessible
- [ ] Code is easier to understand and maintain

### US-2: List Items Command
**As a** CLI user,
**I want** to list my vault items,
**So that** I can see all my stored credentials.

**Acceptance Criteria**:
- [ ] `bw list items` shows all decrypted item names
- [ ] Filtering by folder, collection, organization works
- [ ] Search functionality works
- [ ] Trash items are handled correctly

### US-3: Get Item Command
**As a** CLI user,
**I want** to retrieve a specific vault item,
**So that** I can view or copy its credentials.

**Acceptance Criteria**:
- [ ] `bw get item <id>` returns fully decrypted item
- [ ] All fields (login, card, identity, notes, custom fields) are decrypted
- [ ] Item lookup by name works

### US-4: Create Item Command
**As a** CLI user,
**I want** to create new vault items,
**So that** I can store new credentials.

**Acceptance Criteria**:
- [ ] `bw create item` encrypts and stores new items
- [ ] Created items appear in TypeScript CLI
- [ ] All cipher types can be created

### US-5: Edit Item Command
**As a** CLI user,
**I want** to edit existing vault items,
**So that** I can update my credentials.

**Acceptance Criteria**:
- [ ] `bw edit item` re-encrypts modified items
- [ ] Edited items appear correctly in TypeScript CLI
- [ ] Partial updates preserve unmodified fields

### US-6: Cross-CLI Compatibility
**As a** user of both Rust and TypeScript CLIs,
**I want** both CLIs to interoperate seamlessly,
**So that** I can use either CLI interchangeably.

**Acceptance Criteria**:
- [ ] Login with TypeScript CLI, list items with Rust CLI - works
- [ ] Create item with Rust CLI, view in TypeScript CLI - works
- [ ] Create item with TypeScript CLI, view in Rust CLI - works
- [ ] Edit item in Rust CLI, verify changes in TypeScript CLI - works

## Technical Challenges (Flagged for Architecture)

### TC-1: Key Store Initialization
**Challenge**: The SDK uses `KeyStoreContext` for key management, while the CLI currently uses raw `SymmetricCryptoKey`.

**Current Pattern**:
```rust
let user_key = self.key_service.get_user_key(session).await?;
let cipher_view = cipher_service.decrypt_cipher(&cipher, &user_key).await?;
```

**SDK Pattern**:
```rust
let client = Client::new(None);
client.internal.initialize_user_crypto(...);
let cipher_view = client.vault().ciphers().decrypt(cipher)?;
```

**Questions for Architecture**:
- How to properly initialize SDK Client with session-derived user key?
- Should a new `Client` be created per request or shared?
- How to handle key rotation scenarios?

### TC-2: Storage Type Compatibility
**Challenge**: CLI stores vault data as `HashMap<String, Cipher>` where `Cipher` is a custom type. Need to ensure SDK types serialize to same JSON format.

**Investigation Areas**:
- Compare `serde` attributes on CLI vs SDK Cipher types
- Verify field names match (`rename_all = "camelCase"`)
- Test round-trip serialization

**Questions for Architecture**:
- Option A: Re-export SDK types directly (cleaner but may need storage migration)
- Option B: Keep CLI types for storage, convert at boundaries (safer)
- Which approach balances maintainability vs migration risk?

### TC-3: ID Type Differences
**Challenge**: CLI uses `String` for IDs, SDK uses UUID newtypes (`CipherId`, `FolderId`).

**Current**: `id: String`
**SDK**: `id: Option<CipherId>` where `CipherId` is a UUID newtype

**Questions for Architecture**:
- Keep string IDs at API boundary and convert internally?
- Update storage to use UUID types (breaking change)?
- Use adapter layer for conversion?

### TC-4: Async vs Sync API
**Challenge**: Current implementation uses async methods. SDK vault operations may have different async patterns.

**Questions for Architecture**:
- Does SDK `CiphersClient::decrypt` require async context?
- How to handle potential async/sync mismatches?

## Project Phases

### Phase 1: SDK Integration Setup
**Scope**: Establish SDK client initialization pattern
- Add `VaultClientExt` usage
- Create SDK client initialization from session key
- Verify key store is properly populated

**Milestone**: SDK client can be created and initialized with user key

### Phase 2: Cipher Decryption Migration
**Scope**: Replace cipher decryption with SDK
- Replace `decrypt_cipher` method
- Replace `decrypt_ciphers` method
- Verify all cipher types work
- Update unit tests

**Milestone**: All list/get commands use SDK decryption

### Phase 3: Cipher Encryption Migration
**Scope**: Replace cipher encryption with SDK
- Replace `encrypt_cipher` method
- Update create/edit/move operations
- Verify storage compatibility

**Milestone**: All create/edit commands use SDK encryption

### Phase 4: Folder/Collection Migration
**Scope**: Replace folder and collection operations
- Replace folder encrypt/decrypt
- Replace collection decrypt
- Update related commands

**Milestone**: All folder/collection operations use SDK

### Phase 5: Model Cleanup
**Scope**: Remove or refactor custom models
- Evaluate SDK type re-export vs adapters
- Remove redundant model code
- Update imports throughout codebase

**Milestone**: Clean architecture with SDK types

### Phase 6: Testing & Validation
**Scope**: Comprehensive testing
- Cross-CLI compatibility tests
- Performance benchmarks
- Edge case validation

**Milestone**: All acceptance tests pass

## Dependencies

### Internal Dependencies
- `KeyService` - Provides user key derivation from session
- `Storage` - Stores encrypted vault data
- `ApiClient` - Syncs vault data from server
- `VaultService` - Coordinates vault operations

### External Dependencies
- `bitwarden-vault` crate - SDK vault functionality
- `bitwarden-crypto` crate - Cryptographic primitives
- `bitwarden-core` crate - SDK client infrastructure

## Constraints

### Technical Constraints
1. Must maintain TypeScript CLI storage compatibility
2. Cannot break existing CLI command interfaces
3. Must use SDK crypto exclusively after migration

### Business Constraints
1. Migration must be transparent to end users
2. No data loss or corruption during transition
3. Performance must not degrade noticeably

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Storage format incompatibility | Medium | High | Test round-trip serialization early; maintain adapter layer if needed |
| SDK API differences | Low | Medium | Review SDK source code; create comprehensive test suite |
| Key initialization complexity | Medium | High | Study SDK test patterns; prototype initialization early |
| Performance regression | Low | Medium | Benchmark before/after; profile hot paths |
| Feature parity gaps | Low | Low | Current features are well-supported by SDK |

## Success Metrics

1. **Code Reduction**: 500+ lines removed from vault services
2. **Test Coverage**: All existing tests pass + new SDK integration tests
3. **Compatibility**: 100% cross-CLI compatibility in acceptance tests
4. **Performance**: List/get operations within 10% of baseline
5. **Security**: No custom crypto code in CipherService

## Assumptions

1. SDK `bitwarden-vault` types have compatible serde serialization
2. SDK `Client` can be initialized with existing user key material
3. TOTP service migration (already completed) provides a working pattern
4. No storage migration is required if JSON format is compatible

## Open Questions for Stakeholders

1. **Key persistence**: Should the initialized SDK Client be persisted across commands or re-initialized each time?
2. **Error handling**: Should SDK crypto errors surface as specific error types or be mapped to existing `VaultError` variants?
3. **Feature flags**: Should advanced features (cipher keys, password history) be exposed immediately or in a follow-up enhancement?

## Recommendations

1. **Start with Phase 2** (cipher decryption) as it has the lowest risk and highest value
2. **Create adapter layer** for storage compatibility rather than migrating storage format
3. **Keep CLI types at API boundary** to minimize breaking changes
4. **Leverage TOTP migration pattern** - it already successfully uses SDK

---

## Appendix A: Files to Modify

### Primary Files
| File | Current Lines | Expected Change |
|------|---------------|-----------------|
| `cipher_service.rs` | 419 | Major refactor - delegate to SDK |
| `write_service.rs` | ~600 | Update encryption calls |
| `mod.rs` (VaultService) | 332 | Add SDK client initialization |
| `cipher.rs` (models) | ~800 | Possible removal or adapter conversion |
| `folder.rs` (models) | ~27 | Possible removal or adapter conversion |

### Secondary Files
| File | Expected Change |
|------|-----------------|
| `search_service.rs` | Type updates if models change |
| `validation_service.rs` | Type updates if models change |
| `vault.rs` (commands) | Minimal - service layer abstraction |

## Appendix B: Test Matrix

| Test Case | Cipher Type | Operation | Cross-CLI |
|-----------|-------------|-----------|-----------|
| Decrypt single | Login | Read | TS -> Rust |
| Decrypt single | SecureNote | Read | TS -> Rust |
| Decrypt single | Card | Read | TS -> Rust |
| Decrypt single | Identity | Read | TS -> Rust |
| Decrypt batch | Mixed | Read | TS -> Rust |
| Encrypt new | Login | Create | Rust -> TS |
| Encrypt update | Login | Edit | Rust -> TS |
| Round-trip | All types | All | Both ways |
| Custom fields | Login | Read/Write | Both ways |
| Unicode values | All types | Read/Write | Both ways |
| Empty optionals | All types | Read/Write | Both ways |
