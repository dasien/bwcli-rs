---
enhancement: 11-encrypt-decrypt
agent: requirements-analyst
task_id: task_1765334877_96453
timestamp: 2025-12-09T12:00:00Z
status: READY_FOR_ARCHITECTURE
---

# Requirements Analysis Summary: Vault Encryption/Decryption

## Executive Summary

This enhancement is **critical** and currently blocks all useful CLI functionality. The Rust CLI can login, sync, and list vault items, but returns encrypted EncString data (e.g., `"2.iv|data|mac"`) instead of readable plaintext. Without this enhancement, users cannot view their passwords, usernames, or notes.

The implementation will add protected storage to securely store the user key (encrypted with the session key), and decryption capabilities to convert vault item fields from encrypted EncStrings to readable plaintext.

---

## 1. Functional Requirements

### FR-1: Protected Storage System
**Description**: Implement a protected storage module that encrypts/decrypts sensitive data using the session key.

**User Story**: As a CLI user, I want my encryption keys to be stored securely at rest so that my vault remains protected even if my storage file is compromised.

**Acceptance Criteria**:
- [ ] `encrypt_protected_bytes(plain, key)` encrypts arbitrary bytes using SDK primitives
- [ ] `decrypt_protected_bytes(encrypted_b64, key)` decrypts previously encrypted data
- [ ] Uses `OctetStreamBytes` for raw byte encryption (public SDK API)
- [ ] Uses `EncString::to_buffer()` / `from_buffer()` for binary format
- [ ] Output is base64-encoded EncArrayBuffer format (binary, not string format)
- [ ] Compatible with TypeScript CLI's `__PROTECTED__` storage format

### FR-2: User Key Management
**Description**: Store and retrieve the user key encrypted with the session key.

**User Story**: As a CLI user, I want my encryption key stored securely after login so that subsequent commands can decrypt vault items.

**Acceptance Criteria**:
- [ ] `encrypt_user_key(user_key, session_key)` encrypts user key for storage
- [ ] `decrypt_user_key(encrypted_b64, session_key)` retrieves user key from storage
- [ ] `parse_session_key(session_str)` parses BW_SESSION environment variable
- [ ] Uses `BitwardenLegacyKeyBytes` for key encoding/decoding
- [ ] Storage key format: `__PROTECTED__{userId}_user_auto`

### FR-3: Login Flow Integration
**Description**: Update password login to store the encrypted user key in protected storage.

**User Story**: As a CLI user, when I login with my master password, I want a session key generated so I can later decrypt my vault items.

**Acceptance Criteria**:
- [ ] Password login generates session key using `SymmetricCryptoKey::make_aes256_cbc_hmac_key()`
- [ ] User key (decrypted from server response) is encrypted with session key
- [ ] Encrypted user key stored in `__PROTECTED__{userId}_user_auto`
- [ ] BW_SESSION returned for user to export

### FR-4: Unlock Flow Integration
**Description**: Update vault unlock to store the encrypted user key in protected storage.

**User Story**: As a CLI user, when I unlock my vault with my master password, I want to restore decryption capability.

**Acceptance Criteria**:
- [ ] Unlock retrieves encrypted user key from storage
- [ ] Decrypts with master key to validate password
- [ ] Generates new session key
- [ ] Stores user key encrypted with new session key
- [ ] Returns new BW_SESSION

### FR-5: Key Service
**Description**: Service to retrieve the user key for vault operations.

**User Story**: As a CLI user, when I run vault commands, I want my session key to be used to retrieve my encryption key.

**Acceptance Criteria**:
- [ ] Parses BW_SESSION from environment or --session flag
- [ ] Reads encrypted user key from `__PROTECTED__{userId}_user_auto`
- [ ] Decrypts user key using session key
- [ ] Returns user key ready for vault decryption
- [ ] Returns appropriate error if session key invalid

### FR-6: CipherService Decryption
**Description**: Update CipherService to decrypt vault item fields.

**User Story**: As a CLI user, I want my vault items displayed with decrypted names, usernames, passwords, and notes.

**Acceptance Criteria**:
- [ ] `decrypt_cipher()` accepts user key parameter
- [ ] Uses `EncString::from_str()` to parse encrypted fields
- [ ] Uses `decrypt_with_key()` to decrypt EncStrings
- [ ] Handles missing/null fields gracefully
- [ ] Decrypts all cipher types: Login, SecureNote, Card, Identity

### FR-7: List Items Command
**Description**: Update `bw list items` to show decrypted data.

**User Story**: As a CLI user, I want `bw list items` to show readable item names so I can find what I'm looking for.

**Acceptance Criteria**:
- [ ] Gets user key via KeyService
- [ ] Decrypts cipher names before output
- [ ] Returns decrypted item list as JSON
- [ ] Handles decryption errors gracefully (logs warning, continues)

### FR-8: Get Item Command
**Description**: Update `bw get item` to show decrypted data.

