---
enhancement: 11-encrypt-decrypt
agent: requirements-analyst
document_type: architecture_handoff
timestamp: 2025-12-09T12:00:00Z
---

# Architecture Handoff: Vault Encryption/Decryption

## For the Architect Agent

This document provides context and guidance for designing the technical architecture.

---

## 1. What Needs to Be Built

### Core Components Needed

1. **Protected Storage Module**
   - Encrypt/decrypt arbitrary bytes using session key
   - Base64 encode/decode for storage
   - Binary EncArrayBuffer format (not string format)

2. **Key Management**
   - Encrypt user key with session key
   - Decrypt user key with session key
   - Parse session key from BW_SESSION string

3. **Key Service**
   - Retrieve user key from protected storage
   - Provide user key to vault operations
   - Handle session validation errors

4. **Cipher Decryption**
   - Replace placeholder `decrypt_string()` implementation
   - Parse EncString format and decrypt with user key
   - Handle all cipher types

---

## 2. Technical Flags for Architect

### SDK Primitives to Use

The research document (`docs/research/vault_decryption_research.md`) has identified the exact SDK APIs:

| Operation | SDK Type/Method |
|-----------|-----------------|
| Encrypt raw bytes | `OctetStreamBytes::from(bytes).encrypt_with_key(&key)` |
| Binary format | `EncString::to_buffer()` / `EncString::from_buffer()` |
| Key encoding | `BitwardenLegacyKeyBytes` |
| Generate session key | `SymmetricCryptoKey::make_aes256_cbc_hmac_key()` |
| Decrypt EncString | `enc_string.decrypt_with_key(&key)` |

### Storage Format Constraints

- Storage key: `__PROTECTED__{userId}_user_auto`
- Value: base64-encoded EncArrayBuffer (binary format)
- Must be compatible with TypeScript CLI

### Current Code State

1. **AuthService (`auth_service.rs:96-102`)**: Already decrypts user key with master key, but stores nothing. The `_user_key` variable is unused.

2. **CipherService (`cipher_service.rs:134-139`)**: Has placeholder that returns encrypted string unchanged:
   ```rust
   async fn decrypt_string(&self, enc_string: &str) -> Result<String, VaultError> {
       // TODO: Implement SDK decryption
       Ok(enc_string.to_string())  // Placeholder!
   }
   ```

3. **Vault Commands (`vault.rs:331-339`)**: Creates VaultService without session/key context.

---

## 3. Design Considerations

### Architecture Decisions Needed

1. **Where does KeyService live?**
   - New standalone service in `services/key_service.rs`?
   - Part of AuthService?
   - Part of ServiceContainer?

2. **How is session key passed to commands?**
   - Via GlobalArgs struct?
   - Environment variable read in each command?
   - Stored in ServiceContainer?

3. **How does CipherService get user key?**
   - Passed as parameter to decrypt methods?
   - Injected via constructor?
   - Retrieved internally from KeyService?

### Suggested Patterns from Codebase

- Services use `Arc<Mutex<JsonFileStorage>>` for storage access
- Service container pattern exists in `container.rs`
- Async/await pattern throughout
- Error types defined per module (e.g., `AuthError`, `VaultError`)

---

## 4. Integration Points to Design

### Auth Flow Integration

```
login_with_password()
  |-- decrypt_user_key() [existing]
  |-- generate_session_key() [new]
  |-- encrypt_user_key_with_session() [new]
  |-- store_protected_user_key() [new]
  `-- return session_key
```

### Vault Command Flow

```
execute_list_items()
  |-- get_session() [from env or flag]
  |-- KeyService::get_user_key(session)
  |-- vault_service.list_items()
  `-- cipher_service.decrypt_ciphers(ciphers, user_key)
```

---

## 5. Non-Functional Considerations

### Performance
- Decryption is fast (AES), should be <1ms per field
- Concern is volume (many items, many fields)
- Consider lazy decryption or field filtering

### Security
- User key should not persist in memory longer than needed
- Consider zeroize for sensitive types
- Avoid logging decrypted values

### Testing
- Unit tests need mock session keys
- Need test vectors for encrypt/decrypt roundtrip
- Cross-compatibility tests with TypeScript CLI

---

## 6. Files for Reference

| File | Relevance |
|------|-----------|
| `docs/research/vault_decryption_research.md` | Detailed SDK API documentation |
| `crates/bw-core/src/services/auth/auth_service.rs` | Current auth flow |
| `crates/bw-core/src/services/vault/cipher_service.rs` | Current decryption placeholders |
| `crates/bw-core/src/services/storage/keys.rs` | Storage key patterns |
| `crates/bw-core/src/services/crypto.rs` | Existing SDK crypto wrappers |
| `crates/bw-cli/src/commands/vault.rs` | Current vault command handlers |

---

## 7. Questions for Architecture Phase

The requirements are clear. The architect should focus on:

1. **Service design**: How do KeyService, AuthService, CipherService interact?
2. **Error handling**: What error types are needed for decryption failures?
3. **Module organization**: Where does protected storage module fit?
4. **Async patterns**: Does KeyService need to be async?
5. **Testing strategy**: How to mock SDK crypto for unit tests?
