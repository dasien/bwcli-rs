---
enhancement: 04-auth-commands
agent: requirements-analyst
task_id: task_1764797757_73347
timestamp: 2025-12-03T20:45:00Z
status: READY_FOR_ARCHITECTURE
---

# Authentication Commands Requirements Analysis

## Executive Summary

This document provides comprehensive requirements analysis for implementing authentication commands (`login`, `logout`, `lock`, `unlock`) in the Rust Bitwarden CLI. This enhancement is **critical path** - it blocks all vault operations (enhancements 5-8) and is the foundational authentication layer for the entire CLI.

**Complexity Assessment**: **HIGH**
- Complex cryptographic operations (master key derivation, encryption)
- Multiple authentication flows (password, API key, SSO, 2FA)
- Session state management
- Security-critical implementation
- Integration with Bitwarden SDK crypto primitives

**Dependencies**:
- Enhancement 1 (project-bootstrap) - ✅ COMPLETE
- Enhancement 2 (storage-layer) - ✅ COMPLETE
- Enhancement 3 (api-client) - ✅ COMPLETE
- Bitwarden SDK - REQUIRED (crypto operations)

**Estimated Effort**: 8-12 days (implementer + tester)

---

## Problem Statement

### What Needs to Be Built

Users need to authenticate with Bitwarden servers and manage their session state through four core commands:

1. **`bw login`** - Authenticate with email/password, API key, or SSO
2. **`bw logout`** - Clear all authentication state and session data
3. **`bw unlock`** - Decrypt vault keys using master password
4. **`bw lock`** - Clear decrypted keys while preserving login session

### User Perspective

**Current State (TypeScript CLI)**:
- Users can log in with email/password, API key, or SSO
- Two-factor authentication (TOTP, email, YubiKey) is supported
- Session keys are generated and can be exported as `BW_SESSION` environment variable
- Users can lock/unlock vault without re-authenticating
- Password can be provided via `--passwordenv` or `--passwordfile` flags
- `--check` flag validates credentials without persisting state

**Target State (Rust CLI)**:
- **100% feature parity** with TypeScript CLI
- Identical authentication flows and user experience
- Compatible session key format for environment variable usage
- Same command-line flags and options
- Equivalent error messages and user feedback

### Business Value

- **Migration enablement**: Required for users to switch from TypeScript to Rust CLI
- **Security foundation**: Establishes secure authentication patterns for all future features
- **Critical path**: Blocks all vault operations - highest priority after foundational layers

---

## Functional Requirements

### FR-1: Password-Based Login

**Description**: Users can authenticate using email address and master password.

**What is needed**:
- Interactive prompt for email and password
- Master password hashing (PBKDF2 or Argon2id based on account KDF settings)
- Master key derivation from hashed password
- User key decryption using master key
- Session token generation and storage
- Support for `--passwordenv` and `--passwordfile` flags

**Acceptance Criteria**:
- [ ] Email and password are prompted interactively if not provided
- [ ] Master password is never stored, only used for key derivation
- [ ] KDF parameters (iterations, memory, parallelism) are fetched from server
- [ ] Master key is derived correctly using account's KDF algorithm
- [ ] User key is decrypted successfully
- [ ] Access token and refresh token are stored encrypted
- [ ] Session key is generated and output for `BW_SESSION` usage
- [ ] `--passwordenv <VAR>` reads password from environment variable
- [ ] `--passwordfile <PATH>` reads password from file
- [ ] `--check` validates credentials without persisting tokens

**Data Requirements**:
- Input: Email, password, optional 2FA code
- Output: Session key (64-byte base64), access token, refresh token
- Storage: Encrypted tokens, user profile, KDF config

### FR-2: API Key Authentication

**Description**: Users can authenticate using client_id and client_secret (for automation/CI).

**What is needed**:
- API key login endpoint integration
- Client credentials OAuth2 flow
- Token exchange and storage
- No master password required for API key flow

**Acceptance Criteria**:
- [ ] `bw login --apikey` prompts for client_id and client_secret
- [ ] API key authentication endpoint is called correctly
- [ ] Access token and refresh token are received and stored
- [ ] No master key derivation occurs (API key flow is keyless)
- [ ] Session key is still generated for consistency
- [ ] Vault operations work with API key authentication

**Data Requirements**:
- Input: client_id, client_secret
- Output: Access token, refresh token, session key
- Storage: Encrypted tokens, minimal user profile

### FR-3: SSO Authentication (Should Have)

**Description**: Users can authenticate via Single Sign-On (browser-based flow).

**What is needed**:
- SSO initiation endpoint
- Browser launch with callback URL
- Local HTTP server to receive callback
- Authorization code exchange for tokens
- Master password prompt after SSO for key derivation

**Acceptance Criteria**:
- [ ] `bw login --sso` starts SSO flow
- [ ] Browser opens with SSO provider
- [ ] Local server receives callback with authorization code
- [ ] Code is exchanged for access token
- [ ] User is prompted for master password to derive keys
- [ ] Session is established with both SSO token and vault keys

**Open Questions**:
- Q1: How should the local callback server port be determined? (Random? Configurable?)
- Q2: What happens if browser cannot be launched? (Manual URL copy/paste?)
- Q3: Should SSO session timeout independently from vault lock?

**Risk**: SSO flow is complex in CLI context - consider deferring to post-MVP

### FR-4: Two-Factor Authentication

**Description**: Support multiple 2FA methods when account has 2FA enabled.

**What is needed**:
- 2FA challenge detection during login
- Support for TOTP (authenticator app codes)
- Support for email codes
- Support for YubiKey hardware tokens
- Remember device functionality (optional)

**Acceptance Criteria**:
- [ ] Login detects 2FA requirement from initial auth response
- [ ] User is prompted to select 2FA method if multiple are available
- [ ] TOTP code entry and validation works
- [ ] Email code request and validation works
- [ ] YubiKey token validation works
- [ ] Invalid 2FA code shows clear error and allows retry
- [ ] `--method <METHOD>` flag pre-selects 2FA method

