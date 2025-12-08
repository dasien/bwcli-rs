---
enhancement: 09-sdk-integration
agent: requirements-analyst
task_id: task_1765040755_8731
timestamp: 2025-12-06T12:00:00Z
status: READY_FOR_ARCHITECTURE
---

# Requirements Analysis: SDK Integration

## Executive Summary

This enhancement replaces mock SDK implementations with the real Bitwarden SDK from `../sdk-internal/` to enable actual cryptographic operations, authentication flows, and vault management. The SDK dependencies are already configured in `Cargo.toml`, and the codebase compiles successfully with them. The primary work involves replacing mock types with real SDK types throughout the authentication and crypto layers.

## User Story

> As a CLI user, I want the CLI to use the real Bitwarden SDK so that all cryptographic operations, authentication, and vault management work correctly with my Bitwarden account.

## Current State Analysis

### Existing Mock Implementations

1. **Mock SDK Client** (`crates/bw-core/src/services/sdk.rs:8-36`)
   - Custom `Client` struct with `api_url` and `identity_url` fields
   - `create_sdk_client()` function returns mock `Client`
   - Currently does not use any SDK functionality

2. **Mock Crypto Operations** (`crates/bw-core/src/services/auth/mock_crypto.rs`)
   - `MasterKey` - 32-byte key structure (mock uses SHA256 hash)
   - `UserKey` - 64-byte key structure (mock uses base64 encoding)
   - `derive_master_key()` - Uses SHA256 instead of PBKDF2/Argon2id
   - `hash_password()` - Uses SHA256 instead of proper PBKDF2
   - `decrypt_user_key()` - Stub that doesn't actually decrypt
   - `encrypt_with_key()` / `decrypt_with_key()` - Base64 only (not encrypted)

3. **Auth Service Integration** (`crates/bw-core/src/services/auth/auth_service.rs`)
   - Uses `mock_crypto::MasterKey`, `mock_crypto::UserKey`
   - Calls `mock_crypto::derive_master_key()`, `mock_crypto::hash_password()`, `mock_crypto::decrypt_user_key()`
   - Contains TODO comments indicating SDK replacement is needed

4. **Service Container** (`crates/bw-core/src/services/container.rs`)
   - Uses mock `sdk::Client` type
   - Returns `&Client` via `sdk()` method

### SDK Availability

The Bitwarden SDK is available at `../sdk-internal/` and provides:

| Crate | Purpose | Key Types/Functions |
|-------|---------|---------------------|
| `bitwarden-core` | Core client, settings, API config | `Client`, `ClientSettings`, `DeviceType` |
| `bitwarden-crypto` | Cryptographic operations | `MasterKey`, `UserKey`, `Kdf`, `EncString`, `HashPurpose` |
| `bitwarden-auth` | Authentication flows | `AuthClient`, `AuthClientExt`, `IdentityClient` |
| `bitwarden-vault` | Vault operations | (out of scope for MVP) |
| `bitwarden-generators` | Password/passphrase generation | (should-have) |
| `bitwarden-encoding` | Base64/encoding utilities | `B64` |
| `bitwarden-error` | Error types | Error handling support |

## Functional Requirements

### FR-1: Replace Mock SDK Client

**Description**: Replace the mock `Client` struct in `sdk.rs` with the real `bitwarden_core::Client`.

**Current Mock Location**: `crates/bw-core/src/services/sdk.rs:8-14`

**SDK Replacement**:
```rust
use bitwarden_core::{Client, ClientSettings, DeviceType};
```

**Acceptance Criteria**:
- [ ] Mock `Client` struct is removed
- [ ] `bitwarden_core::Client` is imported and re-exported
- [ ] `ClientSettings` and `DeviceType` are exported for use by other components
- [ ] Project compiles with real SDK types

### FR-2: Configure SDK Client Settings

**Description**: Update `create_sdk_client()` to create a properly configured SDK client.

**SDK API** (from `bitwarden-core/src/client/client_settings.rs`):
```rust
ClientSettings {
    identity_url: String,
    api_url: String,
    user_agent: String,
    device_type: DeviceType,
    bitwarden_client_version: Option<String>,
}
```

**Acceptance Criteria**:
- [ ] `create_sdk_client()` accepts optional API and Identity URLs
- [ ] Defaults to production URLs (`https://api.bitwarden.com`, `https://identity.bitwarden.com`)
- [ ] Sets appropriate `DeviceType` for CLI (platform-specific: `LinuxCLI`, `MacOsCLI`, `WindowsCLI`)
- [ ] Sets user agent string identifying CLI and version
- [ ] Client initializes without error with default settings
- [ ] Client initializes without error with custom URL settings

### FR-3: Replace Mock Crypto with SDK Crypto

**Description**: Replace mock crypto operations with SDK equivalents.

**Mock Functions to Replace**:

| Mock Function | SDK Replacement |
|--------------|-----------------|
| `derive_master_key()` | `MasterKey::derive(password, email, &kdf)` |
| `hash_password()` | `master_key.derive_master_key_hash(password, HashPurpose::ServerAuthorization)` |
| `decrypt_user_key()` | `master_key.decrypt_user_key(encrypted_user_key)` |

**SDK Types Required**:
- `bitwarden_crypto::MasterKey`
- `bitwarden_crypto::Kdf` (replaces `KdfConfig`)
- `bitwarden_crypto::HashPurpose`
- `bitwarden_crypto::EncString`
- `bitwarden_crypto::UserKey`

**Acceptance Criteria**:
- [ ] `mock_crypto.rs` is removed from production code paths
- [ ] `auth_service.rs` uses `bitwarden_crypto::MasterKey::derive()`
- [ ] Password hashing uses `derive_master_key_hash()` with `HashPurpose::ServerAuthorization`
- [ ] User key decryption uses `master_key.decrypt_user_key()`
- [ ] KDF configuration is converted to `bitwarden_crypto::Kdf` enum

### FR-4: Update Auth Service for SDK Types

**Description**: Update `AuthService` to use real SDK types for authentication.

**Current Mock Usage** (`auth_service.rs`):
- Line 11: `use ... mock_crypto`
- Line 323: `-> Result<mock_crypto::MasterKey, ...>`
- Line 349-350: `mock_crypto::MasterKey`
- Line 374: `-> Result<mock_crypto::UserKey, ...>`

**Acceptance Criteria**:
- [ ] Import `bitwarden_crypto::{MasterKey, Kdf, HashPurpose, EncString}`
- [ ] `derive_master_key()` returns `bitwarden_crypto::MasterKey`
- [ ] `hash_password_for_auth()` uses SDK's `derive_master_key_hash()`
- [ ] `decrypt_user_key()` uses SDK's decryption
- [ ] KDF configuration properly converted between CLI's `KdfConfig` and SDK's `Kdf`

### FR-5: Update Service Container

**Description**: Ensure `ServiceContainer` uses the real SDK client type.

**Acceptance Criteria**:
- [ ] `sdk` field uses `bitwarden_core::Client`
- [ ] `sdk()` method returns reference to real SDK client
- [ ] Container creation initializes real SDK client

### FR-6: Error Handling for SDK Errors

**Description**: Handle SDK-specific errors gracefully.

**SDK Error Types**:
- `bitwarden_crypto::CryptoError`
- `bitwarden_error` types

**Acceptance Criteria**:
- [ ] SDK errors are converted to CLI's `AuthError` or appropriate error type
- [ ] Error messages are user-friendly and actionable
- [ ] No sensitive data is leaked in error messages

## Non-Functional Requirements

### NFR-1: Performance

- SDK operations should not add significant overhead compared to mock operations
- KDF operations (PBKDF2/Argon2id) are inherently CPU-intensive (expected)
- No memory leaks from SDK usage

### NFR-2: Security

- SDK handles all cryptographic operations - do not bypass
- Use SDK's secure memory handling (zeroize) automatically
- Never log SDK internal state or crypto material
- SDK already implements constant-time comparisons

### NFR-3: Compatibility

- Maintain existing CLI command interfaces
- No breaking changes to command arguments or outputs
- Existing stored credentials should continue to work

### NFR-4: Reliability

- Handle SDK errors gracefully with clear messages
- Provide actionable guidance for failures
- Support both PBKDF2 and Argon2id KDF types

## Technical Flags for Architecture

### TF-1: KDF Type Conversion

The CLI uses its own `KdfConfig`/`KdfType` enums while the SDK uses `bitwarden_crypto::Kdf`. A conversion layer is needed:

**CLI Types** (`crates/bw-core/src/models/state/mod.rs`):
```rust
pub enum KdfType {
    PBKDF2SHA256,
    Argon2id,
}

pub struct KdfConfig {
    pub kdf: KdfType,
    pub kdf_iterations: Option<u32>,
    pub kdf_memory: Option<u32>,
    pub kdf_parallelism: Option<u32>,
}
```

**SDK Type** (`bitwarden-crypto/src/keys/kdf.rs`):
```rust
pub enum Kdf {
    PBKDF2 { iterations: NonZeroU32 },
    Argon2id {
        iterations: NonZeroU32,
        memory: NonZeroU32,
        parallelism: NonZeroU32,
    },
}
```

### TF-2: DeviceType Selection

The SDK provides platform-specific CLI device types:
- `DeviceType::LinuxCLI = 25`
- `DeviceType::MacOsCLI = 24`
- `DeviceType::WindowsCLI = 23`

Architecture should determine how to select at compile-time vs runtime.

### TF-3: Feature Flags

The `bitwarden-core` crate requires `features = ["internal"]` for full functionality. This is already configured in `bw-core/Cargo.toml`.

### TF-4: SDK Client Token Management

The SDK provides two client creation modes:
1. `Client::new(settings)` - SDK-managed tokens
2. `Client::new_with_client_tokens(settings, tokens)` - Client-managed tokens

Architecture should determine which mode is appropriate for CLI's session management.

