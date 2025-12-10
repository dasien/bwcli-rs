---
slug: encrypt-decrypt
status: NEW
created: 2024-12-09
author: Migration Team
priority: critical
---

# Enhancement: CLI Rust Migration - Vault Encryption/Decryption

## Overview
**Goal:** Implement protected storage and vault item decryption so CLI commands return readable data instead of encrypted EncStrings.

**User Story:**
As a CLI user, I want `bw list items` and `bw get item` to show decrypted names, usernames, passwords, and notes so that I can actually use my vault data from the command line.

## Context & Background
**Current State:**
- Rust CLI can login, sync, and list items
- However, vault item fields (name, username, password, notes) are returned as encrypted EncStrings (e.g., `"2.iv|data|mac"`)
- The login/unlock flow generates a random session key but doesn't store the user key for later decryption
- This makes the CLI effectively unusable for its primary purpose

**Technical Context:**
- The TypeScript CLI uses a layered security model with protected storage
- BW_SESSION is a random 64-byte key used to encrypt sensitive state
- The user key (needed for vault decryption) is stored encrypted in `__PROTECTED__` storage
- The `bitwarden-crypto` crate provides all necessary primitives

**Dependencies:**
- Enhancement: project-bootstrap (complete)
- Enhancement: storage-layer (complete)
- Enhancement: api-client (complete)
- Enhancement: authentication-commands (complete)
- Enhancement: vault-read-commands (complete - but returns encrypted data)
- Bitwarden SDK (`bitwarden-crypto` crate)

## Requirements

### Functional Requirements
1. Protected storage system to encrypt/decrypt sensitive data with session key
2. Store encrypted user key in protected storage on login/unlock
3. Retrieve and decrypt user key when vault operations need decryption
4. Decrypt vault item fields (name, username, password, totp, notes, URIs) for display
5. Maintain compatibility with TypeScript CLI storage format (`__PROTECTED__` prefix)
6. Support the EncArrayBuffer binary format for protected storage

### Non-Functional Requirements
- **Performance:** Decryption should add <100ms to list/get operations
- **Memory:** User key held in memory only during operations, then cleared
- **Security:** User key encrypted at rest, only decryptable with session key
- **Compatibility:** Must read/write storage format compatible with TypeScript CLI

### Must Have (MVP)
- [ ] Protected storage module using SDK crypto
- [ ] `encrypt_protected_bytes()` function using `OctetStreamBytes` and `EncString::to_buffer()`
- [ ] `decrypt_protected_bytes()` function using `EncString::from_buffer()`
- [ ] `encrypt_user_key()` and `decrypt_user_key()` functions
- [ ] `parse_session_key()` function to parse BW_SESSION
- [ ] Update login flow to store encrypted user key as `__PROTECTED__{userId}_user_auto`
- [ ] Update unlock flow to store encrypted user key
- [ ] Key service to retrieve user key from protected storage
- [ ] CipherService decryption methods using `EncString::decrypt_with_key()`
- [ ] Decrypted output for `bw list items`
- [ ] Decrypted output for `bw get item`
- [ ] Decrypted output for `bw get password`, `bw get username`, `bw get totp`, `bw get notes`

### Should Have (if time permits)
- [ ] Decrypted folder names
- [ ] Decrypted collection names
- [ ] Support for organization keys
- [ ] Attachment decryption

### Won't Have (out of scope)
- Vault write operations with encryption (separate enhancement)
- Send encryption/decryption (separate enhancement)
- Offline vault access (future enhancement)

## Open Questions

1. None - research phase complete, architecture is clear

## Constraints & Limitations
**Technical Constraints:**
- Must use SDK crypto primitives, not custom AES implementation
- Must use `OctetStreamBytes` for encrypting raw bytes (public API)
- Must use `EncString::to_buffer()`/`from_buffer()` for binary format
- Must use `BitwardenLegacyKeyBytes` for key encoding/decoding
- Storage keys must match TypeScript CLI format exactly

**Business/Timeline Constraints:**
- This is blocking all useful CLI functionality
- Critical path item - without this, CLI cannot display vault contents

## Success Criteria
**Definition of Done:**
- [ ] Protected storage module implemented with full test coverage
- [ ] Login stores encrypted user key in `__PROTECTED__` storage
- [ ] Unlock stores encrypted user key in `__PROTECTED__` storage
- [ ] `bw list items` shows decrypted item names
- [ ] `bw get item <id>` shows decrypted login credentials
- [ ] `bw get password <id>` returns decrypted password
- [ ] `bw get username <id>` returns decrypted username
- [ ] `bw get totp <id>` generates TOTP from decrypted secret
- [ ] All existing tests still pass
- [ ] New unit tests for protected storage
- [ ] Documentation updated

