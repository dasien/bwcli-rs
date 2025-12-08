---
enhancement: 09-sdk-integration
agent: architect
task_id: task_1765041038_11828
timestamp: 2025-12-06T12:30:00Z
status: READY_FOR_IMPLEMENTATION
---

# Implementation Plan: SDK Integration

## Executive Summary

This document provides the technical architecture and implementation plan for replacing mock SDK implementations with the real Bitwarden SDK. The integration focuses on three key areas: SDK client initialization, cryptographic operations, and auth service integration.

## Architecture Overview

### Component Diagram

```
┌─────────────────────────────────────────────────────────────────────────┐
│                              CLI Layer                                   │
│                         (bw-cli/commands)                               │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                           Service Layer                                  │
│                           (bw-core)                                     │
├─────────────────┬─────────────────────────────────┬────────────────────┤
│  AuthService    │      ServiceContainer           │   BitwardenApiClient│
│  - login        │      - sdk: Client              │   - HTTP requests   │
│  - unlock       │      - storage: JsonFileStorage │   - token refresh   │
│  - logout       │      - api_client               │                     │
└────────┬────────┴────────────┬────────────────────┴────────────────────┘
         │                     │
         ▼                     ▼
┌──────────────────────────────────────────────────────────────────────────┐
│                         SDK Layer (NEW)                                  │
├──────────────────┬────────────────────┬────────────────────────────────┤
│  bitwarden-core  │  bitwarden-crypto  │  bitwarden-auth               │
│  - Client        │  - MasterKey       │  - AuthClient                  │
│  - ClientSettings│  - Kdf             │  - IdentityClient              │
│  - DeviceType    │  - HashPurpose     │  - RegistrationClient          │
│                  │  - EncString       │                                │
│                  │  - UserKey         │                                │
│                  │  - SymmetricCrypto │                                │
│                  │    Key             │                                │
└──────────────────┴────────────────────┴────────────────────────────────┘
```

### Data Flow: Login with Password

```
User                CLI                AuthService           SDK
 │                   │                      │                  │
 │  bw login email   │                      │                  │
 │──────────────────>│                      │                  │
 │                   │ login_with_password()│                  │
 │                   │─────────────────────>│                  │
 │                   │                      │ fetch_kdf_config │
 │                   │                      │──────────────────>
 │                   │                      │      KdfConfig   │
 │                   │                      │<──────────────────
 │                   │                      │                  │
 │                   │                      │ MasterKey::derive│
 │                   │                      │──────────────────>
 │                   │                      │    MasterKey     │
 │                   │                      │<──────────────────
 │                   │                      │                  │
 │                   │                      │derive_master_key_│
 │                   │                      │hash()            │
 │                   │                      │──────────────────>
 │                   │                      │  password_hash   │
 │                   │                      │<──────────────────
 │                   │                      │                  │
 │                   │                      │ authenticate     │
 │                   │                      │ (API call)       │
 │                   │                      │                  │
 │                   │                      │ decrypt_user_key │
 │                   │                      │──────────────────>
 │                   │                      │   SymmetricKey   │
 │                   │                      │<──────────────────
 │                   │                      │                  │
 │   session_key    │      LoginResult     │                  │
 │<──────────────────│<─────────────────────│                  │
```

## Technical Design

### 1. SDK Client Initialization

**File**: `crates/bw-core/src/services/sdk.rs`

#### Current Implementation
```rust
// Mock Client struct
pub struct Client {
    api_url: String,
    identity_url: String,
}
```