**Open Questions**:
- Q4: What's the timeout for interactive 2FA prompts? (Infinite? 5 minutes?)
- Q5: Should we support "remember device" in CLI? (Security vs convenience trade-off)
- Q6: How many 2FA retry attempts before lockout?

**Data Requirements**:
- Input: 2FA code (6 digits for TOTP, variable for email/YubiKey)
- Output: 2FA token for auth request
- Storage: Optional device identifier if "remember me" is implemented

### FR-5: Master Key Derivation

**Description**: Derive cryptographic keys from master password using account's KDF settings.

**What is needed**:
- Support for PBKDF2-SHA256 (legacy accounts)
- Support for Argon2id (newer accounts)
- Correct parameter application (iterations, memory, parallelism)
- User key decryption using derived master key
- Session key generation from user key

**Acceptance Criteria**:
- [ ] KDF type and parameters are fetched from account prelogin endpoint
- [ ] PBKDF2 derivation uses correct iteration count (default: 600,000)
- [ ] Argon2id derivation uses correct memory (default: 64 MB) and parallelism (default: 4)
- [ ] Master key length is 256 bits (32 bytes)
- [ ] Encrypted user key is decrypted successfully
- [ ] Decryption failure shows clear error (wrong password)
- [ ] Key derivation progress is shown to user (can take several seconds)

**Security Requirements**:
- Master password MUST NEVER be stored
- Master key MUST be zeroized after use
- User key MUST be stored encrypted
- Constant-time comparison for password verification
- KDF parameters MUST be validated before use

**Performance Requirements**:
- PBKDF2 with 600k iterations: typically 2-3 seconds
- Argon2id with 64 MB memory: typically 1-2 seconds
- Show progress indicator for operations >1 second

### FR-6: Unlock Command

**Description**: Decrypt vault keys using master password without re-authenticating.

**What is needed**:
- Check that user is logged in (has tokens)
- Prompt for master password
- Re-derive master key using stored KDF parameters
- Decrypt user key
- Update session state to "unlocked"

**Acceptance Criteria**:
- [ ] `bw unlock` checks for existing login session
- [ ] Error if not logged in: "You are not logged in"
- [ ] Master password is prompted securely
- [ ] KDF parameters are loaded from storage
- [ ] Master key derivation matches login flow
- [ ] User key is decrypted and stored (encrypted with BW_SESSION)
- [ ] Session key is output for environment variable usage
- [ ] `--check` validates password without persisting state
- [ ] `--passwordenv` and `--passwordfile` flags work

**Data Requirements**:
- Input: Master password
- Output: Session key (for BW_SESSION)
- Storage: Encrypted user key, KDF config (already stored)

### FR-7: Lock Command

**Description**: Clear decrypted vault keys while maintaining login session.

**What is needed**:
- Remove decrypted user key from memory and storage
- Preserve access token and refresh token
- Clear BW_SESSION if set
- Zeroize sensitive memory

**Acceptance Criteria**:
- [ ] `bw lock` checks for existing session
- [ ] Encrypted user key is removed from storage
- [ ] In-memory keys are zeroized
- [ ] Access token and refresh token remain valid
- [ ] User can unlock again without re-authenticating
- [ ] Success message: "Your vault is locked"

**Data Requirements**:
- Input: None (uses current session)
- Output: Success confirmation
- Storage: Remove `__PROTECTED__userKey`, keep tokens

### FR-8: Logout Command

**Description**: Clear all authentication state and remove all stored data.

**What is needed**:
- Remove access token and refresh token
- Remove encrypted user key
- Remove user profile
- Remove KDF configuration
- Clear BW_SESSION
- Zeroize all sensitive memory

**Acceptance Criteria**:
- [ ] `bw logout` clears all tokens from storage
- [ ] All user data is removed (profile, keys, config)
- [ ] Confirmation prompt before logout (can be skipped with `--force`)
- [ ] Success message: "You have been logged out"
- [ ] Subsequent commands requiring auth show "not logged in" error

**Data Requirements**:
- Input: None (optional `--force` flag)
- Output: Success confirmation
- Storage: Remove all auth-related keys

### FR-9: Session Key Management

**Description**: Generate and manage BW_SESSION environment variable for vault access.

**What is needed**:
- Generate 64-byte session key (32 bytes encryption + 32 bytes MAC)
- Base64-encode session key
- Output session key for environment variable usage
- Use session key for encrypting sensitive storage values

**Acceptance Criteria**:
- [ ] Session key is 64 bytes (512 bits) of cryptographically secure random data
- [ ] Session key is base64-encoded for shell compatibility
- [ ] Session key is output after login/unlock: `export BW_SESSION="..."`
- [ ] Session key format matches TypeScript CLI exactly
- [ ] Storage layer uses BW_SESSION for encrypting `__PROTECTED__` values
- [ ] Commands work with BW_SESSION set in environment

**Security Requirements**:
- Use cryptographically secure random number generator
- Session key should be unique per login/unlock
- Never log or display session key in debug output
- Zeroize session key from memory when done

### FR-10: Password Input Options

**Description**: Support multiple methods for providing master password.

**What is needed**:
- Interactive prompt (default, hidden input)
- `--passwordenv <VAR>` to read from environment variable
- `--passwordfile <PATH>` to read from file

**Acceptance Criteria**:
- [ ] Interactive prompt hides password as typed
- [ ] `--passwordenv` reads from specified environment variable
- [ ] `--passwordfile` reads from specified file path
- [ ] File path can be absolute or relative
- [ ] Error if environment variable not set
- [ ] Error if file not found or not readable
- [ ] Password is trimmed (leading/trailing whitespace removed)
- [ ] Empty password shows clear error

**Security Requirements**:
- Interactive prompt must hide input (no echo)
- Password file should have restricted permissions (warning if world-readable)
- Password is never logged or displayed

### FR-11: Credential Validation (--check flag)

**Description**: Validate credentials without persisting authentication state.