**User Story**: As a CLI user, I want `bw get item <id>` to show my full decrypted login credentials.

**Acceptance Criteria**:
- [ ] Gets user key via KeyService
- [ ] Decrypts all item fields (name, notes, login data, etc.)
- [ ] Returns complete decrypted item as JSON

### FR-9: Get Field Commands
**Description**: Update `bw get password/username/totp/notes` commands.

**User Story**: As a CLI user, I want to quickly retrieve specific fields like passwords for scripting.

**Acceptance Criteria**:
- [ ] `bw get password <id>` returns decrypted password
- [ ] `bw get username <id>` returns decrypted username
- [ ] `bw get totp <id>` generates TOTP code from decrypted secret
- [ ] `bw get notes <id>` returns decrypted notes
- [ ] Supports `--raw` flag for unquoted output

---

## 2. Non-Functional Requirements

### NFR-1: Performance
- Decryption should add <100ms to list/get operations
- Avoid unnecessary decryption (only decrypt displayed fields)

### NFR-2: Security
- User key must be encrypted at rest using session key
- User key must never be logged or printed
- Decrypted passwords should not be cached unnecessarily
- Clear sensitive data from memory when done (use zeroize where applicable)
- Validate EncString format before attempting decryption
- Handle decryption failures gracefully without exposing internals

### NFR-3: Compatibility
- Must read/write storage format compatible with TypeScript CLI
- Cross-compatibility: TypeScript CLI session should work with Rust CLI (same storage)
- Storage key format must match exactly: `__PROTECTED__{userId}_user_auto`

### NFR-4: Memory Safety
- User key held in memory only during operations
- Consider zeroize for sensitive data structures

---

## 3. Technical Constraints

### TC-1: SDK Crypto Primitives Only
- **MUST use** `bitwarden-crypto` crate exclusively
- **DO NOT** implement custom AES encryption
- Key SDK types:
  - `OctetStreamBytes` - for encrypting raw bytes
  - `BitwardenLegacyKeyBytes` - for key encoding/decoding
  - `EncString` with `to_buffer()`/`from_buffer()` - for binary format
  - `KeyEncryptable`/`KeyDecryptable` traits
  - `SymmetricCryptoKey::make_aes256_cbc_hmac_key()` - for session key generation

### TC-2: Storage Format
- Protected storage format: base64-encoded EncArrayBuffer (binary format)
- Storage key: `__PROTECTED__{userId}_user_auto`
- Session key: 64-byte AES-256-CBC-HMAC key encoded as base64 in BW_SESSION

### TC-3: Existing Code Integration
- Auth service (`auth_service.rs`) already decrypts user key - need to store it
- CipherService (`cipher_service.rs`) has placeholder decryption - needs real implementation
- Storage keys module (`keys.rs`) may need new key constants
- Vault commands (`vault.rs`) need to pass session/user key to services

---

## 4. Integration Points

### IP-1: AuthService (auth_service.rs)
- **Current**: Decrypts user key but doesn't store it
- **Change**: Store encrypted user key in protected storage after login/unlock
- **Impact**: Medium - adding storage calls to existing flow

### IP-2: CipherService (cipher_service.rs)
- **Current**: Placeholder `decrypt_string()` that returns encrypted data unchanged
- **Change**: Real decryption using user key
- **Impact**: Medium - replacing placeholders with SDK calls

### IP-3: Storage Keys (keys.rs)
- **Current**: Has `UserKey` for master-key-encrypted user key
- **Change**: May need new key for protected storage format
- **Impact**: Low - adding new constant

### IP-4: Vault Commands (vault.rs)
- **Current**: Creates VaultService without decryption context
- **Change**: Pass session key, use KeyService to get user key
- **Impact**: Medium - adding key retrieval to command execution

### IP-5: ServiceContainer (container.rs)
- **Current**: Creates services without key context
- **Change**: May need KeyService integration
- **Impact**: Low-Medium - adding new service dependency

---

## 5. Scope Boundaries

### In Scope (Must Have MVP)
1. Protected storage module using SDK crypto
2. User key encryption/decryption functions
3. Session key parsing
4. Login flow stores encrypted user key
5. Unlock flow stores encrypted user key
6. Key service to retrieve user key
7. CipherService decryption methods
8. Decrypted output for `bw list items`
9. Decrypted output for `bw get item`
10. Decrypted output for `bw get password/username/totp/notes`

### Should Have (if time permits)
1. Decrypted folder names
2. Decrypted collection names
3. Organization key support
4. Attachment decryption

### Out of Scope
1. Vault write operations with encryption (separate enhancement)
2. Send encryption/decryption (separate enhancement)
3. Offline vault access (future enhancement)
4. Passwordless/SSO authentication flows

---

## 6. Risk Assessment

### High Risk
| Risk | Impact | Mitigation |
|------|--------|------------|
| SDK API changes | Could break crypto implementation | Pin SDK version, test thoroughly |
| Storage format incompatibility | Break TypeScript CLI interop | Extensive cross-testing |