#### Target Implementation
```rust
// Re-export SDK types
pub use bitwarden_core::{Client, ClientSettings, DeviceType};

/// Get the appropriate DeviceType for the current platform
pub fn get_device_type() -> DeviceType {
    #[cfg(target_os = "linux")]
    { DeviceType::LinuxCLI }

    #[cfg(target_os = "macos")]
    { DeviceType::MacOsCLI }

    #[cfg(target_os = "windows")]
    { DeviceType::WindowsCLI }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    { DeviceType::SDK }
}

/// Create the SDK client for all crypto and vault operations
pub fn create_sdk_client(
    api_url: Option<String>,
    identity_url: Option<String>,
) -> Result<Client> {
    let settings = ClientSettings {
        api_url: api_url.unwrap_or_else(|| "https://api.bitwarden.com".to_string()),
        identity_url: identity_url.unwrap_or_else(|| "https://identity.bitwarden.com".to_string()),
        user_agent: format!("Bitwarden CLI/{}", env!("CARGO_PKG_VERSION")),
        device_type: get_device_type(),
        bitwarden_client_version: Some(env!("CARGO_PKG_VERSION").to_string()),
    };

    Ok(Client::new(Some(settings)))
}
```

#### Design Rationale
- **Platform-specific DeviceType**: Uses compile-time `#[cfg]` attributes for platform detection
- **Version embedding**: Uses `env!("CARGO_PKG_VERSION")` for automatic version sync
- **SDK token management**: Uses `Client::new()` with SDK-managed tokens (simplest approach)
- **Fallback to SDK type**: Unknown platforms default to `DeviceType::SDK`

### 2. KDF Configuration Conversion

**File**: `crates/bw-core/src/models/state/kdf.rs`

#### New Code Addition
```rust
use bitwarden_crypto::Kdf;
use std::num::NonZeroU32;

impl TryFrom<&KdfConfig> for Kdf {
    type Error = anyhow::Error;

    fn try_from(config: &KdfConfig) -> Result<Self, Self::Error> {
        match config.kdf {
            KdfType::PBKDF2SHA256 => {
                let iterations = config.kdf_iterations.unwrap_or(600_000);
                Ok(Kdf::PBKDF2 {
                    iterations: NonZeroU32::new(iterations)
                        .ok_or_else(|| anyhow::anyhow!("KDF iterations must be > 0"))?,
                })
            }
            KdfType::Argon2id => {
                let iterations = config.kdf_iterations.unwrap_or(3);
                let memory = config.kdf_memory.unwrap_or(64);
                let parallelism = config.kdf_parallelism.unwrap_or(4);

                Ok(Kdf::Argon2id {
                    iterations: NonZeroU32::new(iterations)
                        .ok_or_else(|| anyhow::anyhow!("KDF iterations must be > 0"))?,
                    memory: NonZeroU32::new(memory)
                        .ok_or_else(|| anyhow::anyhow!("KDF memory must be > 0"))?,
                    parallelism: NonZeroU32::new(parallelism)
                        .ok_or_else(|| anyhow::anyhow!("KDF parallelism must be > 0"))?,
                })
            }
        }
    }
}
```

#### Design Rationale
- **TryFrom trait**: Allows fallible conversion with clear error messages
- **Default values**: Match Bitwarden's default KDF parameters
- **NonZeroU32 validation**: SDK requires non-zero values, caught at conversion time

### 3. Crypto Operations Module

**File**: `crates/bw-core/src/services/crypto.rs` (new file)

#### Implementation
```rust
//! SDK-backed cryptographic operations
//!
//! This module provides thin wrappers around the Bitwarden SDK's crypto operations.

use bitwarden_crypto::{
    CryptoError, EncString, HashPurpose, Kdf, MasterKey, SymmetricCryptoKey,
};

/// Derive a master key from password, email, and KDF configuration
///
/// This wraps SDK's MasterKey::derive() for consistency with CLI patterns.
pub fn derive_master_key(
    password: &str,
    email: &str,
    kdf: &Kdf,
) -> Result<MasterKey, CryptoError> {
    MasterKey::derive(password, email, kdf)
}

/// Hash password for server authentication
///
/// Returns base64-encoded hash suitable for the login API.
pub fn hash_password_for_auth(
    master_key: &MasterKey,
    password: &str,
) -> String {
    master_key
        .derive_master_key_hash(password.as_bytes(), HashPurpose::ServerAuthorization)
        .to_string()
}

/// Decrypt the user's symmetric key using their master key
///
/// The encrypted_key should be in EncString format (type.iv.ct.mac).
pub fn decrypt_user_key(
    master_key: &MasterKey,
    encrypted_key: &str,
) -> Result<SymmetricCryptoKey, CryptoError> {
    let enc_string: EncString = encrypted_key
        .parse()
        .map_err(|_| CryptoError::EncString(
            bitwarden_crypto::EncStringParseError::NoType
        ))?;

    master_key.decrypt_user_key(enc_string)
}
```