**Acceptance Tests:**
1. Given fresh login with password, when running `bw list items`, then item names are readable text (not EncStrings)
2. Given valid session, when running `bw get password <id>`, then actual password is returned
3. Given valid session, when running `bw get username <id>`, then actual username is returned
4. Given valid session, when running `bw get totp <id>`, then valid 6-digit TOTP code is returned
5. Given lock then unlock, when running `bw list items`, then decryption still works
6. Given wrong session key, when attempting decryption, then error is returned (not garbage data)
7. Given TypeScript CLI login, when using Rust CLI with same session, then decryption works (cross-compatibility)

## Security & Safety Considerations
- User key must never be logged or printed
- User key must be encrypted at rest using session key
- Decrypted passwords should not be cached unnecessarily
- Clear sensitive data from memory when done (use zeroize)
- Validate EncString format before attempting decryption
- Handle decryption failures gracefully without exposing internals

## UI/UX Considerations
- No visible change to command interface
- Output format remains the same, just decrypted
- Error messages should not reveal cryptographic details
- If session key invalid, prompt user to login again

## Testing Strategy
**Unit Tests:**
- Protected storage encrypt/decrypt roundtrip
- User key encrypt/decrypt roundtrip
- Session key parsing from base64
- Wrong key fails decryption
- Invalid format handling
- EncString field decryption

**Integration Tests:**
- Full login → list items flow shows decrypted data
- Lock → unlock → list items works
- Get password/username/totp with real vault data

**Manual Test Scenarios:**
1. Login with real account, verify `bw list items` shows readable names
2. Run `bw get password <id>` and verify actual password returned
3. Compare output with TypeScript CLI to verify format matches
4. Test with TypeScript CLI session (cross-compatibility)

## References & Research
- Research document: `docs/research/vault_decryption_research.md`
- TypeScript CLI protected storage: `apps/cli/src/platform/services/node-env-secure-storage.service.ts`
- TypeScript CLI encrypt service: `libs/common/src/key-management/crypto/services/encrypt.service.implementation.ts`
- SDK PureCrypto (reference implementation): `crates/bitwarden-wasm-internal/src/pure_crypto.rs`
- SDK EncString: `crates/bitwarden-crypto/src/enc_string/symmetric.rs`
- SDK SymmetricCryptoKey: `crates/bitwarden-crypto/src/keys/symmetric_crypto_key.rs`

## Notes for PM Subagent
- This is a critical blocking enhancement
- No user-facing API changes, just making existing commands useful
- Verify acceptance tests cover the key user scenarios
- Cross-compatibility with TypeScript CLI is important

## Notes for Architect Subagent
- Use SDK crypto primitives exclusively - do NOT implement custom AES
- Key SDK types to use:
  - `OctetStreamBytes` - for encrypting raw bytes
  - `BitwardenLegacyKeyBytes` - for key encoding/decoding
  - `EncString` with `to_buffer()`/`from_buffer()` - for binary EncArrayBuffer format
  - `KeyEncryptable`/`KeyDecryptable` traits
  - `SymmetricCryptoKey::make_aes256_cbc_hmac_key()` - for generating session key
- Protected storage format: base64-encoded EncArrayBuffer (binary format, not string format)
- Storage key: `__PROTECTED__{userId}_user_auto`
- Session key is 64-byte AES-256-CBC-HMAC key encoded as base64 in BW_SESSION
- See `docs/research/vault_decryption_research.md` for detailed architecture

## Notes for Implementer Subagent
- Create `crates/bw-core/src/services/storage/protected_storage.rs` for protected storage
- Key functions needed:
  ```rust
  pub fn encrypt_protected_bytes(plain: &[u8], key: &SymmetricCryptoKey) -> Result<String>
  pub fn decrypt_protected_bytes(encrypted_b64: &str, key: &SymmetricCryptoKey) -> Result<Vec<u8>>
  pub fn encrypt_user_key(user_key: &SymmetricCryptoKey, session_key: &SymmetricCryptoKey) -> Result<String>
  pub fn decrypt_user_key(encrypted_b64: &str, session_key: &SymmetricCryptoKey) -> Result<SymmetricCryptoKey>
  pub fn parse_session_key(session_str: &str) -> Result<SymmetricCryptoKey>
  ```
- Use `OctetStreamBytes::from(bytes).encrypt_with_key(&key)?` for encryption
- Use `enc_string.to_buffer()` to get binary EncArrayBuffer format
- Use `EncString::from_buffer(&bytes)?` to parse binary format
- Use `enc_string.decrypt_with_key(&key)` for decryption
- Modify `auth_service.rs` login/unlock to store encrypted user key
- Create key service to retrieve user key from protected storage
- Update CipherService to decrypt fields using `EncString::from_str()` and `decrypt_with_key()`
- See `docs/research/vault_decryption_research.md` for code examples

## Notes for Testing Subagent
- Test protected storage roundtrip with various data sizes
- Test that wrong key fails decryption
- Test session key parsing with valid and invalid base64
- Test full flow: login → list items → verify decrypted output
- Test lock/unlock cycle maintains decryption ability
- Compare decrypted output with TypeScript CLI for format compatibility
- Test error handling for corrupted encrypted data