**What is needed**:
- `bw login --check` validates email/password without storing tokens
- `bw unlock --check` validates master password without updating session

**Acceptance Criteria**:
- [ ] `bw login --check` performs full authentication flow
- [ ] No tokens are stored to storage layer
- [ ] Success message: "Valid credentials"
- [ ] Error message if credentials invalid
- [ ] `bw unlock --check` derives master key and validates
- [ ] No session state is updated
- [ ] Exit code 0 for valid, non-zero for invalid

**Use Cases**:
- Automated scripts checking vault availability
- Pre-flight validation before batch operations
- Testing credentials without side effects

---

## Non-Functional Requirements

### NFR-1: Performance

**Requirements**:
- Login flow (email/password): <5 seconds typical (network dependent)
- Master key derivation: Based on KDF iterations (2-3s for PBKDF2, 1-2s for Argon2id)
- Unlock operation: <4 seconds typical
- Lock operation: <100ms (memory operations only)
- Logout operation: <200ms (storage cleanup)

**Rationale**: Key derivation is intentionally slow (security feature), but other operations should be responsive.

### NFR-2: Security

**Requirements**:
- Master password NEVER stored in any form
- All sensitive data zeroized after use
- Constant-time comparisons for password verification
- Secure random for session key generation
- No sensitive data in logs or error messages
- File permissions: 0600 for storage, 0700 for directory (Unix)

**Rationale**: Authentication is security-critical - follow industry best practices.

### NFR-3: Compatibility

**Requirements**:
- 100% compatible with TypeScript CLI authentication flow
- Session key format identical (interchangeable)
- Storage format compatible (can migrate TypeScript -> Rust)
- API requests identical (same endpoints, same payloads)
- Error messages similar (user familiarity)

**Rationale**: Users must be able to migrate seamlessly without learning new workflows.

### NFR-4: Reliability

**Requirements**:
- Handle network failures gracefully (clear error messages)
- Retry token refresh automatically (via API client)
- Detect and handle expired sessions
- Validate all API responses before processing
- Atomic storage updates (no partial state)

**Rationale**: Authentication must be rock-solid - users cannot lose access to their vaults.

### NFR-5: Usability

**Requirements**:
- Clear progress indication during slow operations (key derivation)
- Helpful error messages with actionable suggestions
- Interactive prompts for all required inputs
- Confirmation before destructive operations (logout)
- Success messages after each operation

**Rationale**: CLI users need feedback to understand what's happening and how to fix issues.

---

## User Stories

### US-1: Basic Login Flow

**As a** Bitwarden user
**I want to** log in with my email and master password
**So that** I can access my vault from the Rust CLI

**Acceptance Criteria**:
- [ ] Running `bw login` prompts for email and password
- [ ] Password input is hidden
- [ ] Login succeeds with valid credentials
- [ ] Session key is displayed for `export BW_SESSION="..."`
- [ ] Subsequent vault commands work without re-authentication
- [ ] Invalid password shows clear error: "Invalid master password"

**Edge Cases**:
- Network failure during login → "Cannot connect to server. Check your internet connection."
- Account not found → "Account not found. Check your email address."
- Server error (500) → "Server error. Please try again later."

**Complexity**: Medium

### US-2: API Key Login for Automation

**As a** DevOps engineer
**I want to** authenticate using API keys
**So that** I can automate vault access in CI/CD pipelines

**Acceptance Criteria**:
- [ ] Running `bw login --apikey` prompts for client_id and client_secret
- [ ] API key authentication succeeds
- [ ] No master password is required
- [ ] Session is established for vault operations
- [ ] Invalid API key shows error: "Invalid API key credentials"

**Complexity**: Low-Medium

### US-3: Two-Factor Authentication

**As a** security-conscious user
**I want to** use two-factor authentication
**So that** my vault is protected even if my password is compromised

**Acceptance Criteria**:
- [ ] Login detects 2FA requirement
- [ ] I'm prompted to select 2FA method (authenticator, email, YubiKey)
- [ ] I can enter TOTP code from authenticator app
- [ ] Invalid code shows error with option to retry
- [ ] After 3 failed attempts, I'm locked out temporarily
- [ ] Successful 2FA completes login flow

**Complexity**: High

### US-4: Unlock After Lock

**As a** CLI user
**I want to** unlock my vault after locking it
**So that** I don't need to re-authenticate every time

**Acceptance Criteria**:
- [ ] Running `bw unlock` prompts for master password
- [ ] Password is validated against stored KDF parameters
- [ ] Unlock succeeds and displays session key
- [ ] Vault commands work after unlock
- [ ] I don't need to enter email or deal with 2FA again

**Complexity**: Medium

### US-5: Secure Logout

**As a** shared workstation user
**I want to** log out and clear all vault data
**So that** others cannot access my vault

**Acceptance Criteria**:
- [ ] Running `bw logout` prompts for confirmation
- [ ] I can skip confirmation with `--force` flag
- [ ] All tokens and keys are cleared from storage
- [ ] Success message confirms logout
- [ ] Subsequent commands show "not logged in" error

**Complexity**: Low

### US-6: Password from Environment Variable

**As a** scripter
**I want to** provide master password via environment variable
**So that** I can automate unlock operations securely

**Acceptance Criteria**:
- [ ] `bw login --passwordenv MY_PASSWORD_VAR` reads password from environment
- [ ] `bw unlock --passwordenv MY_PASSWORD_VAR` reads password from environment
- [ ] Error if environment variable doesn't exist
- [ ] Password is used for authentication without prompting
- [ ] Authentication succeeds/fails based on password validity

**Complexity**: Low

### US-7: Password from File

**As a** system administrator
**I want to** store master password in a file with restricted permissions
**So that** I can automate vault access while maintaining security

**Acceptance Criteria**:
- [ ] `bw login --passwordfile ~/.vault-password` reads password from file
- [ ] File path supports both absolute and relative paths
- [ ] Warning if file has overly permissive permissions (e.g., world-readable)
- [ ] Error if file doesn't exist or isn't readable
- [ ] Password is trimmed (leading/trailing whitespace removed)