### Medium Risk
| Risk | Impact | Mitigation |
|------|--------|------------|
| Performance regression | Slow list operations | Benchmark, optimize decryption |
| Memory leaks with sensitive data | Security concern | Use zeroize, audit memory lifecycle |

### Low Risk
| Risk | Impact | Mitigation |
|------|--------|------------|
| Edge case decryption failures | Individual items fail | Log warnings, continue processing |

---

## 7. Success Metrics

### Primary Metrics
1. `bw list items` shows decrypted names (not EncStrings)
2. `bw get password <id>` returns actual password
3. `bw get username <id>` returns actual username
4. `bw get totp <id>` returns valid 6-digit code
5. All existing tests pass
6. New unit tests for protected storage achieve >90% coverage

### Secondary Metrics
1. Cross-compatibility: TypeScript CLI session works with Rust CLI
2. Performance: <100ms added latency for list/get operations
3. Security: User key never appears in logs

---

## 8. Acceptance Tests

| Test ID | Scenario | Expected Result |
|---------|----------|-----------------|
| AT-1 | Fresh login, run `bw list items` | Item names are readable text |
| AT-2 | Valid session, `bw get password <id>` | Actual password returned |
| AT-3 | Valid session, `bw get username <id>` | Actual username returned |
| AT-4 | Valid session, `bw get totp <id>` | Valid 6-digit TOTP code |
| AT-5 | Lock then unlock, `bw list items` | Decryption still works |
| AT-6 | Wrong session key, attempt decryption | Error returned (not garbage) |
| AT-7 | TypeScript CLI login, Rust CLI same session | Decryption works |

---

## 9. Dependencies

### Completed Prerequisites
- Enhancement: project-bootstrap (complete)
- Enhancement: storage-layer (complete)
- Enhancement: api-client (complete)
- Enhancement: authentication-commands (complete)
- Enhancement: vault-read-commands (complete - returns encrypted data)

### External Dependencies
- `bitwarden-crypto` crate (Bitwarden SDK)
- `base64` crate (already in use)

---

## 10. Implementation Phases

### Phase 1: Protected Storage Foundation
- Create `protected_storage.rs` module
- Implement `encrypt_protected_bytes()`, `decrypt_protected_bytes()`
- Implement `parse_session_key()`
- Unit tests for roundtrip encryption

### Phase 2: User Key Management
- Implement `encrypt_user_key()`, `decrypt_user_key()`
- Create KeyService for user key retrieval
- Unit tests for key management

### Phase 3: Auth Flow Integration
- Update `login_with_password()` to store encrypted user key
- Update `unlock()` to store encrypted user key
- Integration tests for login/unlock flows

### Phase 4: Decryption Implementation
- Update CipherService `decrypt_string()` with real SDK calls
- Add user key parameter to decryption methods
- Unit tests for cipher decryption

### Phase 5: Command Integration
- Update `bw list items` to use decryption
- Update `bw get item` to use decryption
- Update `bw get password/username/totp/notes` commands
- End-to-end integration tests

### Phase 6: Cross-Compatibility Testing
- Test with TypeScript CLI sessions
- Verify storage format compatibility
- Performance benchmarking

---

## 11. Files to Create/Modify

### New Files
1. `crates/bw-core/src/services/storage/protected_storage.rs` - Protected storage encryption
2. `crates/bw-core/src/services/key_service.rs` - User key retrieval service
3. (Optional) `crates/bw-core/src/models/vault/decrypted.rs` - Decrypted vault models (if not using existing CipherView)

### Modified Files
1. `crates/bw-core/src/services/storage/mod.rs` - Export protected_storage module
2. `crates/bw-core/src/services/storage/keys.rs` - Add protected storage key constant
3. `crates/bw-core/src/services/auth/auth_service.rs` - Store encrypted user key
4. `crates/bw-core/src/services/vault/cipher_service.rs` - Real decryption implementation
5. `crates/bw-cli/src/commands/vault.rs` - Pass user key to services
6. `crates/bw-core/src/services/mod.rs` - Export key_service
7. `crates/bw-core/src/services/container.rs` - Add KeyService to container

---

## 12. Open Questions

None - the research phase documented in `docs/research/vault_decryption_research.md` has addressed all architectural questions. The implementation approach using SDK primitives is clear.

---

## 13. References

- Enhancement specification: `enhancements/11-encrypt-decrypt/11-encrypt-decrypt.md`
- Research document: `docs/research/vault_decryption_research.md`
- TypeScript CLI protected storage: `apps/cli/src/platform/services/node-env-secure-storage.service.ts`
- SDK EncString: `crates/bitwarden-crypto/src/enc_string/symmetric.rs`
- SDK SymmetricCryptoKey: `crates/bitwarden-crypto/src/keys/symmetric_crypto_key.rs`
