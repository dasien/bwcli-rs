---
enhancement: 04-auth-commands
agent: architect
task_id: task_1764798090_76167
timestamp: 2025-12-03T21:15:00Z
status: READY_FOR_IMPLEMENTATION
---

# Authentication Commands Implementation Plan

## Executive Summary

This document provides the technical architecture and implementation plan for authentication commands (`login`, `logout`, `lock`, `unlock`) in the Rust Bitwarden CLI. This implementation is **critical path** for the entire project, blocking all vault operations.

**Key Architecture Decisions**:
1. **Bitwarden SDK Integration**: All cryptographic operations use SDK - no manual crypto implementation
2. **Modular Command Pattern**: Each command is a self-contained module with clear responsibilities
3. **Layered Services Architecture**: Auth service layer separates business logic from command handlers
4. **Interactive Prompt Library**: Use `dialoguer` for user-friendly password/2FA prompts
5. **Session Management**: Centralized session manager handles session key lifecycle

**Implementation Phases**:
- **Phase 1 (MVP Core)**: Password login, unlock, lock, logout - 5-7 days
- **Phase 2 (Extended Auth)**: API key, 2FA, password input options - 3-4 days
- **Phase 3 (Post-MVP)**: SSO, YubiKey - Deferred

**Risk Mitigation**:
- SDK integration is first task to unblock crypto operations
- Extensive testing required for security-critical code
- TypeScript CLI compatibility validated through integration tests

---

## Table of Contents