**Complexity**: Low-Medium

### US-8: Credential Validation Without State Change

**As a** script author
**I want to** validate credentials without persisting session
**So that** I can check vault availability before running operations

**Acceptance Criteria**:
- [ ] `bw login --check` validates credentials
- [ ] No tokens are stored
- [ ] Exit code 0 if valid, non-zero if invalid
- [ ] Suitable for use in shell scripts (`if bw login --check; then ...`)
- [ ] Same behavior for `bw unlock --check`

**Complexity**: Low

---

## Integration Points

### IP-1: Storage Layer (Enhancement 2)

**Interface**: `Storage` trait from `services::storage`

**Required Operations**:
- `set_secure("accessToken", token)` - Store access token encrypted
- `get_secure("accessToken")` - Retrieve access token
- `set_secure("refreshToken", token)` - Store refresh token encrypted
- `get_secure("refreshToken")` - Retrieve refresh token
- `set_secure("userKey", encrypted_key)` - Store encrypted user key
- `get_secure("userKey")` - Retrieve encrypted user key
- `set("userProfile", profile)` - Store user profile (plain)
- `get::<UserProfile>("userProfile")` - Retrieve user profile
- `set("kdfConfig", config)` - Store KDF parameters
- `get::<KdfConfig>("kdfConfig")` - Retrieve KDF parameters
- `remove_secure("accessToken")` - Remove on logout
- All remove operations for logout

**Data Models Available**:
- `models::state::AuthState` - Authentication state structure
- `models::state::UserProfile` - User profile structure
- `models::state::KdfConfig` - KDF configuration structure

**Considerations**:
- Storage uses `BW_SESSION` environment variable for encryption key
- After login/unlock, set `BW_SESSION` so storage can encrypt
- Storage layer handles all encryption/decryption automatically

### IP-2: API Client (Enhancement 3)

**Interface**: `ApiClient` trait from `services::api`

**Required API Calls**:

```rust
// Pre-login to get KDF parameters
POST /identity/accounts/prelogin
Body: { "email": "user@example.com" }
Response: { "kdf": 0|1, "kdfIterations": 600000, "kdfMemory": 64, "kdfParallelism": 4 }

// Login with password
POST /identity/connect/token
Body (form-encoded): {
  "grant_type": "password",
  "username": "user@example.com",
  "password": "base64_hashed_password",
  "scope": "api offline_access",
  "client_id": "cli",
  "deviceType": 8,
  "deviceName": "rust-cli",
  "deviceIdentifier": "uuid"
}
Response: { "access_token": "...", "refresh_token": "...", "Key": "encrypted_user_key", ... }

// Login with API key
POST /identity/connect/token
Body (form-encoded): {
  "grant_type": "client_credentials",
  "client_id": "user.client_id",
  "client_secret": "client_secret",
  "scope": "api",
  "deviceType": 8,
  "deviceName": "rust-cli",
  "deviceIdentifier": "uuid"
}

// 2FA challenge (if required)
POST /identity/connect/token
Body (form-encoded): { ..., "twoFactorToken": "code", "twoFactorProvider": 0|1|3, ... }

// Get user profile (after login)
GET /api/accounts/profile
Headers: { "Authorization": "Bearer <access_token>" }
Response: { "id": "...", "email": "...", "name": "...", ... }
```

**Available Methods**:
- `post<T, R>("/identity/connect/token", body)` - Token endpoint
- `get_with_auth<T>("/api/accounts/profile")` - User profile
- Token refresh handled automatically by API client

**Considerations**:
- API client handles token refresh automatically on 401
- Use `post()` for `/identity/*` (no auth header)
- Use `get_with_auth()` / `post_with_auth()` for `/api/*` (auth header required)

### IP-3: Bitwarden SDK (Crypto Operations)

**Interface**: Bitwarden SDK crates (to be integrated)

**Required Crypto Operations**:

```rust
// Master key derivation (PBKDF2)
fn derive_master_key_pbkdf2(
    password: &str,
    email: &str, // Used as salt
    iterations: u32
) -> [u8; 32] // 256-bit key

// Master key derivation (Argon2id)
fn derive_master_key_argon2(
    password: &str,
    email: &str, // Used as salt
    memory_mb: u32,
    iterations: u32,
    parallelism: u32
) -> [u8; 32] // 256-bit key

// User key decryption
fn decrypt_user_key(
    encrypted_key: &str, // EncString format
    master_key: &[u8; 32]
) -> Result<[u8; 64]> // Decrypted symmetric key

// Session key generation
fn generate_session_key() -> [u8; 64] // Random 512-bit key

// Password hashing for auth request
fn hash_password(
    password: &str,
    master_key: &[u8; 32]
) -> String // Base64-encoded hashed password
```

**SDK Integration Status**:
- SDK dependency commented out in `Cargo.toml` (lines 30-33)
- Need to uncomment and integrate during implementation
- SDK provides all crypto primitives - DO NOT implement crypto manually

**Security Critical**:
- NEVER implement crypto operations manually
- Always use SDK-provided functions
- Validate all crypto inputs and outputs
- Zeroize sensitive buffers after use

### IP-4: Service Container

**Interface**: `ServiceContainer` from `services::container`

**Integration**:
```rust
pub struct ServiceContainer {
    storage: Arc<dyn Storage>,
    api_client: Arc<dyn ApiClient>,
    // SDK client to be added
}

// Commands will receive container
impl LoginCommand {
    pub async fn execute(&self, container: &ServiceContainer) -> Result<()> {
        let storage = container.storage();
        let api = container.api_client();
        // ... use storage and api
    }
}
```

**Considerations**:
- All commands receive `ServiceContainer` reference
- Container provides access to storage, API client, and SDK
- Container manages service lifetimes and dependencies

---

## Open Questions and Clarifications

### Critical Questions (Block Architecture)

