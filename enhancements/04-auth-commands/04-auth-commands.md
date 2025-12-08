---
slug: auth-commands
status: NEW
created: 2024-12-02
author: Migration Team
priority: high
---

# Enhancement: CLI Rust Migration - Authentication Commands

## Overview
**Goal:** Implement login, logout, lock, and unlock commands with full feature parity to the TypeScript CLI.

**User Story:**
As a CLI user, I want to log in, unlock my vault, and manage my session exactly as I do with the TypeScript CLI so that the migration is seamless.

## Context & Background
**Current State:**
- TypeScript CLI supports multiple login methods: password, API key, SSO
- Two-factor authentication support (authenticator, email, YubiKey)
- Master password verification and key derivation
- Session key generation and management
- Lock/unlock flow with password verification
- This is enhancement 4 of 8, depends on enhancements 1-3

**Technical Context:**
- Rust project at `bwcli-rs/`
- Must integrate with Bitwarden SDK for crypto operations
- Uses storage layer from enhancement 2
- Uses API client from enhancement 3
- Crypto operations already available in Rust SDK

**Dependencies:**
- Enhancement: project-bootstrap (must be complete)
- Enhancement: storage-layer (for storing tokens/keys)
- Enhancement: api-client (for authentication API calls)
- Bitwarden SDK for crypto operations

## Requirements

### Functional Requirements
1. Login command with password authentication
2. Login command with API key authentication
3. Login command with SSO flow (browser-based)
4. Two-factor authentication handling (TOTP, email, YubiKey)
5. Master password hashing and key derivation (PBKDF2, Argon2id)
6. User key decryption
7. Session key generation
8. Unlock command with master password verification
9. Lock command to clear decrypted keys
10. Logout command to clear all state
11. Support --passwordenv and --passwordfile flags
12. Support --check flag for validation without action

### Non-Functional Requirements
- **Performance:** Login flow <3s typical, key derivation based on iterations
- **Memory:** Secure memory handling, zeroize sensitive data
- **Reliability:** Handle network failures, clear error messages
- **Compatibility:** 100% compatible with TypeScript CLI auth flow

### Must Have (MVP)
- [ ] `bw login` with email/password
- [ ] `bw login --apikey` with client_id/client_secret
- [ ] Two-factor authentication prompts
- [ ] Master key derivation (PBKDF2, Argon2id)
- [ ] User key decryption
- [ ] Session key generation and output
- [ ] `bw unlock` with password verification
- [ ] `bw lock` to clear session
- [ ] `bw logout` to clear all state
- [ ] Support --passwordenv flag
- [ ] Support --passwordfile flag
- [ ] Support --check flag

### Should Have (if time permits)
- [ ] SSO login flow
- [ ] Hardware key support (YubiKey)
- [ ] Login session timeout
- [ ] Remember device functionality
- [ ] Login history tracking

### Won't Have (out of scope)
- GUI authentication flows (reason: CLI only)
- Biometric authentication (reason: not in CLI)
- Password manager for master password (reason: security concern)

## Open Questions

1. How should SSO callback be handled in CLI context?
2. Should we support device registration/remember me?
3. What's the timeout for interactive 2FA prompts?
4. How to handle expired sessions gracefully?
5. Should login automatically unlock or require separate unlock?
6. How to handle multiple accounts/profiles?

## Constraints & Limitations
**Technical Constraints:**
- Must use Bitwarden SDK for crypto operations
- Master password never stored, only hash for verification
- Session keys must be ephemeral
- Must support both PBKDF2 and Argon2id KDF
- Must clear sensitive memory after use

**Business/Timeline Constraints:**
- Blocking enhancement 5 (vault operations need authentication)
- Critical path item
- Must maintain security best practices

