---
enhancement: 14-sdk-migration-vault
agent: implementer
task_id: task_1765657335_68890
timestamp: 2025-12-13T19:45:00Z
status: READY_FOR_TESTING
---

# SDK Vault Migration Implementation Summary

## Overview

This implementation adds the foundational SDK vault bridge infrastructure that enables migration from direct `bitwarden-crypto` encryption/decryption to using the official Bitwarden SDK `VaultClient` for vault operations.

## What Was Implemented

### 1. SDK Vault Bridge Module (`crates/bw-core/src/services/vault/sdk_bridge.rs`)

A new bridge module that provides:

#### SDK Crypto Initialization

```rust
pub async fn initialize_crypto(
    &self,
    user_key: &SymmetricCryptoKey,
    email: &str,
    kdf_config: &KdfConfig,
    private_key: Option<&str>,
) -> Result<(), VaultError>
```

Initializes the SDK's internal KeyStore with the user's decrypted symmetric key, enabling vault encryption/decryption operations through the SDK's `VaultClient`.

#### Cipher Operations

- `decrypt_cipher(&self, cipher: &Cipher)` - Decrypt a single cipher
- `decrypt_ciphers(&self, ciphers: &[Cipher])` - Decrypt multiple ciphers
- `encrypt_cipher(&self, cipher_view: &CipherView)` - Encrypt a cipher view

#### Folder Operations

- `decrypt_folder(&self, folder: &Folder)` - Decrypt a single folder
- `decrypt_folders(&self, folders: &[Folder])` - Decrypt multiple folders
- `encrypt_folder_name(&self, name: &str)` - Encrypt a folder name

#### Collection Operations

- `decrypt_collection(&self, collection: &Collection)` - Decrypt a single collection
- `decrypt_collections(&self, collections: &[Collection])` - Decrypt multiple collections

### 2. Type Conversion System

The implementation uses two strategies for type conversion:

#### JSON-Based Conversion (Encrypted Types)

For encrypted types (`Cipher`, `Folder`, `Collection`), JSON serialization is used because:
- CLI models and SDK models use compatible `camelCase` JSON formats
- Some SDK internal types (e.g., `Login`, `Card`, `Identity`) are not publicly exported
- JSON conversion avoids direct struct construction limitations

```rust
fn cli_cipher_to_sdk_via_json(cli: &Cipher) -> Result<bitwarden_vault::Cipher, VaultError>
fn sdk_cipher_to_cli_via_json(sdk: &bitwarden_vault::Cipher) -> Result<Cipher, VaultError>
```

#### Direct Field Mapping (View Types)

For decrypted view types where SDK exports are available, direct field mapping provides type safety:

```rust
fn sdk_cipher_view_to_cli(sdk: &bitwarden_vault::CipherView) -> Result<CipherView, VaultError>
fn cli_cipher_view_to_sdk(cli: &CipherView) -> Result<bitwarden_vault::CipherView, VaultError>
```

### 3. KdfConfig Extension

Added `to_sdk_kdf()` method to convert CLI KDF configuration to SDK's `Kdf` enum:

```rust
impl KdfConfig {
    pub fn to_sdk_kdf(&self) -> Kdf
}
```

Supports both PBKDF2-SHA256 and Argon2id KDF types.

### 4. Dependencies Added

- Added `bitwarden-collections` to workspace and bw-core dependencies

### 5. Error Types

Added new error variant for crypto initialization:

```rust
#[error("SDK crypto initialization failed: {0}")]
CryptoInitFailed(String),
```

## Files Modified

| File | Change |
|------|--------|
| `Cargo.toml` (workspace) | Added `bitwarden-collections` dependency |
| `crates/bw-core/Cargo.toml` | Added `bitwarden-collections` dependency |
| `crates/bw-core/src/services/vault/mod.rs` | Added `sdk_bridge` module export |
| `crates/bw-core/src/services/vault/errors.rs` | Added `CryptoInitFailed` error variant |

## Files Created

| File | Description |
|------|-------------|
| `crates/bw-core/src/services/vault/sdk_bridge.rs` | New SDK vault bridge implementation |

## Build Status

- **Build**: Passes with minor warnings (dead code for unused API methods)
- **Warnings**: 2 warnings about unused fields/methods in existing code (not related to this implementation)

## Architecture Notes

### Why JSON Serialization for Encrypted Types?

The SDK's encrypted types (`Login`, `Card`, `Identity`, etc.) are marked `pub(crate)` and not exported from `bitwarden_vault`. This prevents direct struct construction. JSON serialization works because:

1. Both CLI and SDK use serde with `#[serde(rename_all = "camelCase")]`
2. Field structures are compatible (matching Bitwarden API format)
3. The SDK's `Cipher` struct can be deserialized from compatible JSON

### SDK Initialization Pattern

The SDK uses `InitUserCryptoRequest` with `InitUserCryptoMethod::DecryptedKey` to initialize crypto state:

```rust
InitUserCryptoMethod::DecryptedKey {
    decrypted_user_key: user_key.to_base64().to_string()
}
```

This allows initializing with an already-decrypted key (from protected storage), avoiding re-derivation.

## Integration Points

This implementation provides the foundation but does **not** yet integrate with:

1. **CipherService** - Still uses direct `bitwarden-crypto` for encryption/decryption
2. **WriteService** - Still uses direct encryption for create/edit operations
3. **VaultService** - Not yet configured to initialize SDK crypto on demand

### Next Steps for Full Integration

1. **Modify CipherService** to optionally use `SdkVaultBridge` instead of direct crypto
2. **Add crypto initialization** to VaultService before operations
3. **Update WriteService** to use SDK for encryption
4. **Remove direct bitwarden-crypto usage** from vault operations
5. **Testing** with real vault data to verify type compatibility

## Testing Recommendations

1. **Unit Tests**
   - Test JSON conversion round-trips for all cipher types
   - Test KDF configuration conversion
   - Test SDK initialization with mock keys

2. **Integration Tests**
   - Decrypt real vault ciphers from synced storage
   - Encrypt new ciphers and verify compatibility with server
   - Test all cipher types (Login, SecureNote, Card, Identity)

3. **Compatibility Tests**
   - Verify encrypted data can be decrypted by TypeScript CLI
   - Verify TypeScript-encrypted data can be decrypted by Rust CLI

## Risk Assessment

| Risk | Mitigation |
|------|------------|
| JSON type mismatch | Extensive field mapping tests, graceful error handling |
| SDK API changes | Use pinned SDK version, integration tests |
| Key initialization failures | Detailed error messages, fallback to current implementation |
| Performance regression | JSON conversion adds overhead; profile before full migration |

## Conclusion

This implementation provides the complete infrastructure for SDK vault migration. The bridge module is ready for integration with the existing vault services. The design maintains backward compatibility while enabling gradual migration to SDK-based encryption/decryption.
