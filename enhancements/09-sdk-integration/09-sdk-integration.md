---
slug: sdk-integration
status: NEW
created: 2024-12-06
author: Migration Team
priority: critical
---

# Enhancement: CLI Rust Migration - SDK Integration

## Overview
**Goal:** Replace mock SDK implementations with the real Bitwarden SDK from `../sdk-internal/` to enable actual cryptographic operations, authentication flows, and vault management.

**User Story:**
As a CLI user, I want the CLI to use the real Bitwarden SDK so that all cryptographic operations, authentication, and vault management work correctly with my Bitwarden account.

## Context & Background
**Current State:**
- Project has mock implementations for SDK client (`sdk.rs`) and crypto operations (`mock_crypto.rs`)
- SDK dependencies have been added to `Cargo.toml` pointing to `../sdk-internal/crates/...`
- Build compiles successfully with SDK dependencies
- Auth service and other components reference mock types instead of real SDK types
- This is enhancement 9 of the CLI migration series

**Technical Context:**
- SDK location: `../sdk-internal/` (Bitwarden internal SDK)
- SDK requires Rust 1.85+ and Edition 2024 (already configured)
- SDK provides: `bitwarden-core`, `bitwarden-crypto`, `bitwarden-auth`, `bitwarden-vault`, `bitwarden-generators`, `bitwarden-encoding`, `bitwarden-error`
- The SDK Client handles all crypto operations internally
- Must maintain existing API surface for other CLI components

**Dependencies:**
- Enhancement: project-bootstrap (complete)
- Enhancement: storage-layer (complete)
- Enhancement: api-client (complete)
- Enhancement: auth-commands (complete - but using mocks)
- SDK repository at `../sdk-internal/`

## Requirements

### Functional Requirements
1. Replace mock `Client` in `sdk.rs` with real `bitwarden_core::Client`
2. Replace mock crypto operations in `mock_crypto.rs` with SDK crypto
3. Configure SDK client with proper settings (API URLs, device type, user agent)
4. Expose necessary SDK types for auth, vault, and crypto operations
5. Update auth service to use SDK's AuthClient for real authentication
6. Ensure session management works with SDK token handling

### Non-Functional Requirements
- **Performance:** SDK operations should not add significant overhead
- **Memory:** Use SDK's secure memory handling (zeroize)
- **Reliability:** Handle SDK errors gracefully with clear messages
- **Compatibility:** Maintain existing CLI command interfaces

### Must Have (MVP)
- [ ] Replace mock `Client` struct with `bitwarden_core::Client`
- [ ] Configure `ClientSettings` with proper defaults (API URLs, device type)
- [ ] Replace mock crypto functions with SDK crypto operations
- [ ] Update `ServiceContainer` to use real SDK client
- [ ] Update auth service to use SDK's authentication methods
- [ ] Verify project compiles with all real SDK types
- [ ] Basic smoke test of SDK client initialization

### Should Have (if time permits)
- [ ] Add SDK-based TOTP generation
- [ ] Add SDK-based password generation
- [ ] Add SDK-based Send encryption/decryption
- [ ] Comprehensive error type mapping from SDK errors

### Won't Have (out of scope)
- Full vault sync implementation (separate enhancement)
- Cipher CRUD operations (separate enhancement)
- Organization key handling (separate enhancement)

## Open Questions

1. What `DeviceType` should we use for the CLI? (Likely `LinuxCLI` or similar)
2. How should SDK errors be mapped to CLI-specific error types?
3. Should we expose raw SDK types or wrap them in CLI-specific types?
4. How to handle SDK version pinning for stability?

## Constraints & Limitations
**Technical Constraints:**
- Must use SDK from `../sdk-internal/` path dependency
- SDK's `Client` initialization may require specific settings
- Some SDK features require the "internal" feature flag on `bitwarden-core`
- Must maintain backwards compatibility with existing CLI command structure

**Business/Timeline Constraints:**
- Blocking actual authentication testing with real Bitwarden accounts
- Critical path for vault operations
- Must not break existing CLI command interfaces

## Success Criteria
**Definition of Done:**
- [ ] `cargo build` succeeds with real SDK types
- [ ] `cargo test` passes with real SDK integration
- [ ] `cargo clippy` passes with no warnings
- [ ] SDK client initializes correctly with default settings
- [ ] SDK client initializes correctly with custom API URLs
- [ ] Mock crypto functions replaced with SDK equivalents
- [ ] Auth service uses SDK for login preparation
- [ ] No mock SDK code remains in production paths

