# Key Handling Security Analysis

This document compares the security model of the Rust Bitwarden CLI with the official TypeScript CLI to verify that keys and sensitive data are handled securely and consistently.

## Key Hierarchy

Both implementations follow the same key hierarchy:

```
Master Password
    ↓ [PBKDF2/Argon2id - 600k+ iterations or memory-hard]
Master Key (256-bit, derived fresh, never persisted)
    ↓ [decrypts]
User Key (512-bit, encrypted by master key on disk)
    ↓ [wrapped with session key for runtime use]
Protected User Key (on disk, encrypted with session key)
    ↓ [decrypts]
Vault Data (ciphers, folders, collections)
```

## Security Comparison

### Master Key Handling

| Aspect | TypeScript CLI | Rust CLI |
|--------|---------------|----------|
| Derivation | PBKDF2-SHA256 (600k iterations) or Argon2id | Same - via SDK `MasterKey::derive()` |
| Storage | MEMORY only, `clearOn: ["lock", "logout"]` | Never persisted, derived fresh each time |
| Lifetime | Held during login/unlock, cleared on lock | Same - exists only during auth flow |

**Code Reference (Rust):**
```rust
// auth_service.rs:421-449
async fn derive_master_key(...) -> Result<MasterKey, AuthError> {
    let kdf: Kdf = kdf_config.try_into()?;
    tokio::task::spawn_blocking(move || {
        crypto::derive_master_key(&password_str, &email_clone, &kdf)
    }).await??
}
```

### User Key Handling

| Aspect | TypeScript CLI | Rust CLI |
|--------|---------------|----------|
| Encrypted storage | `user_{id}_crypto_userKey` (encrypted with master key) | Same key format |
| Decryption | Master key decrypts on unlock | Same - `crypto::decrypt_user_key()` |
| Runtime storage | Encrypted with session key as `__PROTECTED__{userId}_user_auto` | Same format and location |

**Code Reference (Rust):**
```rust
// auth_service.rs:122-138
let encrypted_protected_key = encrypt_user_key(uk, &session_key)?;
let protected_key = make_protected_key(&user_key_protected_storage_key(&profile.id));
storage.set(&protected_key, &encrypted_protected_key).await?;
```

### Session Key Handling

| Aspect | TypeScript CLI | Rust CLI |
|--------|---------------|----------|
| Purpose | Encrypts user key for `BW_SESSION` runtime use | Same |
| Generation | 64 bytes (512 bits) from CSPRNG | Same - `SymmetricCryptoKey::make_aes256_cbc_hmac_key()` |
| Format | Base64-encoded for `BW_SESSION` env var | Same |
| Memory safety | Plain JS object, GC-dependent clearing | `ZeroizeOnDrop` trait, explicit buffer clearing |

**Code Reference (Rust):**
```rust
// models/auth/session.rs:4,11-15
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Clone, ZeroizeOnDrop)]
pub struct SessionKey {
    encryption_key: [u8; 32],
    mac_key: [u8; 32],
}
```

### Token Storage

| Aspect | TypeScript CLI | Rust CLI |
|--------|---------------|----------|
| Access token | Plaintext JSON on disk (secure storage not supported on CLI) | Same |
| Refresh token | Plaintext JSON on disk | Same |
| Location | `user_{id}_token_accessToken`, `user_{id}_token_refreshToken` | Same key format |

**Note:** The TypeScript CLI's `supportsSecureStorage()` returns `false` for CLI environments, so tokens are stored in plaintext JSON. The Rust CLI matches this behavior for compatibility.

### Lock/Logout Behavior

| Operation | TypeScript CLI | Rust CLI |
|-----------|---------------|----------|
| Lock | Clears protected user key, keeps tokens | Same - removes `__PROTECTED__` key |
| Logout | Clears tokens (sets to null), clears protected key | Same behavior |

**Code Reference (Rust):**
```rust
// auth_service.rs:329-334 (lock)
let protected_key = make_protected_key(&user_key_protected_storage_key(&user_id));
storage.remove(&protected_key).await?;

// auth_service.rs:356-373 (logout)
storage.set(&StorageKey::UserAccessToken.format(Some(&user_id)), &serde_json::Value::Null).await?;
storage.set(&StorageKey::UserRefreshToken.format(Some(&user_id)), &serde_json::Value::Null).await?;
storage.remove(&protected_key).await?;
```

## Rust-Specific Security Improvements

The Rust implementation adds security improvements over the TypeScript CLI:

### 1. Memory Zeroization

```rust
// session.rs - SessionKey implements ZeroizeOnDrop
#[derive(Clone, ZeroizeOnDrop)]
pub struct SessionKey { ... }

// Intermediate buffers are explicitly zeroed
pub fn from_base64(encoded: &str) -> Result<Self, SessionKeyError> {
    let mut bytes = STANDARD.decode(encoded)?;
    // ... use bytes ...
    bytes.zeroize();  // Explicit zeroing before return
    Ok(key)
}
```

### 2. Password Handling with secrecy Crate

```rust
// auth_service.rs:68-74
pub async fn login_with_password(
    &self,
    email: &str,
    password: Secret<String>,  // Password wrapped in Secret<T>
    ...
)
```

The `Secret<T>` wrapper:
- Prevents accidental logging via `Debug` trait
- Requires explicit `.expose_secret()` to access
- Provides clear intent about sensitive data

### 3. Type Safety

Rust's type system prevents common key handling errors:
- `MasterKey` vs `SymmetricCryptoKey` vs `SessionKey` are distinct types
- Compile-time verification of key usage
- No runtime type confusion possible

## Security Dependencies

| Crate | Purpose | Version |
|-------|---------|---------|
| `zeroize` | Memory clearing on drop | workspace |
| `secrecy` | Secret value wrapper | workspace |
| `bitwarden-crypto` | SDK cryptographic operations | workspace |
| `rand` | CSPRNG for key generation | workspace |

## Limitations (Same as TypeScript CLI)

1. **SDK Key Types**: `MasterKey` and `SymmetricCryptoKey` from the SDK don't implement `Zeroize`, so we're limited by SDK behavior.

2. **No mlock()**: Neither implementation uses `mlock()` to prevent sensitive memory from being swapped to disk.

3. **Token Storage**: CLI doesn't support OS-level secure storage (macOS Keychain, Windows Credential Manager), so tokens are plaintext on disk.

## Summary

| Category | Grade | Notes |
|----------|-------|-------|
| Master key handling | A | Never persisted, SDK-derived |
| User key protection | A | Encrypted with session key in protected storage |
| Session key security | A+ | ZeroizeOnDrop, explicit buffer clearing |
| Password handling | A | Secret<String> wrapper |
| Token storage | B | Plaintext on disk (same as TS CLI limitation) |
| Memory safety | A | Rust ownership + zeroize |

**Overall:** The Rust CLI follows the same security model as the TypeScript CLI and adds improvements through Rust's type system, the `secrecy` crate, and explicit zeroization.