## Integration Points

### IP-1: Storage Integration

The CLI's storage system (`JsonFileStorage`) stores:
- `userProfile` - User profile data
- `kdfConfig` - KDF configuration
- `accessToken` (secure) - API access token
- `refreshToken` (secure) - Refresh token
- `userKey` (secure) - Encrypted user key

The SDK client may need access to tokens for API requests.

### IP-2: API Client Integration

The CLI has its own `BitwardenApiClient` which handles API communication. The SDK also has internal API configuration. Architecture should determine if:
- CLI continues to use its own API client
- CLI migrates to SDK's API handling
- Both coexist for different purposes

### IP-3: Auth Flow Integration

Current auth flow in `auth_service.rs`:
1. Fetch KDF config from server (prelogin)
2. Derive master key
3. Hash password
4. Authenticate with server
5. Decrypt user key
6. Fetch profile
7. Generate session key
8. Persist auth state

The SDK provides `AuthClient` and `IdentityClient` which may handle some of these steps.

## Constraints

### C-1: SDK Path Dependency

SDK must be available at `../sdk-internal/` relative to the CLI project. This is a development-time constraint.

### C-2: Rust Version

SDK requires Rust 1.85+ and Edition 2024 (already configured in CLI).

### C-3: SDK API Stability

The SDK is marked as "Internal crate for the bitwarden crate. Do not use." in its package descriptions. This indicates the API may change. Architecture should consider abstraction layers.

### C-4: Backwards Compatibility

Must not break existing CLI command interfaces. Users should not need to re-authenticate after upgrade (if encrypted key format is compatible).

## Out of Scope

The following are explicitly out of scope for this enhancement:

1. **Full Vault Sync** - Covered by separate enhancement
2. **Cipher CRUD Operations** - Covered by separate enhancement
3. **Organization Key Handling** - Separate enhancement
4. **SSO Authentication** - Not in current CLI scope
5. **Passwordless Authentication** - Future enhancement

## Success Criteria

### Definition of Done

1. `cargo build` succeeds with real SDK types
2. `cargo test` passes with real SDK integration
3. `cargo clippy` passes with no warnings
4. SDK client initializes correctly with default settings
5. SDK client initializes correctly with custom API URLs
6. Mock crypto functions replaced with SDK equivalents
7. Auth service uses SDK for login preparation
8. No mock SDK code remains in production paths

### Acceptance Tests

| Test ID | Given | When | Then |
|---------|-------|------|------|
| AT-1 | Default settings | Creating SDK client | Client initializes with production URLs |
| AT-2 | Custom API URLs | Creating SDK client | Client uses those URLs |
| AT-3 | Valid SDK client | Accessing auth client | Auth operations are available |
| AT-4 | Valid SDK client | Accessing crypto | Encryption/decryption works |
| AT-5 | Any error from SDK | Handling it | Error is converted to CLI error type |
| AT-6 | Valid credentials | Deriving master key | Key matches SDK test vectors |
| AT-7 | Valid master key and password | Hashing password | Hash matches SDK test vectors |

## Risk Assessment

### High Risk

1. **SDK API Changes** - SDK is marked internal and may change
   - Mitigation: Pin SDK version, create abstraction layer

2. **Token Management Conflicts** - SDK and CLI both manage tokens
   - Mitigation: Clear architecture decision on token ownership

### Medium Risk

1. **KDF Parameter Validation** - SDK has minimum requirements
   - Mitigation: Validate KDF params before passing to SDK

2. **Error Message Changes** - SDK errors may differ from current mock errors
   - Mitigation: Comprehensive error mapping

### Low Risk

1. **Performance Changes** - Real crypto may be slower
   - Mitigation: Expected behavior, document performance characteristics

## Phased Implementation Approach

### Phase 1: SDK Client Setup
- Replace mock `Client` with `bitwarden_core::Client`
- Update `create_sdk_client()` for proper initialization
- Update exports in `mod.rs`

### Phase 2: Remove Mock Crypto
- Remove `mock_crypto.rs` from module
- Update auth module exports

### Phase 3: Update Auth Service
- Replace mock crypto imports with SDK imports
- Implement KDF config conversion
- Update master key derivation
- Update password hashing
- Update user key decryption

### Phase 4: Update Container
- Update `sdk` field type
- Verify all SDK access patterns work

### Phase 5: Verification
- Run full test suite
- Run clippy
- Manual testing of auth commands

## Appendix: File Impact Analysis

| File | Changes Required | Complexity |
|------|-----------------|------------|
| `services/sdk.rs` | Replace mock Client, update create_sdk_client | Medium |
| `services/auth/mock_crypto.rs` | Remove entirely | Low |
| `services/auth/mod.rs` | Remove mock_crypto export | Low |
| `services/auth/auth_service.rs` | Replace all mock_crypto usage | High |
| `services/container.rs` | Update sdk field type | Low |
| `models/state/mod.rs` | Add From<KdfConfig> for Kdf | Medium |