**Q1: SSO Callback Server Port**
- **Question**: How should the local HTTP server determine which port to use for SSO callback?
- **Options**:
  - A) Random available port (requires URL parameter to server)
  - B) Fixed port (8087? might conflict)
  - C) User-configurable via flag or config
- **Impact**: Affects SSO flow implementation complexity
- **Recommendation**: Start with Option B (fixed port 8087), add configurability if needed
- **Decision Needed From**: Architect

**Q2: SSO Browser Launch Failure**
- **Question**: What should happen if CLI cannot launch browser automatically?
- **Options**:
  - A) Fall back to "Copy this URL and paste in browser" flow
  - B) Error and abort
  - C) Prompt user: "Open browser manually? [y/N]"
- **Impact**: Affects SSO user experience in headless/SSH environments
- **Recommendation**: Option A (fallback to manual URL)
- **Decision Needed From**: Architect

**Q3: Login vs Unlock Behavior**
- **Question**: Should `bw login` automatically unlock the vault, or should unlock be separate?
- **Current TypeScript Behavior**: Login also unlocks (user key is decrypted immediately)
- **Options**:
  - A) Login unlocks automatically (matches TypeScript)
  - B) Login only authenticates, unlock is always separate
- **Impact**: User workflow and session state management
- **Recommendation**: Option A (match TypeScript behavior)
- **Decision Needed From**: Product (but strong recommendation to match existing CLI)

### Important Questions (Inform Architecture)

**Q4: 2FA Prompt Timeout**
- **Question**: How long should CLI wait for 2FA code entry before timing out?
- **Current Behavior**: Unknown (need to test TypeScript CLI)
- **Options**: No timeout, 5 minutes, 10 minutes?
- **Recommendation**: No timeout (user controls with Ctrl+C)

**Q5: Remember Device for 2FA**
- **Question**: Should CLI support "remember this device" to skip 2FA for 30 days?
- **Security Trade-off**: Convenience vs security on shared machines
- **Recommendation**: Support it but default to OFF, require `--remember` flag
- **Decision Needed From**: Product/Security team

**Q6: 2FA Retry Attempts**
- **Question**: How many invalid 2FA code attempts before account lockout?
- **Server-Side**: Likely enforced by server (need to verify)
- **Client-Side**: Should CLI limit retries locally?
- **Recommendation**: Let server handle lockout, CLI allows unlimited retries

**Q7: Multiple Accounts/Profiles**
- **Question**: Should CLI support multiple logged-in accounts simultaneously?
- **Current State**: TypeScript CLI stores single account
- **Complexity**: High (would need profile switching, separate storage)
- **Recommendation**: Out of scope for MVP, single account only

**Q8: Session Timeout Behavior**
- **Question**: How should CLI handle expired sessions?
- **Current Behavior**: API client refreshes token automatically
- **Edge Case**: What if refresh token also expired?
- **Recommendation**: Detect failed refresh, show clear error: "Session expired. Please run `bw login` again."

### Nice-to-Have Questions (Can Decide During Implementation)

**Q9: Login Success Message Format**
- **Question**: Should success message show user email, account type, server URL?
- **Recommendation**: Show email and success: "Logged in as user@example.com"

**Q10: Progress Indicator Style**
- **Question**: Spinner, progress bar, or simple text for key derivation progress?
- **Recommendation**: Simple text: "Deriving master key... (this may take a few seconds)"

**Q11: Password Strength Validation**
- **Question**: Should CLI validate password strength or length on login?
- **Recommendation**: No - server handles this on account creation

---

## Constraints and Limitations

### Technical Constraints

**TC-1: Bitwarden SDK Dependency**
- **Constraint**: All cryptographic operations MUST use Bitwarden SDK
- **Rationale**: Security-critical, requires expert review, already implemented
- **Impact**: Cannot begin crypto implementation until SDK is integrated
- **Mitigation**: Prioritize SDK integration as first implementation step

**TC-2: Master Password Security**
- **Constraint**: Master password MUST NEVER be stored in any form
- **Rationale**: Security best practice, Bitwarden security model
- **Impact**: Always re-derive master key on unlock, cannot cache password
- **Trade-off**: Slower unlock (2-3s) vs security

**TC-3: TypeScript CLI Compatibility**
- **Constraint**: Session key format must be identical to TypeScript CLI
- **Rationale**: Users may switch between TypeScript and Rust CLI
- **Impact**: Must match exact format: 64 bytes base64-encoded
- **Validation**: Test interoperability with TypeScript CLI

**TC-4: Bitwarden API Contract**
- **Constraint**: Must use exact API endpoints and payloads as TypeScript CLI
- **Rationale**: Server expects specific format, changing it could break things
- **Impact**: Cannot optimize or simplify API calls
- **Reference**: TypeScript CLI source code is reference implementation

### Business Constraints

**BC-1: Critical Path Priority**
- **Constraint**: This enhancement blocks all vault operations
- **Impact**: Highest priority after foundational layers (bootstrap, storage, API)
- **Timeline**: Must complete before vault operations can begin

**BC-2: Security First**
- **Constraint**: Security takes precedence over features and performance
- **Impact**: Extensive code review required, security testing mandatory
- **Timeline**: Budget extra time for security validation

### Resource Constraints

**RC-1: Bitwarden SDK Documentation**
- **Constraint**: SDK may have limited or incomplete documentation
- **Impact**: May need to read SDK source code for implementation details
- **Mitigation**: Reference TypeScript CLI as integration example

**RC-2: Testing Accounts**
- **Constraint**: Need test accounts with various configurations (2FA, KDF settings)
- **Impact**: Testing requires multiple account types
- **Mitigation**: Create test accounts during enhancement 1 (project bootstrap)

---

## Success Criteria

### Definition of Done

**Authentication Functionality**:
- [ ] `bw login` with email/password succeeds
- [ ] `bw login --apikey` with API credentials succeeds
- [ ] Two-factor authentication (TOTP) works correctly
- [ ] `bw unlock` decrypts vault successfully after lock
- [ ] `bw lock` clears session state appropriately
- [ ] `bw logout` removes all authentication data
- [ ] Session key format matches TypeScript CLI