**Acceptance Tests:**
1. Given default settings, when creating SDK client, then client initializes with production URLs
2. Given custom API URLs, when creating SDK client, then client uses those URLs
3. Given valid SDK client, when accessing auth client, then auth operations are available
4. Given valid SDK client, when accessing crypto, then encryption/decryption works
5. Given any error from SDK, when handling it, then error is converted to CLI error type

## Security & Safety Considerations
- SDK handles all cryptographic operations - do not bypass
- Use SDK's secure memory handling (automatic with SDK types)
- Never log SDK internal state or crypto material
- Ensure proper error handling doesn't leak sensitive data
- SDK already implements constant-time comparisons and secure practices

## UI/UX Considerations (if applicable)
- Error messages from SDK should be user-friendly
- SDK operation failures should provide actionable guidance
- No visible changes to CLI interface - internal refactoring only

## Testing Strategy
**Unit Tests:**
- Test SDK client creation with various settings
- Test SDK type conversions and wrappers
- Test error mapping from SDK to CLI errors

**Integration Tests:**
- Test SDK client initialization
- Test SDK crypto operations with known test vectors
- Test SDK auth client accessibility

**Manual Test Scenarios:**
1. Run `bw status` to verify SDK client initializes
2. Run `bw config server` to verify custom URL handling
3. Attempt login to verify SDK auth client works (with test account)

## References & Research
- SDK Internal Repository: `../sdk-internal/`
- `bitwarden-core/src/client/client.rs` - Client implementation
- `bitwarden-core/src/client/client_settings.rs` - ClientSettings, DeviceType
- `bitwarden-core/src/auth/auth_client.rs` - AuthClient
- `bitwarden-crypto/src/` - Crypto operations
- Enhancement 01-project-bootstrap: SDK integration section

## Notes for PM Subagent
- Verify SDK types are stable enough for integration
- Confirm minimum viable SDK operations for auth flow
- Flag if SDK API changes are expected soon
- Ensure error handling requirements are clear

## Notes for Architect Subagent
- Evaluate whether to re-export SDK types directly or wrap them
- Design error type mapping from SDK to CLI errors
- Consider future SDK version update strategy
- Plan for SDK feature flag requirements
- Ensure ServiceContainer changes are backwards compatible

## Notes for Implementer Subagent
- Start with `sdk.rs` - replace mock Client with real SDK Client
- Use `bitwarden_core::Client::new(Some(settings))` for initialization
- Remove `mock_crypto.rs` entirely, use `bitwarden_crypto` directly
- Update imports in `container.rs`, `auth_service.rs`
- Add `features = ["internal"]` to `bitwarden-core` dependency
- Test compilation frequently during changes
- Check SDK source for correct type paths and APIs

## Notes for Testing Subagent
- Focus on SDK client initialization testing
- Verify crypto operations work with known test vectors
- Test error handling paths
- Verify no mock code remains in production paths
- Test with both default and custom URL configurations
- Consider testing with a real test Bitwarden account

## Implementation Phases

### Phase 1: SDK Client Setup
Files: `crates/bw-core/src/services/sdk.rs`
- Remove mock Client struct
- Import and re-export `bitwarden_core::{Client, ClientSettings, DeviceType}`
- Update `create_sdk_client()` to use real SDK types

### Phase 2: Remove Mock Crypto
Files: `crates/bw-core/src/services/mock_crypto.rs`, `mod.rs`
- Remove `mock_crypto.rs` file entirely
- Update `mod.rs` to remove mock_crypto module export
- Update any imports that used mock crypto

### Phase 3: Update Container
Files: `crates/bw-core/src/services/container.rs`
- Update `sdk` field type if needed
- Ensure `sdk()` method returns correct type
- Verify all SDK access patterns work

### Phase 4: Update Auth Service
Files: `crates/bw-core/src/services/auth_service.rs`
- Use SDK's auth client for login preparation
- Update key derivation to use SDK crypto
- Ensure token handling works with SDK

### Phase 5: Verification
- Run `cargo build`
- Run `cargo test`
- Run `cargo clippy`
- Manual testing of basic commands