#### Design Rationale
- **Thin wrappers**: Functions delegate to SDK, no custom logic
- **Consistent interface**: Functions match the expected CLI usage patterns
- **Clear error handling**: SDK errors propagate directly

### 4. Auth Service Updates

**File**: `crates/bw-core/src/services/auth/auth_service.rs`

#### Key Changes

##### Import Changes
```rust
// Remove:
// use crate::services::auth::mock_crypto;

// Add:
use bitwarden_crypto::{CryptoError, Kdf, MasterKey, SymmetricCryptoKey};
use crate::services::crypto;
```

##### Method Signature Changes

**Before:**
```rust
async fn derive_master_key(
    &self,
    password: &Secret<String>,
    email: &str,
    kdf_config: &KdfConfig,
) -> Result<mock_crypto::MasterKey, AuthError>
```

**After:**
```rust
async fn derive_master_key(
    &self,
    password: &Secret<String>,
    email: &str,
    kdf_config: &KdfConfig,
) -> Result<MasterKey, AuthError>
```

##### Implementation Changes

**derive_master_key:**
```rust
async fn derive_master_key(
    &self,
    password: &Secret<String>,
    email: &str,
    kdf_config: &KdfConfig,
) -> Result<MasterKey, AuthError> {
    let kdf: Kdf = kdf_config.try_into().map_err(|e: anyhow::Error| {
        AuthError::KdfError {
            message: e.to_string(),
        }
    })?;

    let password_clone = password.expose_secret().clone();
    let email_clone = email.to_string();

    // Run KDF in blocking task (CPU-intensive)
    tokio::task::spawn_blocking(move || {
        crypto::derive_master_key(&password_clone, &email_clone, &kdf)
    })
    .await
    .map_err(|e| AuthError::CryptoError {
        message: format!("KDF task failed: {}", e),
    })?
    .map_err(|e: CryptoError| AuthError::CryptoError {
        message: format!("Key derivation failed: {}", e),
    })
}
```

**hash_password_for_auth:**
```rust
async fn hash_password_for_auth(
    &self,
    password: &Secret<String>,
    master_key: &MasterKey,
) -> Result<String, AuthError> {
    let password_str = password.expose_secret().clone();

    // Password hashing is fast (single PBKDF2 iteration), but run in blocking
    // task for consistency with KDF operations
    let master_key_clone = master_key.clone();

    tokio::task::spawn_blocking(move || {
        crypto::hash_password_for_auth(&master_key_clone, &password_str)
    })
    .await
    .map_err(|e| AuthError::CryptoError {
        message: format!("Password hash task failed: {}", e),
    })
}
```

**decrypt_user_key:**
```rust
async fn decrypt_user_key(
    &self,
    encrypted_key: &str,
    master_key: &MasterKey,
) -> Result<SymmetricCryptoKey, AuthError> {
    let encrypted_clone = encrypted_key.to_string();
    let master_key_clone = master_key.clone();

    tokio::task::spawn_blocking(move || {
        crypto::decrypt_user_key(&master_key_clone, &encrypted_clone)
    })
    .await
    .map_err(|e| AuthError::CryptoError {
        message: format!("User key decryption task failed: {}", e),
    })?
    .map_err(|e: CryptoError| AuthError::CryptoError {
        message: format!("User key decryption failed: {}", e),
    })
}
```