1. [System Architecture](#system-architecture)
2. [Component Design](#component-design)
3. [API Integration Design](#api-integration-design)
4. [SDK Crypto Integration](#sdk-crypto-integration)
5. [Data Models](#data-models)
6. [Command Implementations](#command-implementations)
7. [Error Handling Strategy](#error-handling-strategy)
8. [Security Architecture](#security-architecture)
9. [Testing Strategy](#testing-strategy)
10. [Implementation Phases](#implementation-phases)
11. [Open Questions & Decisions](#open-questions--decisions)

---

## System Architecture

### High-Level Component Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                     CLI Layer (bw-cli)                      │
│  ┌─────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │LoginCommand │  │UnlockCommand │  │LockCommand   │      │
│  └──────┬──────┘  └──────┬───────┘  └──────┬───────┘      │
│         │                 │                  │               │
│         └─────────────────┴──────────────────┘               │
│                           │                                  │
│                    ┌──────▼────────┐                        │
│                    │  AuthService  │◄─────┐                 │
│                    └──────┬────────┘      │                 │
└───────────────────────────┼───────────────┼─────────────────┘
                            │               │
┌───────────────────────────┼───────────────┼─────────────────┐
│              Core Layer (bw-core)         │                 │
│                           │               │                 │
│  ┌────────────────────────▼───────┐       │                 │
│  │     ServiceContainer           │       │                 │
│  │  ┌──────────┐ ┌──────────┐   │       │                 │
│  │  │ Storage  │ │ApiClient │   │       │                 │
│  │  └────┬─────┘ └────┬─────┘   │       │                 │
│  │       │            │          │       │                 │
│  │  ┌────▼─────┐ ┌───▼──────┐   │       │                 │
│  │  │JsonFile  │ │Bitwarden │   │       │                 │
│  │  │Storage   │ │ApiClient │   │       │                 │
│  │  └──────────┘ └──────────┘   │       │                 │
│  └───────────────────────────────┘       │                 │
│                                           │                 │
│  ┌────────────────────────────────────┐  │                 │
│  │    Bitwarden SDK (bitwarden-core)  │◄─┘                 │
│  │  ┌────────────┐  ┌──────────────┐ │                    │
│  │  │MasterKey   │  │UserKey       │ │                    │
│  │  │Derivation  │  │Encryption/   │ │                    │
│  │  │(PBKDF2/    │  │Decryption    │ │                    │
│  │  │ Argon2id)  │  │              │ │                    │
│  │  └────────────┘  └──────────────┘ │                    │
│  └────────────────────────────────────┘                    │
└─────────────────────────────────────────────────────────────┘
```

### Layer Responsibilities

**CLI Layer (bw-cli crate)**:
- Command parsing and validation (clap)
- User interaction (prompts, output formatting)
- Global flag handling (--session, --quiet, --raw, etc.)
- Command orchestration (calls services, handles responses)

**Core Layer (bw-core crate)**:
- Business logic implementation (AuthService)
- Service coordination (ServiceContainer)
- API communication (BitwardenApiClient)
- Storage management (JsonFileStorage)
- Data models (UserProfile, KdfConfig, etc.)

**SDK Layer (Bitwarden SDK)**:
- All cryptographic operations
- Key derivation (PBKDF2-SHA256, Argon2id)
- Encryption/decryption (AES-256-CBC/GCM)
- EncString parsing and validation
- TOTP generation

---

## Component Design

### 1. AuthService (New Component)

**Location**: `crates/bw-core/src/services/auth/mod.rs`

**Purpose**: Centralized authentication business logic, orchestrates SDK crypto, API calls, and storage.

**Public Interface**:

```rust
pub struct AuthService {
    storage: Arc<JsonFileStorage>,
    api_client: Arc<BitwardenApiClient>,
    sdk_client: Arc<Client>, // Bitwarden SDK client
    session_manager: Arc<SessionManager>,
}

impl AuthService {
    /// Password-based login flow
    pub async fn login_with_password(
        &self,
        email: &str,
        password: Secret<String>,
        two_factor: Option<TwoFactorData>,
    ) -> Result<LoginResult>;

    /// API key login flow
    pub async fn login_with_api_key(
        &self,
        client_id: &str,
        client_secret: Secret<String>,
    ) -> Result<LoginResult>;

    /// Unlock vault with master password
    pub async fn unlock(
        &self,
        password: Secret<String>,
    ) -> Result<UnlockResult>;

    /// Lock vault (clear session keys)
    pub async fn lock(&self) -> Result<()>;

    /// Logout (clear all auth state)
    pub async fn logout(&self) -> Result<()>;

    /// Validate credentials without persisting
    pub async fn check_credentials(
        &self,
        email: &str,
        password: Secret<String>,
    ) -> Result<bool>;
}
```

**Internal Methods**:

```rust
impl AuthService {
    /// Fetch KDF parameters from server
    async fn fetch_kdf_config(&self, email: &str) -> Result<KdfConfig>;

    /// Derive master key using SDK
    async fn derive_master_key(
        &self,
        password: &Secret<String>,
        email: &str,
        kdf_config: &KdfConfig,
    ) -> Result<MasterKey>;

    /// Hash password for authentication request
    async fn hash_password_for_auth(
        &self,
        password: &Secret<String>,
        master_key: &MasterKey,
    ) -> Result<String>;

    /// Decrypt user key from encrypted key
    async fn decrypt_user_key(
        &self,
        encrypted_key: &str,
        master_key: &MasterKey,
    ) -> Result<UserKey>;

    /// Store authentication state
    async fn persist_auth_state(
        &self,
        user_id: &str,
        email: &str,
        access_token: &str,
        refresh_token: &str,
        encrypted_user_key: &str,
        kdf_config: &KdfConfig,
    ) -> Result<()>;
}
```

**Design Rationale**:
- **Single Responsibility**: AuthService owns all authentication flows
- **Testability**: Can mock dependencies (storage, API, SDK) for unit tests
- **Reusability**: Shared logic (KDF, key derivation) is centralized
- **Security**: Sensitive operations (key handling) are encapsulated

---

### 2. SessionManager (New Component)

**Location**: `crates/bw-core/src/services/auth/session_manager.rs`

**Purpose**: Manages session key lifecycle and BW_SESSION environment variable integration.

**Public Interface**:

```rust
pub struct SessionManager {
    storage: Arc<JsonFileStorage>,
}

impl SessionManager {
    /// Generate new session key (512 bits = 64 bytes)
    pub fn generate_session_key() -> SessionKey;

    /// Store session key in storage
    pub async fn save_session_key(&self, key: &SessionKey) -> Result<()>;

    /// Load session key from BW_SESSION or storage
    pub async fn load_session_key(&self) -> Result<Option<SessionKey>>;

    /// Clear session key
    pub async fn clear_session_key(&self) -> Result<()>;

    /// Format session key for environment variable export
    pub fn format_for_export(key: &SessionKey) -> String;

    /// Validate session key format
    pub fn validate_session_key(key_str: &str) -> Result<SessionKey>;
}

// Helper type
pub struct SessionKey {
    encryption_key: [u8; 32], // AES-256 key
    mac_key: [u8; 32],        // HMAC-SHA256 key
}

impl SessionKey {
    /// Convert to base64 for storage/export
    pub fn to_base64(&self) -> String;

    /// Parse from base64
    pub fn from_base64(encoded: &str) -> Result<Self>;
}
```

**Design Rationale**:
- **TypeScript CLI Compatibility**: Session key format must match exactly (64 bytes base64)
- **Security**: Uses cryptographically secure RNG (`rand::rngs::OsRng`)
- **Environment Integration**: Handles BW_SESSION variable parsing and validation

---

### 3. Command Modules Structure

**Location**: `crates/bw-cli/src/commands/auth/`

**New File Structure**:

```
crates/bw-cli/src/commands/
├── auth/
│   ├── mod.rs              # Command types and routing
│   ├── login.rs            # Login command implementations
│   ├── unlock.rs           # Unlock command
│   ├── lock.rs             # Lock command
│   ├── logout.rs           # Logout command
│   ├── prompts.rs          # User interaction helpers
│   └── validators.rs       # Input validation
├── config.rs
├── status.rs
└── ... (other commands)
```

**Command Pattern**:

Each command follows this structure:

```rust
// Example: login.rs
use crate::{GlobalArgs, output::Response};
use bw_core::services::auth::AuthService;
use anyhow::Result;

pub async fn execute_password_login(
    cmd: LoginPasswordCommand,
    global_args: &GlobalArgs,
    auth_service: &AuthService,
) -> Result<Response> {
    // 1. Gather inputs (prompt if missing)
    let email = get_email_input(cmd.email, global_args)?;
    let password = get_password_input(cmd.password, global_args)?;

    // 2. Call service layer
    let result = auth_service
        .login_with_password(&email, password, None)
        .await?;

    // 3. Format output
    Ok(Response::success(format!(
        "Logged in as {}\n\nTo unlock your vault, set your session key:\n\n\
         export BW_SESSION=\"{}\"",
        result.email,
        result.session_key
    )))
}

// Helper functions
fn get_email_input(
    email_arg: Option<String>,
    global_args: &GlobalArgs,
) -> Result<String> {
    if let Some(email) = email_arg {
        return Ok(email);
    }

    if global_args.nointeraction {
        anyhow::bail!("Email is required. Use --nointeraction=false or provide EMAIL argument.");
    }

    prompts::prompt_email()
}
```

---

### 4. Interactive Prompts Module

**Location**: `crates/bw-cli/src/commands/auth/prompts.rs`

**Dependencies**: Add `dialoguer = "0.11"` to `Cargo.toml`

**Implementation**:

```rust
use dialoguer::{Input, Password, Select};
use anyhow::Result;
use secrecy::Secret;

/// Prompt for email address
pub fn prompt_email() -> Result<String> {
    let email: String = Input::new()
        .with_prompt("Email address")
        .interact_text()?;

    validate_email(&email)?;
    Ok(email)
}

/// Prompt for password (hidden input)
pub fn prompt_password() -> Result<Secret<String>> {
    let password = Password::new()
        .with_prompt("Master password")
        .interact()?;

    if password.is_empty() {
        anyhow::bail!("Password cannot be empty");
    }

    Ok(Secret::new(password))
}

/// Prompt for 2FA method selection
pub fn prompt_two_factor_method(
    available_methods: &[TwoFactorMethod],
) -> Result<TwoFactorMethod> {
    let methods: Vec<&str> = available_methods
        .iter()
        .map(|m| m.display_name())
        .collect();

    let selection = Select::new()
        .with_prompt("Two-step login method")
        .items(&methods)
        .default(0)
        .interact()?;

    Ok(available_methods[selection])
}

/// Prompt for 2FA code
pub fn prompt_two_factor_code(method: TwoFactorMethod) -> Result<String> {
    let prompt = match method {
        TwoFactorMethod::Authenticator => "Authenticator app code",
        TwoFactorMethod::Email => "Email verification code",
        TwoFactorMethod::YubiKey => "YubiKey OTP",
        _ => "Two-factor code",
    };

    let code: String = Input::new()
        .with_prompt(prompt)
        .interact_text()?;

    // Basic validation: TOTP codes are 6 digits
    if method == TwoFactorMethod::Authenticator {
        if !code.chars().all(|c| c.is_ascii_digit()) || code.len() != 6 {
            anyhow::bail!("Authenticator code must be 6 digits");
        }
    }

    Ok(code)
}

/// Prompt for confirmation
pub fn prompt_confirmation(prompt: &str) -> Result<bool> {
    dialoguer::Confirm::new()
        .with_prompt(prompt)
        .default(false)
        .interact()
}
```

**Design Rationale**:
- **User-Friendly**: `dialoguer` provides polished, cross-platform prompts
- **Security**: Password prompts hide input
- **Validation**: Input validation happens at prompt level
- **Testability**: Prompts can be skipped with `--nointeraction` flag

---

## API Integration Design

### 1. Authentication API Endpoints

**Prelogin Request** (Get KDF Parameters):

```rust
// Request
#[derive(Serialize)]
struct PreloginRequest {
    email: String,
}

// Response
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PreloginResponse {
    kdf: u8,                  // 0 = PBKDF2, 1 = Argon2id
    kdf_iterations: u32,      // PBKDF2: iterations (600k default)
    kdf_memory: Option<u32>,  // Argon2id: memory in MB (64 default)
    kdf_parallelism: Option<u32>, // Argon2id: parallelism (4 default)
}

// API Call
let response: PreloginResponse = api_client
    .post("/identity/accounts/prelogin", &PreloginRequest { email })
    .await?;
```

**Password Login Request** (OAuth2 Password Grant):

```rust
// Request (form-encoded, not JSON!)
#[derive(Serialize)]
struct PasswordLoginRequest {
    grant_type: String,           // "password"
    username: String,             // email
    password: String,             // base64 hashed password
    scope: String,                // "api offline_access"
    client_id: String,            // "cli"
    #[serde(rename = "deviceType")]
    device_type: u8,              // 8 (CLI)
    #[serde(rename = "deviceName")]
    device_name: String,          // "rust-cli"
    #[serde(rename = "deviceIdentifier")]
    device_identifier: String,    // UUID

    // Optional 2FA fields
    #[serde(rename = "twoFactorToken", skip_serializing_if = "Option::is_none")]
    two_factor_token: Option<String>,
    #[serde(rename = "twoFactorProvider", skip_serializing_if = "Option::is_none")]
    two_factor_provider: Option<u8>,
    #[serde(rename = "twoFactorRemember", skip_serializing_if = "Option::is_none")]
    two_factor_remember: Option<u8>, // 0 = false, 1 = true
}

// Response
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoginResponse {
    access_token: String,
    expires_in: i64,              // Seconds (typically 3600 = 1 hour)
    token_type: String,           // "Bearer"
    refresh_token: String,

    #[serde(rename = "Key")]
    key: String,                  // Encrypted user key (EncString format)

    #[serde(rename = "PrivateKey")]
    private_key: Option<String>,  // Encrypted RSA private key

    #[serde(rename = "Kdf")]
    kdf: u8,
    #[serde(rename = "KdfIterations")]
    kdf_iterations: u32,
    #[serde(rename = "KdfMemory")]
    kdf_memory: Option<u32>,
    #[serde(rename = "KdfParallelism")]
    kdf_parallelism: Option<u32>,

    #[serde(rename = "ResetMasterPassword")]
    reset_master_password: bool,

    // 2FA fields (if 2FA required)
    #[serde(rename = "TwoFactorProviders")]
    two_factor_providers: Option<Vec<u8>>,
    #[serde(rename = "TwoFactorProviders2")]
    two_factor_providers2: Option<serde_json::Value>,
}

// API Call (note: form-encoded, not JSON!)
let response: LoginResponse = api_client
    .post_form("/identity/connect/token", &login_request)
    .await?;
```

**API Key Login Request** (OAuth2 Client Credentials):

```rust
#[derive(Serialize)]
struct ApiKeyLoginRequest {
    grant_type: String,           // "client_credentials"
    client_id: String,            // "user.{uuid}"
    client_secret: String,        // Client secret
    scope: String,                // "api"
    #[serde(rename = "deviceType")]
    device_type: u8,              // 8 (CLI)
    #[serde(rename = "deviceName")]
    device_name: String,
    #[serde(rename = "deviceIdentifier")]
    device_identifier: String,
}

// Response: Same as LoginResponse but Key is null
```

**User Profile Request** (After Login):

```rust
// GET /api/accounts/profile (with Bearer token)
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProfileResponse {
    id: String,                   // User UUID
    email: String,
    name: Option<String>,
    email_verified: bool,
    premium: bool,
    security_stamp: String,       // For session invalidation
    // ... other profile fields
}
```

### 2. API Client Extensions

**Add Form-Encoded POST Method**:

The `/identity/connect/token` endpoint requires `application/x-www-form-urlencoded` encoding, not JSON.

**Location**: `crates/bw-core/src/services/api/client.rs`

**Add to BitwardenApiClient**:

```rust
/// Post form-encoded data (for OAuth2 token endpoint)
pub async fn post_form<T, R>(&self, path: &str, body: &T) -> Result<R>
where
    T: Serialize + Send + Sync,
    R: for<'de> Deserialize<'de>,
{
    let url = self.build_url(path, path.starts_with("/identity"));

    let request = self
        .http_client
        .post(&url)
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .form(body) // form() handles urlencoding
        .build()?;

    let response = self.execute_with_retry(request, false).await?;
    let data: R = response.json().await?;

    Ok(data)
}
```

### 3. 2FA Detection and Handling

**Error Response** (When 2FA Required):

When 2FA is required, the token endpoint returns HTTP 400 with this body:

```json
{
  "error": "invalid_grant",
  "error_description": "Two factor required.",
  "TwoFactorProviders": [0, 1],
  "TwoFactorProviders2": {
    "0": null,
    "1": {
      "Email": "u***@example.com"
    }
  }
}
```

**2FA Provider Codes**:
- `0` = Authenticator (TOTP)
- `1` = Email
- `2` = Duo
- `3` = YubiKey
- `4` = U2F
- `5` = Remember (device)
- `6` = OrganizationDuo
- `7` = WebAuthn

**Implementation Strategy**:

```rust
// In AuthService::login_with_password
match api_client.post_form("/identity/connect/token", &request).await {
    Ok(response) => Ok(response),
    Err(e) if is_two_factor_required(&e) => {
        // Extract available 2FA methods from error
        let providers = extract_two_factor_providers(&e)?;

        // Prompt user to select method and enter code
        let method = prompts::prompt_two_factor_method(&providers)?;
        let code = prompts::prompt_two_factor_code(method)?;

        // Retry with 2FA token
        let two_factor = TwoFactorData {
            token: code,
            provider: method.to_provider_code(),
            remember: false,
        };

        // Recursive call with 2FA data
        self.login_with_password_internal(email, password, Some(two_factor)).await
    }
    Err(e) => Err(e),
}
```

---

## SDK Crypto Integration

### 1. SDK Setup and Configuration

**Cargo.toml Changes**:

```toml
[workspace.dependencies]
# Uncomment these lines (currently lines 30-33)
bitwarden-core = { path = "../sdk/crates/bitwarden-core" }
bitwarden-crypto = { path = "../sdk/crates/bitwarden-crypto" }

# CLI crate needs prompts
dialoguer = "0.11"
indicatif = "0.17" # For progress indicators
```

**SDK Client Initialization** (Already exists, needs enhancement):

**Location**: `crates/bw-core/src/services/sdk.rs`

Replace mock with real SDK:

```rust
use bitwarden_core::{Client, ClientSettings, DeviceType};
use anyhow::Result;

pub fn create_sdk_client(
    api_url: Option<String>,
    identity_url: Option<String>,
) -> Result<Client> {
    let settings = ClientSettings {
        api_url: api_url.unwrap_or_else(|| "https://api.bitwarden.com".to_string()),
        identity_url: identity_url
            .unwrap_or_else(|| "https://identity.bitwarden.com".to_string()),
        device_type: DeviceType::CLI,
        user_agent: format!("Bitwarden_CLI/{}", env!("CARGO_PKG_VERSION")),
    };

    Client::new(Some(settings))
}
```

### 2. Master Key Derivation

**SDK Methods to Use**:

```rust
// From bitwarden-crypto crate
use bitwarden_crypto::{
    derive_master_key_pbkdf2,
    derive_master_key_argon2,
    MasterKey
};

// PBKDF2-SHA256 derivation
let master_key: MasterKey = derive_master_key_pbkdf2(
    password.expose_secret().as_bytes(),
    email.as_bytes(), // Email is used as salt
    kdf_config.kdf_iterations.unwrap_or(600_000),
)?;

// Argon2id derivation
let master_key: MasterKey = derive_master_key_argon2(
    password.expose_secret().as_bytes(),
    email.as_bytes(),
    kdf_config.kdf_memory.unwrap_or(64),      // MB
    kdf_config.kdf_iterations.unwrap_or(3),   // Iterations
    kdf_config.kdf_parallelism.unwrap_or(4),  // Threads
)?;
```

**Progress Indicator** (for slow operations):

```rust
use indicatif::{ProgressBar, ProgressStyle};

async fn derive_master_key_with_progress(
    password: &Secret<String>,
    email: &str,
    kdf_config: &KdfConfig,
) -> Result<MasterKey> {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner} {msg}")
            .unwrap()
    );
    pb.set_message("Deriving master key (this may take a few seconds)...");

    // Run KDF in blocking task (CPU-intensive)
    let password_clone = password.clone();
    let email_clone = email.to_string();
    let kdf_clone = kdf_config.clone();

    let master_key = tokio::task::spawn_blocking(move || {
        derive_master_key(&password_clone, &email_clone, &kdf_clone)
    })
    .await??;

    pb.finish_with_message("Master key derived");

    Ok(master_key)
}
```

### 3. Password Hashing for Authentication

**SDK Method**:

```rust
use bitwarden_crypto::{hash_password, MasterKey};

// Hash password for server authentication
// Result is base64-encoded PBKDF2 hash
let hashed_password: String = hash_password(
    password.expose_secret(),
    &master_key,
    1, // Additional iteration (total = KDF iterations + 1)
)?;
```

**Note**: The hashed password is NOT the master key. It's a separate hash sent to the server for authentication.

### 4. User Key Decryption

**SDK Method**:

```rust
use bitwarden_crypto::{decrypt_enc_string, EncString, MasterKey, UserKey};

// Parse encrypted key from server response
let enc_string = EncString::from_str(&encrypted_key_from_server)?;

// Decrypt using master key
let user_key: UserKey = decrypt_user_key(&enc_string, &master_key)?;

// UserKey is 64 bytes: [32 bytes encryption key | 32 bytes MAC key]
```

### 5. Session Key Generation

**Implementation** (Not SDK - use `rand`):

```rust
use rand::{rngs::OsRng, RngCore};
use base64::{Engine as _, engine::general_purpose::STANDARD};

pub fn generate_session_key() -> SessionKey {
    let mut key_bytes = [0u8; 64]; // 512 bits
    OsRng.fill_bytes(&mut key_bytes);

    SessionKey {
        encryption_key: key_bytes[0..32].try_into().unwrap(),
        mac_key: key_bytes[32..64].try_into().unwrap(),
    }
}

impl SessionKey {
    pub fn to_base64(&self) -> String {
        let mut bytes = Vec::with_capacity(64);
        bytes.extend_from_slice(&self.encryption_key);
        bytes.extend_from_slice(&self.mac_key);
        STANDARD.encode(&bytes)
    }

    pub fn from_base64(encoded: &str) -> Result<Self> {
        let bytes = STANDARD.decode(encoded)?;
        if bytes.len() != 64 {
            anyhow::bail!("Invalid session key length: expected 64 bytes, got {}", bytes.len());
        }

        Ok(SessionKey {
            encryption_key: bytes[0..32].try_into().unwrap(),
            mac_key: bytes[32..64].try_into().unwrap(),
        })
    }
}
```

### 6. Memory Security

**Zeroization**:

All sensitive data must be zeroized after use:

```rust
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(ZeroizeOnDrop)]
struct MasterKey([u8; 32]);

impl Drop for MasterKey {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

// Usage
let mut master_key = derive_master_key(...)?;
// ... use master_key ...
// Automatically zeroized when dropped
```

**Note**: Bitwarden SDK types (MasterKey, UserKey) already implement zeroization.

---

## Data Models

### 1. New Models to Add

**Location**: `crates/bw-core/src/models/auth/` (new module)

**Login Result**:

```rust
use secrecy::Secret;

pub struct LoginResult {
    pub user_id: String,
    pub email: String,
    pub session_key: String, // Base64-encoded for export
}
```

**Unlock Result**:

```rust
pub struct UnlockResult {
    pub session_key: String, // Base64-encoded for export
}
```

**Two-Factor Data**:

```rust
#[derive(Debug, Clone)]
pub struct TwoFactorData {
    pub token: String,
    pub provider: u8,
    pub remember: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TwoFactorMethod {
    Authenticator = 0,
    Email = 1,
    Duo = 2,
    YubiKey = 3,
    U2F = 4,
    WebAuthn = 7,
}

impl TwoFactorMethod {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Authenticator => "Authenticator app",
            Self::Email => "Email",
            Self::Duo => "Duo",
            Self::YubiKey => "YubiKey",
            Self::U2F => "FIDO U2F",
            Self::WebAuthn => "FIDO2 WebAuthn",
        }
    }

    pub fn to_provider_code(&self) -> u8 {
        *self as u8
    }
}
```

**Device Information**:

```rust
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub device_type: u8,        // 8 = CLI
    pub device_name: String,    // "rust-cli"
    pub device_identifier: Uuid, // Persistent device ID
}

impl DeviceInfo {
    pub fn new() -> Self {
        // Load or generate device ID (persist to storage)
        let device_id = Self::load_or_generate_device_id();

        Self {
            device_type: 8,
            device_name: "rust-cli".to_string(),
            device_identifier: device_id,
        }
    }

    fn load_or_generate_device_id() -> Uuid {
        // Check storage for existing device ID
        // If not found, generate and persist
        Uuid::new_v4()
    }
}
```

### 2. Storage Keys

**Keys Used**:

```rust
// Encrypted keys (use set_secure/get_secure)
const KEY_ACCESS_TOKEN: &str = "accessToken";
const KEY_REFRESH_TOKEN: &str = "refreshToken";
const KEY_USER_KEY: &str = "userKey"; // Encrypted user key (EncString)

// Plain keys (use set/get)
const KEY_USER_PROFILE: &str = "userProfile"; // JSON: UserProfile
const KEY_KDF_CONFIG: &str = "kdfConfig";     // JSON: KdfConfig
const KEY_DEVICE_ID: &str = "deviceId";       // String: UUID
```

**Storage Layout After Login**:

```json
{
  "__PROTECTED__accessToken": "encrypted_base64...",
  "__PROTECTED__refreshToken": "encrypted_base64...",
  "__PROTECTED__userKey": "encrypted_base64...",
  "userProfile": {
    "id": "user-uuid",
    "email": "user@example.com",
    "name": "User Name",
    "emailVerified": true,
    "premium": false
  },
  "kdfConfig": {
    "kdf": 1,
    "kdfIterations": 3,
    "kdfMemory": 64,
    "kdfParallelism": 4
  },
  "deviceId": "device-uuid",
  "environmentUrls": { ... }
}
```

---

## Command Implementations

### 1. Login Command (Password)

**Flow Diagram**:

```
User: bw login
    │
    ├─→ Prompt: Email? → user@example.com
    ├─→ Prompt: Password? → ********
    │
    ├─→ API: POST /identity/accounts/prelogin
    │       Request: { email }
    │       Response: { kdf, kdfIterations, ... }
    │
    ├─→ SDK: Derive master key
    │       derive_master_key_pbkdf2(password, email, iterations)
    │       [Progress: "Deriving master key..."]
    │       Result: MasterKey (32 bytes)
    │
    ├─→ SDK: Hash password for auth
    │       hash_password(password, master_key, 1)
    │       Result: Base64 hashed password
    │
    ├─→ API: POST /identity/connect/token
    │       Request: { grant_type: "password", username, password, ... }
    │       Response: { access_token, refresh_token, Key, ... }
    │   │
    │   ├─→ If 2FA required (error "Two factor required"):
    │   │   ├─→ Prompt: Select method? → Authenticator
    │   │   ├─→ Prompt: Code? → 123456
    │   │   └─→ Retry with twoFactorToken and twoFactorProvider
    │   │
    │   └─→ Success: Tokens received
    │
    ├─→ SDK: Decrypt user key
    │       decrypt_enc_string(response.Key, master_key)
    │       Result: UserKey (64 bytes)
    │
    ├─→ Generate session key
    │       generate_session_key() → SessionKey (64 bytes)
    │
    ├─→ API: GET /api/accounts/profile
    │       (with Bearer access_token)
    │       Response: { id, email, name, ... }
    │
    ├─→ Storage: Save auth state
    │       set_secure("accessToken", access_token)
    │       set_secure("refreshToken", refresh_token)
    │       set_secure("userKey", encrypted_user_key)
    │       set("userProfile", profile)
    │       set("kdfConfig", kdf_config)
    │
    └─→ Output: "Logged in as user@example.com"
                "export BW_SESSION=\"base64_session_key\""
```

**Implementation Skeleton**:

```rust
// crates/bw-cli/src/commands/auth/login.rs
pub async fn execute_password_login(
    cmd: LoginPasswordCommand,
    global_args: &GlobalArgs,
) -> Result<Response> {
    // 1. Initialize services
    let container = ServiceContainer::new(None, None, None, None)?;
    let auth_service = AuthService::new(
        container.storage(),
        container.api_client(),
        container.sdk(),
    );

    // 2. Gather inputs
    let email = get_email_input(cmd.email, global_args)?;
    let password = get_password_input(cmd.password, global_args)?;
    let two_factor = if cmd.code.is_some() {
        Some(TwoFactorData {
            token: cmd.code.unwrap(),
            provider: cmd.method.unwrap_or(0),
            remember: false,
        })
    } else {
        None
    };

    // 3. Execute login
    let result = auth_service
        .login_with_password(&email, password, two_factor)
        .await?;

    // 4. Format output
    Ok(Response::success(format!(
        "You are logged in!\n\n\
         To unlock your vault, set your session key to the BW_SESSION environment variable. \
         ex:\n\
         $ export BW_SESSION=\"{}\"\n\
         > $env:BW_SESSION=\"{}\"\n\n\
         You can also pass the session key to any command with the --session option. \
         ex:\n\
         $ bw list items --session {}",
        result.session_key,
        result.session_key,
        result.session_key
    )))
}
```

### 2. Unlock Command

**Flow Diagram**:

```
User: bw unlock
    │
    ├─→ Check: Logged in?
    │   └─→ If no: Error "You are not logged in"
    │
    ├─→ Prompt: Password? → ********
    │
    ├─→ Storage: Load KDF config
    │       get::<KdfConfig>("kdfConfig")
    │
    ├─→ SDK: Derive master key
    │       derive_master_key_pbkdf2(password, email, kdf)
    │       [Progress: "Deriving master key..."]
    │
    ├─→ Storage: Load encrypted user key
    │       get_secure("userKey")
    │
    ├─→ SDK: Decrypt user key
    │       decrypt_enc_string(encrypted_key, master_key)
    │       Result: UserKey (64 bytes)
    │   │
    │   └─→ If decrypt fails: Error "Invalid master password"
    │
    ├─→ Generate session key
    │       generate_session_key() → SessionKey (64 bytes)
    │
    ├─→ Storage: Update session (optional - store for convenience)
    │       set("lastUnlock", timestamp)
    │
    └─→ Output: "Your vault is unlocked!"
                "export BW_SESSION=\"base64_session_key\""
```

**Implementation**:

```rust
pub async fn execute_unlock(
    cmd: UnlockCommand,
    global_args: &GlobalArgs,
) -> Result<Response> {
    let container = ServiceContainer::new(None, None, None, None)?;
    let auth_service = AuthService::new(
        container.storage(),
        container.api_client(),
        container.sdk(),
    );

    // Check if logged in
    let storage = container.storage();
    let profile: Option<UserProfile> = storage.get("userProfile")?;
    if profile.is_none() {
        anyhow::bail!("You are not logged in. Run 'bw login' to authenticate.");
    }

    // Get password
    let password = get_password_input(cmd.password, global_args)?;

    // Unlock
    let result = auth_service.unlock(password).await?;

    // Format output
    Ok(Response::success(format!(
        "Your vault is unlocked!\n\n\
         export BW_SESSION=\"{}\"",
        result.session_key
    )))
}
```

### 3. Lock Command

**Flow**:

```
User: bw lock
    │
    ├─→ Check: Logged in?
    │   └─→ If no: Error "You are not logged in"
    │
    ├─→ Storage: Remove user key
    │       remove_secure("userKey")
    │
    ├─→ Memory: Clear BW_SESSION (if set in memory)
    │
    └─→ Output: "Your vault is locked."
```

**Implementation**:

```rust
pub async fn execute_lock(
    _cmd: LockCommand,
    _global_args: &GlobalArgs,
) -> Result<Response> {
    let container = ServiceContainer::new(None, None, None, None)?;
    let auth_service = AuthService::new(
        container.storage(),
        container.api_client(),
        container.sdk(),
    );

    auth_service.lock().await?;

    Ok(Response::success("Your vault is locked."))
}
```

### 4. Logout Command

**Flow**:

```
User: bw logout
    │
    ├─→ Prompt: "Are you sure you want to log out? [y/N]"
    │   └─→ If no: Abort
    │
    ├─→ Storage: Clear all auth data
    │       remove_secure("accessToken")
    │       remove_secure("refreshToken")
    │       remove_secure("userKey")
    │       remove("userProfile")
    │       remove("kdfConfig")
    │
    └─→ Output: "You have been logged out."
```

**Implementation**:

```rust
pub async fn execute_logout(
    _cmd: LogoutCommand,
    global_args: &GlobalArgs,
) -> Result<Response> {
    // Confirmation prompt (unless --force or --nointeraction)
    if !global_args.nointeraction {
        let confirmed = prompts::prompt_confirmation(
            "Are you sure you want to log out?"
        )?;
        if !confirmed {
            return Ok(Response::error("Logout cancelled"));
        }
    }

    let container = ServiceContainer::new(None, None, None, None)?;
    let auth_service = AuthService::new(
        container.storage(),
        container.api_client(),
        container.sdk(),
    );

    auth_service.logout().await?;

    Ok(Response::success("You have been logged out."))
}
```

### 5. API Key Login

**Flow**:

```
User: bw login --apikey
    │
    ├─→ Prompt: Client ID? → user.xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
    ├─→ Prompt: Client Secret? → ********
    │
    ├─→ API: POST /identity/connect/token
    │       Request: { grant_type: "client_credentials", client_id, client_secret, ... }
    │       Response: { access_token, refresh_token }
    │
    ├─→ Generate session key (for consistency)
    │       generate_session_key() → SessionKey
    │
    ├─→ API: GET /api/accounts/profile
    │       Response: { id, email, ... }
    │
    ├─→ Storage: Save tokens and profile
    │       Note: No user key or KDF config (API key flow)
    │
    └─→ Output: "Logged in with API key!"
                "export BW_SESSION=\"...\""
```

**Note**: API key authentication does NOT require master password or KDF. The user key is not available, so vault item decryption may be limited.

---

## Error Handling Strategy

### 1. Error Type Hierarchy

**Location**: `crates/bw-core/src/services/auth/errors.rs`

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Invalid credentials: {message}")]
    InvalidCredentials { message: String },

    #[error("Two-factor authentication required")]
    TwoFactorRequired {
        available_methods: Vec<TwoFactorMethod>,
    },

    #[error("Invalid two-factor code")]
    InvalidTwoFactorCode,

    #[error("Not logged in")]
    NotLoggedIn,

    #[error("Master password incorrect")]
    InvalidPassword,

    #[error("KDF configuration error: {message}")]
    KdfError { message: String },

    #[error("Crypto operation failed: {message}")]
    CryptoError { message: String },

    #[error("Storage error: {0}")]
    Storage(#[from] crate::services::storage::StorageError),

    #[error("API error: {0}")]
    Api(#[from] crate::services::api::ApiError),

    #[error("SDK error: {0}")]
    Sdk(String), // Wrap SDK errors
}
```

### 2. User-Facing Error Messages

**Mapping Strategy**:

Each internal error should map to a user-friendly message with actionable hints:

```rust
impl AuthError {
    pub fn user_message(&self) -> String {
        match self {
            Self::InvalidCredentials { message } => {
                format!("Login failed: {}\n\nPlease check your email and password.", message)
            }
            Self::TwoFactorRequired { .. } => {
                "Two-factor authentication is required for your account.".to_string()
            }
            Self::InvalidTwoFactorCode => {
                "Invalid two-factor code. Please try again.".to_string()
            }
            Self::NotLoggedIn => {
                "You are not logged in.\n\nRun 'bw login' to authenticate.".to_string()
            }
            Self::InvalidPassword => {
                "Invalid master password.\n\nPlease try again or run 'bw login' if you've forgotten your password.".to_string()
            }
            Self::KdfError { message } => {
                format!("Key derivation error: {}\n\nThis may indicate a server issue. Please try again.", message)
            }
            Self::CryptoError { message } => {
                format!("Encryption error: {}\n\nThis may indicate corrupted data. Try logging out and back in.", message)
            }
            _ => format!("{}", self),
        }
    }
}
```

### 3. Error Recovery Strategies

**Network Errors**:
- Retry with exponential backoff (handled by API client)
- Clear error message: "Cannot connect to server. Check your internet connection."

**Invalid Credentials**:
- Do NOT retry automatically (security: prevent brute force)
- Clear message: "Invalid email or password"
- Suggest: Check email spelling, password correctness

**Invalid 2FA Code**:
- Allow user to retry immediately (prompt again)
- After 3 failed attempts: Error and require re-login
- Server may implement rate limiting

**Expired Session**:
- API client auto-refreshes tokens
- If refresh fails: "Session expired. Please run 'bw login' again."

**Corrupted Storage**:
- Detect via deserialization errors
- Suggest: Run 'bw logout --force' to clear corrupted data

---

## Security Architecture

### 1. Sensitive Data Handling

**Never Store**:
- ❌ Master password (in any form)
- ❌ Decrypted master key
- ❌ Decrypted user key

**Store Encrypted** (with BW_SESSION):
- ✅ Access token
- ✅ Refresh token
- ✅ Encrypted user key (EncString from server)

**Store Plain**:
- ✅ User profile (non-sensitive)
- ✅ KDF configuration (non-sensitive)
- ✅ Device ID (non-sensitive)

### 2. Memory Zeroization

**Zeroize Immediately After Use**:

```rust
// Master key - derive, use, zeroize
{
    let master_key = derive_master_key(...)?;
    let user_key = decrypt_user_key(..., &master_key)?;
    // master_key is zeroized when dropped (via ZeroizeOnDrop)
}

// Password - use once, zeroize
{
    let password = get_password_input(...)?;
    let master_key = derive_master_key(&password, ...)?;
    // password is zeroized when dropped (secrecy::Secret)
}

// Session key - use, export, zeroize
{
    let session_key = generate_session_key();
    let exported = session_key.to_base64();
    // session_key is zeroized when dropped
}
```

### 3. Constant-Time Operations

**Password Verification**:

When validating password during unlock, use constant-time comparison to prevent timing attacks:

```rust
use subtle::ConstantTimeEq;

// After deriving master key, attempt user key decryption
// Decryption failure indicates wrong password
match decrypt_user_key(encrypted_key, &master_key) {
    Ok(user_key) => Ok(user_key),
    Err(_) => Err(AuthError::InvalidPassword),
}
```

**Note**: Bitwarden SDK's decrypt functions already use constant-time operations internally.

### 4. File Permissions

**Storage File** (`~/.config/Bitwarden CLI/data.json`):

- Unix: `0600` (owner read/write only)
- Windows: User-only access (via Windows ACLs)

**Validation**:

```rust
#[cfg(unix)]
fn validate_file_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let metadata = std::fs::metadata(path)?;
    let mode = metadata.permissions().mode();

    // Check for world-readable (0o004) or group-readable (0o040)
    if mode & 0o044 != 0 {
        eprintln!(
            "Warning: Storage file has insecure permissions (mode: {:o}). \
             Run: chmod 600 {}",
            mode, path.display()
        );
    }

    Ok(())
}
```

### 5. Environment Variable Security

**BW_SESSION**:

- Generated per login/unlock session
- Base64-encoded 64 bytes (512 bits)
- Should be treated as secret (don't log, don't print accidentally)
- User's responsibility to protect in shell environment

**Warnings**:

```rust
// When --session is provided on command line (visible in ps)
if global_args.session.is_some() {
    eprintln!(
        "Warning: Passing --session on the command line may expose your session key. \
         Consider using the BW_SESSION environment variable instead."
    );
}
```

---

## Testing Strategy

### 1. Unit Tests

**Test Coverage Requirements**:
- ✅ KDF parameter parsing
- ✅ Session key generation and encoding
- ✅ Error handling and error messages
- ✅ Input validation (email, password, 2FA code)
- ✅ Storage operations (mock storage)
- ✅ API request building (not actual HTTP)

**Example Unit Tests**:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_key_generation() {
        let key1 = SessionKey::generate();
        let key2 = SessionKey::generate();

        // Keys should be unique
        assert_ne!(key1.to_base64(), key2.to_base64());

        // Keys should be 64 bytes when decoded
        let decoded = SessionKey::from_base64(&key1.to_base64()).unwrap();
        assert_eq!(decoded.to_base64(), key1.to_base64());
    }

    #[test]
    fn test_kdf_config_parsing() {
        let json = r#"{"kdf":1,"kdfIterations":3,"kdfMemory":64,"kdfParallelism":4}"#;
        let config: KdfConfig = serde_json::from_str(json).unwrap();

        assert_eq!(config.kdf, KdfType::Argon2id);
        assert_eq!(config.kdf_iterations, Some(3));
        assert_eq!(config.kdf_memory, Some(64));
        assert_eq!(config.kdf_parallelism, Some(4));
    }

    #[tokio::test]
    async fn test_login_flow_with_mock_storage() {
        let storage = Arc::new(MockStorage::new());
        let api_client = Arc::new(MockApiClient::new());
        let sdk_client = Arc::new(MockSdkClient::new());

        let auth_service = AuthService::new(storage, api_client, sdk_client);

        let result = auth_service
            .login_with_password("test@example.com", Secret::new("password".into()), None)
            .await;

        assert!(result.is_ok());
    }
}
```

### 2. Integration Tests

**Test Coverage**:
- ✅ Full login flow with real Bitwarden account
- ✅ 2FA flow with TOTP authenticator
- ✅ Unlock after lock
- ✅ Token refresh on 401
- ✅ Logout clears all data
- ✅ API key login

**Test Setup**:

```rust
// tests/integration/auth_tests.rs
use std::env;

#[tokio::test]
#[ignore] // Run with: cargo test --ignored
async fn test_full_login_flow() {
    let email = env::var("TEST_BW_EMAIL").expect("TEST_BW_EMAIL not set");
    let password = env::var("TEST_BW_PASSWORD").expect("TEST_BW_PASSWORD not set");

    // Use temporary storage for test
    let temp_dir = tempfile::tempdir().unwrap();
    let container = ServiceContainer::new(None, None, Some(temp_dir.path().into()), None)
        .unwrap();

    let auth_service = AuthService::new(
        container.storage(),
        container.api_client(),
        container.sdk(),
    );

    // Test login
    let result = auth_service
        .login_with_password(&email, Secret::new(password.clone()), None)
        .await
        .expect("Login should succeed");

    assert!(!result.session_key.is_empty());
    assert_eq!(result.email, email);

    // Test lock
    auth_service.lock().await.expect("Lock should succeed");

    // Test unlock
    let unlock_result = auth_service
        .unlock(Secret::new(password))
        .await
        .expect("Unlock should succeed");

    assert!(!unlock_result.session_key.is_empty());

    // Test logout
    auth_service.logout().await.expect("Logout should succeed");

    // Verify storage is cleared
    let profile: Option<UserProfile> = container.storage().get("userProfile").unwrap();
    assert!(profile.is_none());
}
```

### 3. TypeScript CLI Compatibility Tests

**Test Session Key Format**:

```bash
# Generate session key with Rust CLI
rust_session=$(./target/release/bw login --check | grep BW_SESSION | cut -d'"' -f2)

# Test that TypeScript CLI accepts it
export BW_SESSION="$rust_session"
bw sync  # TypeScript CLI command

# Verify success
if [ $? -eq 0 ]; then
    echo "✅ Session key compatible with TypeScript CLI"
else
    echo "❌ Session key NOT compatible"
fi
```

### 4. Security Tests

**Test Zeroization**:

```rust
#[test]
fn test_password_zeroized_after_use() {
    let password = Secret::new("test_password".to_string());
    let ptr = password.expose_secret().as_ptr();

    drop(password);

    // Note: This test is conceptual - actual memory inspection is platform-specific
    // In practice, rely on secrecy crate's implementation
}
```

**Test File Permissions**:

```rust
#[test]
#[cfg(unix)]
fn test_storage_file_permissions() {
    let temp_dir = tempfile::tempdir().unwrap();
    let storage = JsonFileStorage::new(Some(temp_dir.path().into())).unwrap();

    storage.set("test", &"value").unwrap();
    storage.flush().unwrap();

    let file_path = temp_dir.path().join("data.json");
    let metadata = std::fs::metadata(&file_path).unwrap();
    let mode = metadata.permissions().mode();

    // Should be 0600 (owner read/write only)
    assert_eq!(mode & 0o777, 0o600);
}
```

---

## Implementation Phases

### Phase 1: Foundation (MVP Core) - 5-7 days

**Goal**: Basic password authentication and session management

**Tasks**:

1. **SDK Integration** (Day 1-2)
   - Uncomment SDK dependencies in Cargo.toml
   - Replace mock SDK client with real implementation
   - Add SDK wrapper methods for crypto operations
   - Verify SDK builds and basic crypto functions work
   - **Deliverable**: `create_sdk_client()` returns real SDK client

2. **AuthService Implementation** (Day 2-3)
   - Create `crates/bw-core/src/services/auth/mod.rs`
   - Implement `login_with_password()` flow
   - Implement `derive_master_key()` with progress indicator
   - Implement `decrypt_user_key()`
   - **Deliverable**: Working AuthService with password login

3. **SessionManager** (Day 3)
   - Create `session_manager.rs`
   - Implement session key generation
   - Implement BW_SESSION encoding/decoding
   - **Deliverable**: Session key generation and export working

4. **Command Implementations** (Day 4-5)
   - Implement `execute_password_login()`
   - Implement `execute_unlock()`
   - Implement `execute_lock()`
   - Implement `execute_logout()`
   - Add interactive prompts (dialoguer)
   - **Deliverable**: All four basic commands functional

5. **Testing and Bug Fixes** (Day 5-7)
   - Write unit tests for auth logic
   - Integration test with real Bitwarden account
   - Test TypeScript CLI compatibility
   - Fix bugs and edge cases
   - **Deliverable**: MVP passes all tests

**Acceptance Criteria**:
- [ ] User can log in with email/password
- [ ] User can unlock vault after lock
- [ ] User can lock vault
- [ ] User can logout
- [ ] Session key format matches TypeScript CLI
- [ ] All sensitive data is zeroized
- [ ] Storage is encrypted with BW_SESSION

---

### Phase 2: Extended Authentication - 3-4 days

**Goal**: API key, 2FA, password input options

**Tasks**:

1. **API Key Authentication** (Day 1)
   - Implement `login_with_api_key()` in AuthService
   - Add API key prompts
   - Handle keyless session (no user key)
   - **Deliverable**: API key login working

2. **Two-Factor Authentication** (Day 2-3)
   - Implement 2FA error detection
   - Add 2FA method selection prompt
   - Add 2FA code prompt
   - Handle TOTP (authenticator) method
   - Handle email code method
   - **Deliverable**: 2FA login working for TOTP and email

3. **Password Input Options** (Day 3)
   - Add `--passwordenv` flag support
   - Add `--passwordfile` flag support
   - Add `--check` flag support
   - **Deliverable**: All password input options working

4. **Testing and Polish** (Day 4)
   - Test API key flow
   - Test 2FA flow (TOTP)
   - Test password input options
   - User feedback and error messages
   - **Deliverable**: Phase 2 complete and tested

**Acceptance Criteria**:
- [ ] API key login works
- [ ] 2FA (TOTP and email) works
- [ ] `--passwordenv` works
- [ ] `--passwordfile` works
- [ ] `--check` validates without persisting

---

### Phase 3: SSO and Advanced Features (Post-MVP) - 4-5 days

**Scope**: Defer to post-MVP unless required

**Features**:
- SSO browser-based login
- YubiKey hardware token support
- Remember device for 2FA
- Login history tracking

**Recommendation**: Evaluate complexity and user demand before implementing. SSO in CLI is complex and used by minority of users.

---

## Open Questions & Decisions

### Critical Decisions Made

**Q1: SSO Implementation**
- **Decision**: ✅ **DEFER TO POST-MVP**
- **Rationale**: High complexity, low usage, can be added later without breaking changes
- **Impact**: Reduces Phase 1/2 scope by ~4-5 days

**Q2: Interactive Prompt Library**
- **Decision**: ✅ **Use `dialoguer` crate**
- **Rationale**: Battle-tested, cross-platform, good UX, actively maintained
- **Alternative Considered**: `inquire` (less mature)

**Q3: Progress Indicator**
- **Decision**: ✅ **Use `indicatif` for spinners**
- **Rationale**: Standard library for progress indication, works well in terminals

**Q4: Login Auto-Unlocks**
- **Decision**: ✅ **Yes, login automatically unlocks vault**
- **Rationale**: Matches TypeScript CLI behavior, better UX

**Q5: Session State Management**
- **Decision**: ✅ **Create dedicated SessionManager service**
- **Rationale**: Centralizes session logic, cleaner architecture, testable

---

### Questions for Implementer

**Q6: Device ID Persistence**
- **Question**: Should device ID be persistent across sessions or regenerated?
- **Current Approach**: Generate new UUID per login (matches TypeScript CLI)
- **Alternative**: Store device ID in storage for consistency
- **Impact**: Device management in web vault shows multiple "rust-cli" devices vs single device
- **Recommendation**: Use persistent device ID, store in `deviceId` key

**Q7: 2FA Retry Limit**
- **Question**: How many 2FA retry attempts before forcing re-login?
- **Options**: Unlimited (user Ctrl+C to abort), 3 attempts, 5 attempts
- **Recommendation**: 3 attempts, then error and require fresh login

**Q8: Password from File - Permissions Check**
- **Question**: Should CLI error if password file is world-readable?
- **Options**: Error, warning, or silent
- **Recommendation**: Warning (eprintln!), don't block usage

**Q9: Email Validation**
- **Question**: How strict should email validation be?
- **Options**: Regex (strict), contains @ (loose), no validation (server validates)
- **Recommendation**: Loose validation (contains @ and .), server will validate

**Q10: KDF Progress Indicator Style**
- **Question**: Spinner, progress bar, or text message?
- **Recommendation**: Spinner with message: "Deriving master key... (this may take a few seconds)"

---

## Implementation Checklist

### Prerequisites
- [ ] SDK repository cloned at `../sdk/`
- [ ] Test Bitwarden account created (with 2FA enabled)
- [ ] Environment variables set for integration tests
- [ ] Dependencies added to Cargo.toml (dialoguer, indicatif)

### Phase 1 Tasks
- [ ] SDK integration complete
- [ ] AuthService module created
- [ ] SessionManager implemented
- [ ] Login command implemented
- [ ] Unlock command implemented
- [ ] Lock command implemented
- [ ] Logout command implemented
- [ ] Interactive prompts working
- [ ] Unit tests passing
- [ ] Integration tests passing
- [ ] TypeScript CLI compatibility validated

### Phase 2 Tasks
- [ ] API key login implemented
- [ ] 2FA error detection working
- [ ] 2FA method selection prompt
- [ ] TOTP authentication working
- [ ] Email code authentication working
- [ ] `--passwordenv` flag implemented
- [ ] `--passwordfile` flag implemented
- [ ] `--check` flag implemented
- [ ] All tests passing

### Quality Gates
- [ ] `cargo fmt` passes
- [ ] `cargo clippy` has no warnings
- [ ] `cargo test` passes (all unit tests)
- [ ] `cargo test --ignored` passes (integration tests)
- [ ] Security review completed
- [ ] Documentation updated

---

## Success Metrics

**Functional**:
- ✅ All authentication flows work (password, API key, 2FA)
- ✅ Session management working (lock/unlock/logout)
- ✅ TypeScript CLI session key compatibility

**Security**:
- ✅ No master password stored
- ✅ All sensitive data zeroized
- ✅ Encrypted storage working
- ✅ No security vulnerabilities in code review

**Quality**:
- ✅ >80% code coverage in unit tests
- ✅ Zero clippy warnings
- ✅ All integration tests pass

**User Experience**:
- ✅ Clear error messages
- ✅ Helpful prompts
- ✅ Progress indicators for slow operations
- ✅ Consistent with TypeScript CLI behavior

---

## References

### Bitwarden Documentation
- [Security White Paper](https://bitwarden.com/help/bitwarden-security-white-paper/)
- [KDF Algorithms](https://bitwarden.com/help/kdf-algorithms/)
- [CLI Documentation](https://bitwarden.com/help/cli/)

### TypeScript CLI Source (Reference Implementation)
- `apps/cli/src/auth/commands/login.command.ts`
- `apps/cli/src/auth/commands/unlock.command.ts`
- `libs/common/src/platform/services/crypto.service.ts`

### Rust Libraries
- [dialoguer](https://docs.rs/dialoguer/) - Interactive prompts
- [indicatif](https://docs.rs/indicatif/) - Progress indicators
- [secrecy](https://docs.rs/secrecy/) - Secret handling
- [zeroize](https://docs.rs/zeroize/) - Memory zeroization

---

**Status**: READY_FOR_IMPLEMENTATION

**Next Agent**: Implementer

**Estimated Effort**: 8-12 days (Phase 1 + Phase 2)

**Blocking Issues**: None

**Final Notes**:
- SDK integration must be first task (critical dependency)
- Security review is mandatory before production use
- Phase 3 (SSO) is deferred - reassess based on user demand
- TypeScript CLI compatibility must be validated throughout implementation