## Success Criteria
**Definition of Done:**
- [ ] `bw login` with email/password works
- [ ] `bw login --apikey` works
- [ ] Two-factor authentication prompts work
- [ ] `bw unlock` decrypts vault successfully
- [ ] `bw lock` clears session
- [ ] `bw logout` clears all stored state
- [ ] `bw login --check` validates credentials without logging in
- [ ] `bw unlock --check` validates password without unlocking
- [ ] Session key output matches TypeScript format
- [ ] All tests pass
- [ ] Documentation complete

**Acceptance Tests:**
1. Given valid email/password, when running `bw login`, then authentication succeeds and session key returned
2. Given valid API key, when running `bw login --apikey`, then authentication succeeds
3. Given 2FA enabled, when logging in, then prompts for 2FA code and validates
4. Given locked vault, when running `bw unlock` with correct password, then vault unlocks
5. Given incorrect password, when unlocking, then clear error message shown
6. Given logged in session, when running `bw lock`, then session cleared
7. Given logged in session, when running `bw logout`, then all state cleared
8. Given --passwordenv flag, when logging in, then reads password from environment variable
9. Given --passwordfile flag, when logging in, then reads password from file
10. Given --check flag, when running login, then validates without persisting state

## Security & Safety Considerations
- Never store master password in any form
- Zeroize all sensitive buffers after use
- Use constant-time comparisons for secrets
- Validate all user inputs
- Clear session keys on logout/lock
- Don't log sensitive data
- Use secure random for session key generation
- Validate KDF parameters before use
- Rate limit failed authentication attempts where possible

## UI/UX Considerations (if applicable)
- Clear progress indication during key derivation (can be slow)
- Helpful error messages for common issues
- Interactive prompts for passwords (hidden input)
- Show which 2FA methods are available
- Confirm before logout
- Clear success/failure messages

## Testing Strategy
**Unit Tests:**
- Test password hashing with known vectors
- Test key derivation with test KDF parameters
- Test session key generation
- Test token storage/retrieval
- Test 2FA code validation
- Test error handling for invalid inputs

**Integration Tests:**
- Test full login flow with test account
- Test 2FA flow with test account
- Test unlock/lock/logout flows
- Test API key authentication
- Test password file reading
- Test environment variable reading

**Manual Test Scenarios:**
1. Login with valid credentials
2. Login with invalid credentials
3. Login with 2FA enabled
4. Unlock after lock
5. Logout and verify state cleared
6. Test with self-hosted server
7. Compare session key format with TypeScript CLI

## References & Research
- apps/cli/src/auth/commands/login.command.ts
- apps/cli/src/auth/commands/logout.command.ts
- apps/cli/src/auth/commands/lock.command.ts
- apps/cli/src/key-management/commands/unlock.command.ts
- Bitwarden SDK documentation for crypto operations
- KDF specifications (PBKDF2, Argon2id)

## Notes for PM Subagent
- Verify all login methods must be supported in MVP
- Confirm 2FA provider priorities
- Flag if SSO flow is unclear for CLI context
- Ensure error messages are user-friendly

## Notes for Architect Subagent
- Design auth state machine clearly
- Separate authentication from key derivation concerns
- Use SDK for all crypto operations
- Plan for multiple authentication methods
- Design session management for future features
- Consider command trait pattern for auth commands
- Plan error types for authentication failures

## Notes for Implementer Subagent
- Use Bitwarden SDK for all crypto operations
- Integrate with dialoguer for interactive prompts
- Use secrecy crate for sensitive data
- Implement zeroization for all sensitive buffers
- Use constant-time comparisons
- Store tokens using storage layer from enhancement 2
- Use API client from enhancement 3
- Follow TypeScript CLI behavior exactly
- Add comprehensive error context

## Notes for Testing Subagent
- Test all authentication flows thoroughly
- Test with various KDF configurations
- Test 2FA with different providers
- Verify password never stored
- Verify session keys match TypeScript format
- Test error scenarios exhaustively
- Test with self-hosted servers
- Verify memory is cleared after operations
- Test concurrent session handling