#### Design Rationale
- **spawn_blocking usage**: KDF operations are CPU-intensive and should not block the async runtime
- **Error mapping**: SDK's `CryptoError` is converted to CLI's `AuthError` for consistent error handling
- **Clone for thread safety**: Master key is cloned for use in blocking tasks

### 5. Service Container Updates

**File**: `crates/bw-core/src/services/container.rs`

#### Changes
```rust
// Update import
use super::sdk::{create_sdk_client, Client};  // Now re-exports from bitwarden_core

// ServiceContainer struct remains the same - sdk field type is now bitwarden_core::Client
```

#### Design Rationale
- **Minimal changes**: The re-export pattern means container code changes are minimal
- **Type consistency**: `Client` type is now the real SDK client

### 6. Error Handling Integration

**File**: `crates/bw-core/src/services/auth/errors.rs`

#### New Error Variant Integration
```rust
use bitwarden_crypto::CryptoError;

impl From<CryptoError> for AuthError {
    fn from(e: CryptoError) -> Self {
        match e {
            CryptoError::InvalidKey => AuthError::CryptoError {
                message: "Invalid encryption key".to_string(),
            },
            CryptoError::InvalidMac => AuthError::InvalidPassword,
            CryptoError::InsufficientKdfParameters => AuthError::KdfError {
                message: "Insufficient KDF parameters".to_string(),
            },
            other => AuthError::CryptoError {
                message: other.to_string(),
            },
        }
    }
}
```

#### Design Rationale
- **Semantic mapping**: `InvalidMac` typically means wrong password (MAC verification failed during decryption)
- **Preserve detail**: Other errors preserve the SDK error message

### 7. Module Structure Changes

**File**: `crates/bw-core/src/services/auth/mod.rs`

#### Changes
```rust
mod auth_service;
mod errors;
// mod mock_crypto;  // REMOVE
mod session_manager;

pub use auth_service::AuthService;
pub use errors::AuthError;
pub use session_manager::SessionManager;

pub use crate::models::auth::{LoginResult, TwoFactorData, UnlockResult};
```

**File**: `crates/bw-core/src/services/mod.rs`

#### Changes
```rust
mod api;
mod auth;
mod container;
mod sdk;
mod storage;
mod crypto;  // ADD

pub use api::{ApiClient, BitwardenApiClient, Environment};
pub use auth::{AuthError, AuthService, LoginResult, SessionManager, TwoFactorData, UnlockResult};
pub use container::ServiceContainer;
pub use sdk::{create_sdk_client, Client, ClientSettings, DeviceType, get_device_type};
pub use storage::{JsonFileStorage, Storage, StorageError};
pub use crypto::{derive_master_key, hash_password_for_auth, decrypt_user_key};  // ADD
```

## Implementation Phases

### Phase 1: SDK Client Setup (Low Risk)

**Tasks:**
1. Update `sdk.rs` to re-export SDK types
2. Add `get_device_type()` function
3. Update `create_sdk_client()` to return real `Client`
4. Verify compilation succeeds

**Files Modified:**
- `crates/bw-core/src/services/sdk.rs`

**Verification:**
```bash
cargo build -p bw-core
```

### Phase 2: KDF Conversion (Low Risk)

**Tasks:**
1. Add `TryFrom<&KdfConfig> for Kdf` implementation
2. Add necessary imports to `kdf.rs`
3. Add unit tests for conversion

**Files Modified:**
- `crates/bw-core/src/models/state/kdf.rs`

**Verification:**
```bash
cargo test -p bw-core kdf
```

### Phase 3: Crypto Module (Medium Risk)

**Tasks:**
1. Create `crates/bw-core/src/services/crypto.rs`
2. Implement wrapper functions
3. Add to module exports
4. Add unit tests

**Files Created:**
- `crates/bw-core/src/services/crypto.rs`

**Files Modified:**
- `crates/bw-core/src/services/mod.rs`

**Verification:**
```bash
cargo test -p bw-core crypto
```

### Phase 4: Auth Service Migration (High Risk)