**Password Input Options**:
- [ ] `--passwordenv` reads password from environment variable
- [ ] `--passwordfile` reads password from file
- [ ] Interactive password prompt hides input

**Validation Options**:
- [ ] `bw login --check` validates credentials without persisting
- [ ] `bw unlock --check` validates password without updating session
- [ ] Exit codes correctly indicate success/failure

**Security Requirements**:
- [ ] Master password never stored
- [ ] All sensitive buffers zeroized after use
- [ ] Tokens stored encrypted via storage layer
- [ ] No sensitive data in logs or error messages

**Compatibility Requirements**:
- [ ] Session key interoperable with TypeScript CLI
- [ ] API requests match TypeScript CLI format
- [ ] Storage format compatible for migration

**Quality Requirements**:
- [ ] All unit tests pass (>80% coverage target)
- [ ] Integration tests pass with real Bitwarden account
- [ ] Security review completed and approved
- [ ] Documentation complete (usage, API, architecture)

### Validation Tests

**Test Scenario 1: Basic Login Flow**
1. Start with clean state (no stored auth)
2. Run `bw login`
3. Enter valid email and master password
4. Verify session key is displayed
5. Verify subsequent `bw status` shows logged in
6. Verify tokens stored in encrypted storage

**Test Scenario 2: Two-Factor Authentication**
1. Use account with TOTP 2FA enabled
2. Run `bw login`
3. Enter email and password
4. Verify 2FA prompt appears
5. Enter valid TOTP code
6. Verify login succeeds

**Test Scenario 3: Lock/Unlock Cycle**
1. Log in with valid credentials
2. Run `bw lock`
3. Verify vault is locked (user key removed)
4. Run `bw unlock`
5. Enter master password
6. Verify unlock succeeds and session key is displayed

**Test Scenario 4: API Key Authentication**
1. Run `bw login --apikey`
2. Enter valid client_id and client_secret
3. Verify authentication succeeds
4. Verify vault operations work (no master password required)

**Test Scenario 5: Credential Validation**
1. Run `bw login --check`
2. Enter valid credentials
3. Verify exit code 0
4. Verify no tokens stored
5. Verify `bw status` shows not logged in

**Test Scenario 6: Error Handling**
1. Test invalid password → clear error message
2. Test invalid email → "account not found" error
3. Test invalid 2FA code → error with retry option
4. Test network failure → connection error with troubleshooting
5. Test expired session → automatic token refresh

---

## Risk Assessment

### High Risks

**R-1: SDK Integration Complexity**
- **Risk**: Bitwarden SDK may be difficult to integrate or have breaking changes
- **Impact**: Could delay entire enhancement
- **Probability**: Medium
- **Mitigation**: Start with SDK integration first, have fallback plan
- **Owner**: Implementer + Architect

**R-2: Cryptographic Implementation Errors**
- **Risk**: Incorrect key derivation or encryption could corrupt vaults
- **Impact**: CRITICAL - users could lose access to vaults
- **Probability**: Low (using SDK reduces risk)
- **Mitigation**: Extensive testing, security code review, use SDK for all crypto
- **Owner**: Implementer + Security Reviewer

**R-3: TypeScript CLI Compatibility**
- **Risk**: Session key format or storage incompatibility prevents migration
- **Impact**: Users cannot switch between TypeScript and Rust CLI
- **Probability**: Medium
- **Mitigation**: Integration tests with TypeScript CLI, validate session key format
- **Owner**: Tester

### Medium Risks

**R-4: SSO Flow Complexity**
- **Risk**: SSO implementation in CLI context is complex and may have edge cases
- **Impact**: SSO users cannot migrate to Rust CLI
- **Probability**: Medium
- **Mitigation**: Consider deferring SSO to post-MVP if too complex
- **Owner**: Architect (decision to defer)

**R-5: 2FA Provider Coverage**
- **Risk**: May not support all 2FA methods (Duo, FIDO2, etc.)
- **Impact**: Users with unsupported 2FA cannot use Rust CLI
- **Probability**: Medium
- **Mitigation**: Prioritize common methods (TOTP, email), add others incrementally
- **Owner**: Product (prioritization)

**R-6: Performance on Low-End Hardware**
- **Risk**: Key derivation may be too slow on resource-constrained devices
- **Impact**: Poor user experience on slow machines
- **Probability**: Low
- **Mitigation**: Show clear progress indicator, use optimized Argon2id implementation
- **Owner**: Implementer

### Low Risks

**R-7: Error Message Clarity**
- **Risk**: Error messages may not be helpful enough for users
- **Impact**: Poor UX, support burden
- **Probability**: Low
- **Mitigation**: User testing, compare with TypeScript CLI messages
- **Owner**: Tester + Documenter

---

## Dependencies and Blocking Issues

### Hard Dependencies (Must Complete First)

**D-1: Project Bootstrap (Enhancement 1)**
- **Status**: ✅ COMPLETE
- **Blocks**: All implementation work
- **Provides**: Project structure, CI/CD, basic CLI framework

**D-2: Storage Layer (Enhancement 2)**
- **Status**: ✅ COMPLETE
- **Blocks**: Token persistence, encrypted storage
- **Provides**: `Storage` trait, encrypted value storage

**D-3: API Client (Enhancement 3)**
- **Status**: ✅ COMPLETE
- **Blocks**: Authentication API calls, token refresh
- **Provides**: `ApiClient` trait, HTTP communication

**D-4: Bitwarden SDK Integration**
- **Status**: ⏳ NOT STARTED
- **Blocks**: All cryptographic operations
- **Provides**: Key derivation, encryption/decryption, password hashing
- **Critical**: Must be first task in implementation phase

### Soft Dependencies (Nice to Have)

**D-5: Interactive Prompt Library**
- **Status**: Need to select and integrate (dialoguer, inquire, or similar)
- **Blocks**: User-friendly password prompts, 2FA method selection
- **Fallback**: Can use basic stdin/stdout temporarily

