# SDK Integration Developer Guide

This guide explains how the Bitwarden CLI integrates with the Bitwarden SDK for cryptographic operations.

## Architecture Overview

The CLI uses a **thin wrapper architecture** where all cryptographic operations are delegated to the Bitwarden SDK. No custom cryptography is implemented in the CLI.

```
┌─────────────────────────────────────────────────────────────────┐
│                         CLI Commands                             │
│                    (bw login, bw unlock, etc.)                  │
└─────────────────────────────────────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────┐
│                       AuthService (bw-core)                      │
│  ┌──────────────────┐  ┌──────────────────┐  ┌───────────────┐ │
│  │ login_password() │  │ unlock()         │  │ logout()      │ │
│  └────────┬─────────┘  └────────┬─────────┘  └───────────────┘ │
│           │                     │                               │
└───────────┼─────────────────────┼───────────────────────────────┘
            │                     │
            ▼                     ▼
┌─────────────────────────────────────────────────────────────────┐
│                     Crypto Module (bw-core)                      │
│  ┌───────────────────┐  ┌─────────────────┐  ┌───────────────┐ │
│  │derive_master_key()│  │hash_password()  │  │decrypt_key()  │ │
│  └────────┬──────────┘  └────────┬────────┘  └───────┬───────┘ │
└───────────┼──────────────────────┼───────────────────┼──────────┘
            │                      │                   │
            ▼                      ▼                   ▼
┌─────────────────────────────────────────────────────────────────┐
│                     Bitwarden SDK (bitwarden-crypto)             │
│  MasterKey::derive()    derive_master_key_hash()   decrypt_key()│
└─────────────────────────────────────────────────────────────────┘
```

## Key Components

### 1. SDK Client (`services/sdk.rs`)

The SDK client provides the main interface to the Bitwarden SDK:

```rust
use bw_core::services::{create_sdk_client, Client, ClientSettings, DeviceType};

// Create with default Bitwarden URLs
let client = create_sdk_client(None, None)?;

// Create with custom server URLs
let client = create_sdk_client(
    Some("https://api.example.com".to_string()),
    Some("https://identity.example.com".to_string()),
)?;
```

The `get_device_type()` function returns the appropriate `DeviceType` for the current platform:
- Linux: `DeviceType::LinuxCLI`
- macOS: `DeviceType::MacOsCLI`
- Windows: `DeviceType::WindowsCLI`
- Other: `DeviceType::SDK`

### 2. Crypto Module (`services/crypto.rs`)

The crypto module provides thin wrappers around SDK crypto operations:

```rust
use bitwarden_crypto::Kdf;
use bw_core::services::{derive_master_key, hash_password_for_auth, decrypt_user_key};
use std::num::NonZeroU32;

// Configure KDF
let kdf = Kdf::PBKDF2 {
    iterations: NonZeroU32::new(600_000).unwrap(),
};

// Derive master key from password
let master_key = derive_master_key("password", "user@example.com", &kdf)?;

// Hash password for server authentication
let password_hash = hash_password_for_auth(&master_key, "password");

// Decrypt user key (from server response)
let user_key = decrypt_user_key(&master_key, "2.encrypted_key_string...")?;
```

### 3. KDF Configuration (`models/state/kdf.rs`)

The `KdfConfig` struct stores KDF parameters and converts to SDK types:

```rust
use bw_core::models::state::{KdfConfig, KdfType};
use bitwarden_crypto::Kdf;

// PBKDF2 configuration
let pbkdf2_config = KdfConfig {
    kdf: KdfType::PBKDF2SHA256,
    kdf_iterations: Some(600_000),
    kdf_memory: None,
    kdf_parallelism: None,
};

// Argon2id configuration
let argon2_config = KdfConfig {
    kdf: KdfType::Argon2id,
    kdf_iterations: Some(3),      // iterations
    kdf_memory: Some(64),         // MB
    kdf_parallelism: Some(4),     // threads
};

// Convert to SDK type
let sdk_kdf: Kdf = (&pbkdf2_config).try_into()?;
```

**Default Parameters:**
| KDF Type | Parameter | Default Value |
|----------|-----------|---------------|
| PBKDF2 | iterations | 600,000 |
| Argon2id | iterations | 3 |
| Argon2id | memory | 64 MB |
| Argon2id | parallelism | 4 |

### 4. Error Handling (`services/auth/errors.rs`)

SDK crypto errors are mapped to CLI-specific auth errors:

| SDK Error | CLI Error | User Message |
|-----------|-----------|--------------|
| `CryptoError::InvalidKey` | `AuthError::CryptoOperationFailed` | "Invalid encryption key" |
| `CryptoError::InvalidMac` | `AuthError::InvalidPassword` | "Invalid master password" |
| `CryptoError::InsufficientKdfParameters` | `AuthError::KdfError` | "Insufficient KDF parameters" |
| Other | `AuthError::CryptoOperationFailed` | Error message from SDK |

The `InvalidMac` -> `InvalidPassword` mapping is intentional: MAC verification failure during decryption typically indicates the wrong password was used.

## Login Flow

The complete login flow with SDK integration:

```
1. User enters email + password
                │
                ▼
2. Fetch KDF config from server
   GET /identity/accounts/prelogin
                │
                ▼
3. Convert KdfConfig → Kdf
   (&kdf_config).try_into()?
                │
                ▼
4. Derive master key (CPU-intensive)
   derive_master_key(password, email, &kdf)
   [Runs in spawn_blocking]
                │
                ▼
5. Hash password for server auth
   hash_password_for_auth(&master_key, password)
                │
                ▼
6. Authenticate with server
   POST /identity/token
   { email, masterPasswordHash, ... }
                │
                ▼
7. Decrypt user key from response
   decrypt_user_key(&master_key, encrypted_user_key)
                │
                ▼
8. Store session + encrypted key
```

## Async Considerations

KDF operations are CPU-intensive (can take 100ms-2s depending on parameters). They are run in `spawn_blocking` to avoid blocking the async runtime:

```rust
// KDF derivation - run in blocking task
let master_key = tokio::task::spawn_blocking(move || {
    derive_master_key(&password, &email, &kdf)
})
.await??;

// Password hashing - fast, but also in blocking for consistency
let hash = tokio::task::spawn_blocking(move || {
    hash_password_for_auth(&master_key, &password)
})
.await??;
```

## Security Considerations

1. **Never bypass SDK crypto**: All encryption/decryption must go through SDK
2. **Secure memory**: SDK uses `zeroize` for automatic memory clearing
3. **No logging**: Never log passwords, master keys, or user keys
4. **Error messages**: Don't leak crypto details in user-facing messages
5. **Constant-time**: SDK handles constant-time comparisons internally

## SDK Dependencies

The CLI uses these SDK crates:

```toml
# Cargo.toml
bitwarden-core = { workspace = true, features = ["internal"] }
bitwarden-crypto = { workspace = true }
```

Key types from each:

| Crate | Types Used |
|-------|------------|
| `bitwarden-core` | `Client`, `ClientSettings`, `DeviceType` |
| `bitwarden-crypto` | `MasterKey`, `SymmetricCryptoKey`, `Kdf`, `HashPurpose`, `EncString`, `CryptoError` |

## Testing

### Unit Tests

Test vectors from the SDK are used to validate integration:

```rust
#[test]
fn test_derive_master_key_pbkdf2() {
    let password = "asdfasdf";
    let email = "test@bitwarden.com";
    let kdf = Kdf::PBKDF2 { iterations: NonZeroU32::new(100_000).unwrap() };

    let master_key = derive_master_key(password, email, &kdf).unwrap();
    let hash = hash_password_for_auth(&master_key, password);

    // Expected from SDK test vectors
    assert_eq!(hash, "wmyadRMyBZOH7P/a/ucTCbSghKgdzDpPqUnu/DAVtSw=");
}
```

### Running Tests

```bash
# Run all crypto tests
cargo test -p bw-core crypto

# Run KDF tests
cargo test -p bw-core kdf

# Run auth error tests
cargo test -p bw-core auth::errors
```

## Troubleshooting

### Common Issues

**"Invalid master password" on unlock**
- The stored encrypted user key couldn't be decrypted with the provided password
- SDK returns `InvalidMac` when MAC verification fails (wrong key)
- Solution: Use the correct password or re-login

**"KDF iterations must be > 0"**
- Server returned invalid KDF parameters
- Solution: Check server configuration, ensure prelogin endpoint returns valid params

**"Key derivation error"**
- KDF operation failed (very rare with valid parameters)
- Could indicate memory issues with Argon2id (high memory requirement)
- Solution: Check system resources, try with lower memory setting

### Debug Logging

The SDK doesn't expose internal state for security, but you can verify:

```bash
# Check that crypto operations complete without error
RUST_LOG=debug cargo run -- login user@example.com
```