**Tasks:**
1. Update imports in `auth_service.rs`
2. Update method signatures to use SDK types
3. Update method implementations
4. Add `From<CryptoError>` impl to `errors.rs`
5. Update tests

**Files Modified:**
- `crates/bw-core/src/services/auth/auth_service.rs`
- `crates/bw-core/src/services/auth/errors.rs`

**Verification:**
```bash
cargo test -p bw-core auth
```

### Phase 5: Cleanup (Low Risk)

**Tasks:**
1. Remove `mock_crypto.rs`
2. Remove mock_crypto from `auth/mod.rs` exports
3. Remove `sha2` dependency if no longer needed elsewhere

**Files Deleted:**
- `crates/bw-core/src/services/auth/mock_crypto.rs`

**Files Modified:**
- `crates/bw-core/src/services/auth/mod.rs`
- `crates/bw-core/Cargo.toml` (optional cleanup)

**Verification:**
```bash
cargo build --all
cargo test --all
cargo clippy --all-features --all-targets
```

### Phase 6: Integration Testing

**Tasks:**
1. Verify password login with test account
2. Verify unlock operation
3. Verify API key login
4. Test error scenarios (wrong password, invalid KDF params)

**Test Commands:**
```bash
./target/debug/bw login user@example.com
./target/debug/bw unlock
./target/debug/bw logout
```

## API/Interface Specification

### Public API Changes

| Before | After | Breaking |
|--------|-------|----------|
| `sdk::Client` (mock) | `bitwarden_core::Client` | No* |
| `mock_crypto::MasterKey` | `bitwarden_crypto::MasterKey` | No** |
| `mock_crypto::UserKey` | `bitwarden_crypto::SymmetricCryptoKey` | No** |

*Container returns the same interface (`&Client`)
**Types are internal to auth service, not exposed publicly

### New Public Exports

```rust
// From services::sdk
pub use bitwarden_core::{Client, ClientSettings, DeviceType};
pub fn get_device_type() -> DeviceType;

// From services::crypto
pub fn derive_master_key(password: &str, email: &str, kdf: &Kdf) -> Result<MasterKey, CryptoError>;
pub fn hash_password_for_auth(master_key: &MasterKey, password: &str) -> String;
pub fn decrypt_user_key(master_key: &MasterKey, encrypted_key: &str) -> Result<SymmetricCryptoKey, CryptoError>;
```

## Testing Strategy

### Unit Tests

| Component | Test Focus | File |
|-----------|------------|------|
| KDF Conversion | PBKDF2 and Argon2id conversion | `kdf.rs` |
| Crypto Module | SDK wrapper functions | `crypto.rs` |
| Error Mapping | CryptoError to AuthError | `errors.rs` |
| SDK Client | Client creation | `sdk.rs` |

### Integration Tests

| Test | Description | Prerequisites |
|------|-------------|---------------|
| Password Login | Full login flow with real SDK crypto | Test credentials |
| Unlock | Vault unlock with stored user key | Prior login |
| Wrong Password | Verify InvalidMac maps to InvalidPassword | Test credentials |
| Invalid KDF | Verify KDF validation errors | None |

### Test Vectors (from SDK)

**PBKDF2:**
```
Password: "asdfasdf"
Email: "test@bitwarden.com"
KDF: PBKDF2 { iterations: 100_000 }
Expected Hash: "wmyadRMyBZOH7P/a/ucTCbSghKgdzDpPqUnu/DAVtSw="
```

**Argon2id:**
```
Password: "asdfasdf"
Salt: "test_salt"
KDF: Argon2id { iterations: 4, memory: 32, parallelism: 2 }
Expected Hash: "PR6UjYmjmppTYcdyTiNbAhPJuQQOmynKbdEl1oyi/iQ="
```

## Security Considerations

### Sensitive Data Handling

1. **Password in memory**: Use `secrecy::Secret<String>` for password handling (existing pattern)
2. **Master key**: SDK's `MasterKey` uses pinned heap allocation with automatic zeroing
3. **User key**: `SymmetricCryptoKey` similarly uses secure memory patterns
4. **Logging**: Never log passwords, master keys, or user keys