**D-6: Progress Indicator Library**
- **Status**: Need to select (indicatif or similar)
- **Blocks**: Key derivation progress display
- **Fallback**: Simple text messages work but less polished

### Blocking Downstream

**Blocks Enhancement 5**: Vault Read Commands (needs authentication)
**Blocks Enhancement 6**: Vault Write Commands (needs authentication)
**Blocks Enhancement 7**: Tool Commands (needs authentication)
**Blocks Enhancement 8**: Import/Export (needs authentication)

**Critical Path**: This is the longest pole - delays here delay everything

---

## Phased Implementation Recommendation

### Phase 1: Foundation (MVP Core)

**Scope**: Basic login/logout with password authentication

**Features**:
- `bw login` with email/password
- Master key derivation (PBKDF2 + Argon2id)
- User key decryption
- Session key generation
- Token storage (via storage layer)
- `bw logout` to clear state
- `bw unlock` with password
- `bw lock` to clear session

**Success Criteria**: Users can authenticate and access vault

**Estimated Effort**: 5-7 days

**Dependencies**: SDK integration complete

**Deliverables**:
- Auth commands implementation
- SDK crypto integration
- Unit tests for key derivation
- Integration tests for login flow

### Phase 2: Extended Authentication (MVP Complete)

**Scope**: API key, 2FA, password input options

**Features**:
- `bw login --apikey` for API key authentication
- Two-factor authentication (TOTP, email)
- `--passwordenv` and `--passwordfile` flags
- `--check` flag for validation
- Error handling and user feedback

**Success Criteria**: Feature parity with TypeScript CLI (minus SSO)

**Estimated Effort**: 3-4 days

**Dependencies**: Phase 1 complete

**Deliverables**:
- API key flow implementation
- 2FA prompt and validation
- Password input option handlers
- Integration tests for all flows

### Phase 3: SSO and Advanced Features (Post-MVP)

**Scope**: SSO, YubiKey, remember device

**Features**:
- `bw login --sso` with browser flow
- YubiKey hardware token support
- Remember device functionality
- Login history tracking

**Success Criteria**: Full feature parity including SSO

**Estimated Effort**: 4-5 days

**Dependencies**: Phases 1-2 complete, product decision on SSO priority

**Deliverables**:
- SSO flow implementation
- YubiKey integration
- Remember device logic
- Comprehensive integration tests

**Recommendation**: Consider deferring Phase 3 if SSO complexity is high

---

## Technical Debt and Future Considerations

### Known Limitations

**L-1: Single Account Support**
- **Current**: Only one logged-in account at a time
- **Future**: Multi-account support with profile switching
- **Impact**: Power users with multiple accounts must logout/login to switch
- **Timeline**: Post-MVP (Enhancement 9+)

**L-2: No Biometric Unlock**
- **Current**: Only master password unlock
- **Future**: Platform biometric integration (Touch ID, Windows Hello)
- **Impact**: Mobile-like convenience not available
- **Complexity**: High (platform-specific implementations)
- **Timeline**: Post-MVP, requires research

**L-3: Limited 2FA Methods in MVP**
- **Current**: TOTP, email, YubiKey (Phase 2-3)
- **Future**: Duo, FIDO2, hardware security keys
- **Impact**: Users with unsupported 2FA methods cannot use Rust CLI
- **Timeline**: Add incrementally based on usage data

### Technical Debt to Address

**TD-1: Error Message Consistency**
- **Issue**: Need to ensure error messages match TypeScript CLI where sensible
- **Effort**: Low (review and update messages)
- **Priority**: Medium
- **Timeline**: During implementation

**TD-2: Telemetry and Analytics**
- **Issue**: May want to track login method usage, error rates
- **Effort**: Medium (requires privacy-conscious implementation)
- **Priority**: Low
- **Timeline**: Post-MVP

**TD-3: Session Timeout Policy**
- **Issue**: Current design has no session timeout
- **Risk**: Long-lived sessions could be security risk
- **Consideration**: Should vault auto-lock after inactivity?
- **Decision Needed**: Product/Security
- **Timeline**: Can be added later without breaking changes

---

## Recommendations for Architecture Team

### Prioritization Guidance

1. **Must Have (Phase 1)**:
   - Email/password login ✅
   - Master key derivation (PBKDF2 + Argon2id) ✅
   - Unlock/lock/logout ✅
   - Session key generation ✅

2. **Should Have (Phase 2)**:
   - API key authentication ✅
   - Two-factor authentication (TOTP, email) ✅
   - Password input options (env, file) ✅
   - Validation mode (--check) ✅

3. **Nice to Have (Phase 3 / Post-MVP)**:
   - SSO flow ⚠️ (complex, consider deferring)
   - YubiKey support ⚠️ (if time allows)
   - Remember device ⚠️ (security review needed)

### Architecture Decisions Needed

**AD-1: SSO Implementation Approach**
- **Question**: Proceed with SSO in MVP or defer to post-MVP?
- **Recommendation**: Defer to post-MVP if it adds >2 days of effort
- **Rationale**: SSO is complex in CLI, used by minority of users, can add later

**AD-2: Interactive Prompt Library Selection**
- **Options**: dialoguer, inquire, custom implementation
- **Recommendation**: Use `dialoguer` (proven, widely used)
- **Rationale**: Battle-tested, good UX, actively maintained

**AD-3: Progress Indicator Implementation**
- **Options**: indicatif library, simple text, none
- **Recommendation**: Use `indicatif` for spinners/progress bars
- **Rationale**: Key derivation can take 2-3s, user needs feedback

**AD-4: Session State Management**
- **Question**: Where should session state be managed? (Commands vs Container vs dedicated SessionManager?)
- **Recommendation**: Create `SessionManager` service in container
- **Rationale**: Centralizes session logic, testable, reusable across commands