### SDK Security Features (Inherited)

- AES-256-CBC/GCM encryption with HMAC-SHA256
- Constant-time MAC comparison (via SDK)
- PBKDF2-SHA256 and Argon2id key derivation
- Zeroizing allocator for memory safety

### Validation Requirements

| Input | Validation | Location |
|-------|------------|----------|
| KDF iterations | > 0, SDK validates minimums | `KdfConfig::try_into()` |
| Email | Trimmed, lowercased (by SDK) | `MasterKey::derive()` |
| Encrypted key | Valid EncString format | `decrypt_user_key()` |

## Performance Considerations

### KDF Performance

Real KDF operations are CPU-intensive:
- **PBKDF2**: ~600,000 iterations (default), 100-500ms typical
- **Argon2id**: Memory-hard, 500ms-2s typical

These are expected and handled with `spawn_blocking` to avoid blocking the async runtime.

### Memory Usage

SDK crypto operations allocate pinned heap memory:
- MasterKey: 32 bytes
- SymmetricCryptoKey: 64 bytes (32 enc + 32 mac)

This is negligible and managed by SDK.

## Migration Notes

### Backwards Compatibility

1. **Stored credentials**: Encrypted user keys remain compatible (EncString format unchanged)
2. **Session keys**: BW_SESSION format unchanged (CLI-generated, not SDK)
3. **CLI commands**: No changes to command arguments or output formats

### Data Migration

None required. The SDK handles the same EncString formats as the mock implementation.

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| SDK API change | Medium | Medium | Pin SDK version, abstraction layer |
| Wrong KDF params | Low | High | Validation before SDK call |
| Memory leaks | Low | Low | SDK handles cleanup |
| Test failures | Medium | Low | Comprehensive test suite |

## Dependencies

### Required Crates (already in Cargo.toml)

```toml
bitwarden-core = { workspace = true, features = ["internal"] }
bitwarden-crypto.workspace = true
bitwarden-auth.workspace = true
```

### Optional Cleanup

After migration, these may be removable if unused elsewhere:
- `sha2` (was used by mock_crypto)

## Verification Checklist

- [ ] `cargo build --all` succeeds
- [ ] `cargo test --all` passes
- [ ] `cargo clippy --all-features --all-targets` has no warnings
- [ ] `cargo fmt --check` passes
- [ ] Password login works with test account
- [ ] Unlock works with stored credentials
- [ ] Wrong password returns appropriate error
- [ ] API key login works
- [ ] No mock crypto code remains in production paths

## Appendix A: File Change Summary

| File | Action | Complexity |
|------|--------|------------|
| `services/sdk.rs` | Modify | Low |
| `services/crypto.rs` | Create | Medium |
| `services/mod.rs` | Modify | Low |
| `services/auth/auth_service.rs` | Modify | High |
| `services/auth/errors.rs` | Modify | Low |
| `services/auth/mod.rs` | Modify | Low |
| `services/auth/mock_crypto.rs` | Delete | Low |
| `models/state/kdf.rs` | Modify | Medium |

## Appendix B: SDK Type Reference

| CLI Context | SDK Type | Purpose |
|-------------|----------|---------|
| Master key derivation | `bitwarden_crypto::MasterKey` | Password-derived key |
| User encryption key | `bitwarden_crypto::SymmetricCryptoKey` | Vault encryption |
| KDF configuration | `bitwarden_crypto::Kdf` | PBKDF2/Argon2id params |
| Password hash purpose | `bitwarden_crypto::HashPurpose` | Server vs local auth |
| Encrypted strings | `bitwarden_crypto::EncString` | Encrypted data format |
| SDK client | `bitwarden_core::Client` | Main SDK interface |
| Client config | `bitwarden_core::ClientSettings` | API URLs, device type |
| Device type | `bitwarden_core::DeviceType` | Platform identifier |