**AD-5: Error Hierarchy**
- **Question**: Should authentication errors be separate type or use general error types?
- **Recommendation**: Create `AuthError` enum in auth module
- **Rationale**: Specific error types for better error handling, maps to user messages

### Implementation Risks to Monitor

1. **SDK Integration Issues**: Watch for API changes, breaking updates
2. **Crypto Performance**: Monitor key derivation speed on various hardware
3. **TypeScript Compatibility**: Validate session key format early and often
4. **2FA Edge Cases**: Test thoroughly with all 2FA methods
5. **Error Message Quality**: User test for clarity and actionability

---

## Appendix

### A. Terminology

- **Master Password**: User's memorable password, used to derive master key
- **Master Key**: 256-bit cryptographic key derived from master password
- **User Key**: Symmetric key used to encrypt/decrypt vault items
- **Session Key**: 512-bit key (256-bit encryption + 256-bit MAC) for local storage encryption
- **Access Token**: JWT token for API authentication (short-lived, 1 hour)
- **Refresh Token**: Long-lived token (30 days) to obtain new access tokens
- **KDF**: Key Derivation Function (PBKDF2 or Argon2id)
- **2FA**: Two-Factor Authentication
- **TOTP**: Time-based One-Time Password (authenticator app codes)
- **EncString**: Bitwarden's encrypted string format: `type.iv|ciphertext|mac`
- **BW_SESSION**: Environment variable containing session key for vault access

### B. Reference Documentation

**TypeScript CLI Source Files** (for reference during implementation):
- `apps/cli/src/auth/commands/login.command.ts` - Login command implementation
- `apps/cli/src/auth/commands/logout.command.ts` - Logout command
- `apps/cli/src/auth/commands/lock.command.ts` - Lock command
- `apps/cli/src/key-management/commands/unlock.command.ts` - Unlock command
- `libs/common/src/auth/services/key-connector.service.ts` - Key management
- `libs/common/src/platform/services/crypto.service.ts` - Crypto operations

**Bitwarden API Endpoints**:
- `POST /identity/accounts/prelogin` - Get KDF parameters
- `POST /identity/connect/token` - OAuth2 token endpoint
- `GET /api/accounts/profile` - User profile information

**Bitwarden Documentation**:
- [Encryption Overview](https://bitwarden.com/help/bitwarden-security-white-paper/)
- [KDF Information](https://bitwarden.com/help/kdf-algorithms/)
- [CLI Documentation](https://bitwarden.com/help/cli/)

### C. Data Models

**Authentication Request (Password Login)**:
```json
{
  "grant_type": "password",
  "username": "user@example.com",
  "password": "base64_hashed_password",
  "scope": "api offline_access",
  "client_id": "cli",
  "deviceType": 8,
  "deviceName": "rust-cli",
  "deviceIdentifier": "uuid",
  "twoFactorToken": "optional_2fa_code",
  "twoFactorProvider": 0
}
```

**Authentication Response**:
```json
{
  "access_token": "jwt_token",
  "expires_in": 3600,
  "token_type": "Bearer",
  "refresh_token": "long_lived_token",
  "Key": "encrypted_user_key",
  "PrivateKey": "encrypted_private_key",
  "Kdf": 0,
  "KdfIterations": 600000,
  "ResetMasterPassword": false,
  "scope": "api offline_access",
  "unofficialServer": false
}
```

**Prelogin Request/Response**:
```json
// Request
{
  "email": "user@example.com"
}

// Response
{
  "kdf": 0,  // 0 = PBKDF2, 1 = Argon2id
  "kdfIterations": 600000,
  "kdfMemory": 64,       // Argon2id only
  "kdfParallelism": 4    // Argon2id only
}
```

### D. Storage Keys

**Encrypted Keys** (use `__PROTECTED__` prefix):
- `__PROTECTED__accessToken` - JWT access token
- `__PROTECTED__refreshToken` - OAuth2 refresh token
- `__PROTECTED__userKey` - Encrypted symmetric vault key

**Plain Keys**:
- `userProfile` - User profile JSON
  ```json
  {
    "id": "uuid",
    "email": "user@example.com",
    "name": "User Name",
    "emailVerified": true,
    "premium": false
  }
  ```

- `kdfConfig` - KDF configuration JSON
  ```json
  {
    "kdf": 0,
    "kdfIterations": 600000,
    "kdfMemory": 64,
    "kdfParallelism": 4
  }
  ```

- `environmentUrls` - Server URLs (already stored by config commands)

---

## Status

**Completion Status**: READY_FOR_ARCHITECTURE

**Confidence Level**: HIGH

**Analysis Quality**:
- ✅ Requirements extracted from specification
- ✅ Integration points identified with existing enhancements
- ✅ User stories created with acceptance criteria
- ✅ Open questions documented clearly
- ✅ Risks assessed and mitigated
- ✅ Phased implementation recommended
- ✅ Technical constraints identified
- ✅ Success criteria defined

**Key Findings**:
1. **Complexity is HIGH** due to cryptographic operations and multiple auth flows
2. **SDK integration is critical** - should be first implementation task
3. **TypeScript CLI compatibility** must be validated throughout development
4. **SSO flow** is complex and recommended for deferral to post-MVP
5. **Security review** is mandatory before production use

**Next Steps**:
1. Architect reviews this analysis
2. Architect makes decisions on open questions (especially Q1-Q3)
3. Architect creates detailed implementation plan
4. Implementer begins with SDK integration
5. Tester develops comprehensive test suite

**Estimated Timeline** (after architecture):
- Phase 1 (MVP Core): 5-7 days
- Phase 2 (Extended Auth): 3-4 days
- Phase 3 (SSO, optional): 4-5 days
- **Total**: 8-12 days for MVP (Phases 1-2)

**Blockers**: None - ready for architecture phase

**Concerns**: None major. SSO complexity is the only significant risk, but it can be deferred.

---

**Document Version**: 1.0
**Author**: Requirements Analyst Agent
**Date**: 2025-12-03
**Enhancement**: 04-auth-commands
**Status**: READY_FOR_ARCHITECTURE
