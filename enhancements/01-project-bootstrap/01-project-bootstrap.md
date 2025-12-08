---
slug: project-bootstrap
status: NEW
created: 2024-12-02
author: Migration Team
priority: critical
---

# Enhancement: CLI Rust Migration - Project Bootstrap

## Overview
**Goal:** Create the foundational Rust project structure for migrating the Bitwarden CLI from TypeScript to Rust, including workspace setup, CLI parsing, output formatting, and command stubs.

**User Story:**
As a developer, I want a properly configured Rust project with CLI parsing and command structure so that subsequent enhancements can implement actual command functionality.

## Context & Background
**Current State:**
- Bitwarden CLI exists as TypeScript/JavaScript in apps/cli directory
- Uses Commander.js for CLI parsing
- Uses Response class pattern for structured output
- Environment variables: BW_SESSION, BW_QUIET, BW_RESPONSE, BW_RAW, BW_PRETTY, BW_NOINTERACTION, BW_CLEANEXIT
- New Rust project at `bwcli-rs/`

**Technical Context:**
- Target platform: Cross-platform (Windows, macOS, Linux)
- This is enhancement 1 of 8 for the full migration
- Migration goals: single binary, smaller size (~8MB vs ~60MB), faster startup
- Direct integration with existing Rust SDK
- Must maintain CLI command structure compatibility

**Dependencies:**
- Rust stable toolchain
- Bitwarden SDK (already Rust)
- TypeScript CLI for reference (separate directory)

## Bitwarden SDK Integration

### Overview

The Bitwarden internal SDK (https://github.com/bitwarden/sdk) provides all cryptographic primitives and core business logic in Rust. The CLI migration **must** use this existing SDK rather than reimplementing crypto operations. The TypeScript CLI currently uses `@bitwarden/sdk-internal` (the WASM bindings); the Rust CLI will use the native crates directly.

> **⚠️ Important:** The password manager SDK interface is internal and unstable. Pin to specific versions and expect breaking changes. Coordinate with the SDK team for updates.

### SDK Crate Structure

The SDK is a Cargo workspace with these relevant crates:

| Crate | Purpose | Use In CLI |
|-------|---------|------------|
| `bitwarden-core` | Core functionality, client initialization | Service container setup |
| `bitwarden-crypto` | All cryptographic operations | Auth, vault encrypt/decrypt |
| `bitwarden-api-api` | Auto-generated API server bindings | API client (if not using SDK client) |
| `bitwarden-api-identity` | Auto-generated Identity server bindings | Authentication flows |

### Dependency Configuration

Add to `Cargo.toml` using path dependencies (for internal development) or git dependencies:

```toml
[dependencies]
# Option 1: Path dependency (recommended for internal development)
# Assumes SDK repo is cloned alongside CLI repo
bitwarden-core = { path = "../sdk/crates/bitwarden-core" }
bitwarden-crypto = { path = "../sdk/crates/bitwarden-crypto" }

# Option 2: Git dependency (for CI/external builds)
# bitwarden-core = { git = "https://github.com/bitwarden/sdk", branch = "main" }
# bitwarden-crypto = { git = "https://github.com/bitwarden/sdk", branch = "main" }

# Optional: If you need direct API bindings (SDK client may handle this)
# bitwarden-api-api = { path = "../sdk/crates/bitwarden-api-api" }
# bitwarden-api-identity = { path = "../sdk/crates/bitwarden-api-identity" }
```

### SDK Capabilities - What to Use

The SDK provides these operations that **must NOT be reimplemented**:

#### From `bitwarden-crypto`
| Operation | Description | Used In Enhancement |
|-----------|-------------|---------------------|
| Key derivation (PBKDF2) | Master password → master key | 04-authentication |
| Key derivation (Argon2id) | Master password → master key | 04-authentication |
| AES-256 encryption | Encrypt vault items | 06-vault-write |
| AES-256 decryption | Decrypt vault items | 05-vault-read |
| EncString parsing | Parse `type.iv.ct.mac` format | 05-vault-read, 06-vault-write |
| HMAC-SHA256 | MAC verification | All crypto operations |
| RSA operations | Org key exchange | 06-vault-write (org items) |
| Secure memory (zeroize) | Clear sensitive data | All operations |

#### From `bitwarden-core`
| Operation | Description | Used In Enhancement |
|-----------|-------------|---------------------|
| Client initialization | SDK client setup | 01-project-bootstrap |
| Auth flows | Login, 2FA, token management | 04-authentication |
| Vault operations | Cipher/folder/collection handling | 05-vault-read, 06-vault-write |
| Sync | Full vault synchronization | 05-vault-read |
| TOTP generation | Generate TOTP codes | 05-vault-read (get totp) |
| Password generator | Generate passwords/passphrases | 07-tools-commands |
| Send operations | Create/decrypt Sends | 07-tools-commands |
| Export | Vault export functionality | 08-import-export |

### SDK Client Initialization

```rust
// src/services/sdk.rs
use bitwarden_core::{Client, ClientSettings, DeviceType};
use std::sync::Arc;

/// Create the SDK client for all crypto and vault operations
pub fn create_sdk_client(
    api_url: Option<String>,
    identity_url: Option<String>,
) -> anyhow::Result<Client> {
    let settings = ClientSettings {
        identity_url: identity_url.unwrap_or_else(|| 
            "https://identity.bitwarden.com".to_string()
        ),
        api_url: api_url.unwrap_or_else(|| 
            "https://api.bitwarden.com".to_string()
        ),
        user_agent: format!("Bitwarden_CLI/{}", env!("CARGO_PKG_VERSION")),
        device_type: DeviceType::CLI,
    };
    
    Client::new(Some(settings))
}
```

### Integration Pattern

```rust
// src/services/container.rs
use bitwarden_core::Client;
use bitwarden_crypto::SymmetricCryptoKey;

pub struct ServiceContainer {
    /// SDK client - handles all crypto and most business logic
    sdk: Client,
    
    /// Storage for persisted state (tokens, cached data)
    storage: Box<dyn Storage>,
    
    /// HTTP client for any non-SDK API calls
    http: reqwest::Client,
}

impl ServiceContainer {
    pub fn new(settings: &AppSettings) -> anyhow::Result<Self> {
        let sdk = create_sdk_client(
            settings.api_url.clone(),
            settings.identity_url.clone(),
        )?;
        
        Ok(Self {
            sdk,
            storage: create_storage(settings)?,
            http: create_http_client(settings)?,
        })
    }
    
    /// Access SDK client for crypto/vault operations
    pub fn sdk(&self) -> &Client {
        &self.sdk
    }
    
    /// Example: Decrypt a cipher using SDK
    pub async fn decrypt_cipher(&self, cipher: &Cipher) -> anyhow::Result<CipherView> {
        // SDK handles all decryption internally
        self.sdk.vault().ciphers().decrypt(cipher).await
    }
}
```

### What NOT to Implement

These operations **must use the SDK** - do not reimplement:

| ❌ Do Not Implement | ✅ Use Instead |
|--------------------|----------------|
| AES-256-CBC/GCM encryption | `bitwarden_crypto::encrypt` |
| PBKDF2/Argon2id key derivation | `bitwarden_crypto::derive_key` |
| EncString parsing (`2.xxx\|yyy\|zzz`) | `bitwarden_crypto::EncString::from_str` |
| Master key stretching | SDK auth flows |
| TOTP code generation | `bitwarden_core` vault client |
| Password/passphrase generation | `bitwarden_core` generator |
| Send encryption/decryption | `bitwarden_core` send client |

### Allowed CLI-Specific Implementations

These supporting operations **can** be implemented in the CLI crate:

| ✅ Allowed | Crate to Use | Purpose |
|-----------|--------------|---------|
| BW_SESSION key generation | `rand` (OsRng) | Session token for env var |
| Base64 display formatting | `base64` | User-facing output |
| Zeroize wrappers | `zeroize` | Additional safety |
| JSON file storage | `serde_json` | Config/cache persistence |
| Platform paths | `directories` | XDG/AppData locations |

### Build Configuration

Ensure the workspace is configured to find SDK crates:

```toml
# .cargo/config.toml (if using path dependencies)
[env]
# Point to SDK repo location if needed
SDK_PATH = { value = "../sdk", relative = true }
```

### Testing with SDK

```rust
#[cfg(test)]
mod tests {
    use bitwarden_crypto::SymmetricCryptoKey;
    
    #[test]
    fn test_sdk_crypto_available() {
        // Verify SDK crypto is accessible
        let key = SymmetricCryptoKey::generate(rand::thread_rng());
        assert!(key.is_ok());
    }
}
```

### Version Compatibility

| CLI Version | SDK Commit/Version | Notes |
|-------------|--------------------|-------|
| 0.1.x | `main` branch | Initial development |

> **Note:** Since the SDK interface is unstable, document the specific SDK commit hash used for each CLI release. Consider using git submodules or a lock file to pin SDK versions.

### References

- SDK Repository: https://github.com/bitwarden/sdk
- SDK Internal API Docs: https://sdk-api-docs.bitwarden.com/bitwarden_core/index.html
- SDK Contributing Guide: https://contributing.bitwarden.com/getting-started/sdk/

## Requirements

### Functional Requirements
1. Cargo workspace structure with bw-cli and bw-core crates
2. CLI argument parsing using clap with derive macros
3. All command stubs (login, logout, lock, unlock, sync, list, get, create, edit, delete, etc.)
4. Response formatting system (JSON, pretty JSON, raw, quiet modes)
5. Global flags: --session, --quiet, --response, --raw, --pretty, --nointeraction, --cleanexit
6. Help text matching TypeScript CLI structure
7. Version command

### Non-Functional Requirements
- **Performance:** Fast compilation during development, binary size <5MB stripped
- **Memory:** Minimal allocation during CLI parsing
- **Reliability:** All commands compile and return stub responses
- **Compatibility:** Exact CLI interface compatibility with TypeScript version

### Must Have (MVP)
- [ ] Cargo workspace at bwcli-rs/ with workspace manifest
- [ ] Main binary crate (bw-cli) with src/main.rs
- [ ] Core library crate (bw-core) as placeholder
- [ ] clap-based CLI with derive macros for all commands
- [ ] Command structure: auth, vault, sync, tools, send, config, transfer
- [ ] Output formatting: Response type with JSON/raw/pretty/quiet modes
- [ ] All commands return "Not yet implemented" stubs
- [ ] Environment variable support for flags
- [ ] Version and help commands working
- [ ] README with build instructions

### Should Have (if time permits)
- [ ] Shell completion generation setup
- [ ] Basic CI/CD configuration
- [ ] Dockerfile/development container
- [ ] .gitignore configured properly

### Won't Have (out of scope)
- Actual command implementation (reason: later enhancements)
- Storage layer (reason: enhancement 2)
- API client (reason: enhancement 3)

## Open Questions

1. Should we use workspace structure or single crate initially?
2. What's the best way to organize command modules?
3. Should Response formatting be in main crate or core?
4. Do we need separate error types at this stage?
5. What logging framework to use (tracing vs env_logger)?

## Constraints & Limitations
**Technical Constraints:**
- Must maintain exact CLI command structure
- Cannot break existing scripts using the CLI
- Must support same environment variables
- Must compile on all target platforms

**Business/Timeline Constraints:**
- Blocking all other enhancement work
- Must be complete before enhancement 2
- Critical path item

## Success Criteria
**Definition of Done:**
- [ ] `cargo build --release` succeeds with no warnings
- [ ] `cargo test` passes
- [ ] `cargo clippy` passes with no warnings
- [ ] Binary size under 5MB stripped
- [ ] `./target/release/bw --help` displays comprehensive help
- [ ] `./target/release/bw --version` outputs version
- [ ] `./target/release/bw status --response` returns JSON stub
- [ ] All commands exist as stubs returning "Not yet implemented"
- [ ] `--quiet` flag suppresses output correctly
- [ ] `--response` flag returns JSON format
- [ ] `--pretty` flag formats JSON with indentation

**Acceptance Tests:**
1. Given `bw --help`, when invoked, then displays all command categories
2. Given `bw login --help`, when invoked, then shows login-specific help
3. Given `bw status --response`, when run, then returns JSON `{"success":false,"message":"Not yet implemented"}`
4. Given `BW_QUIET=true bw status`, when run, then produces no output
5. Given `bw --pretty status --response`, when run, then returns formatted JSON
6. Given any command, when invoked, then exits with appropriate code

## Security & Safety Considerations
- Don't log sensitive arguments (passwords, tokens, API keys)
- Sanitize error messages to avoid leaking data
- Use strong typing for sensitive data from start
- Plan for secure memory handling in future enhancements

## UI/UX Considerations (if applicable)
- Help text should match TypeScript CLI formatting
- Error messages should be clear and actionable
- Command grouping in help output for clarity
- Consistent formatting across all commands
- Response format matches existing CLI exactly

## Testing Strategy
**Unit Tests:**
- Test CLI argument parsing for each command
- Test flag combinations
- Test environment variable integration
- Test Response formatting variants
- Test help text generation

**Integration Tests:**
- Build and run on multiple platforms
- Test command invocation patterns
- Test all global flag combinations

**Manual Test Scenarios:**
1. Run each command with --help, compare with TypeScript output
2. Test global flags on various commands
3. Verify version output
4. Test with various environment variable combinations

## References & Research
- apps/cli/src/bw.ts (TypeScript entry point)
- apps/cli/src/program.ts (command registration)
- apps/cli/src/base-program.ts (processResponse logic)
- apps/cli/src/models/response.ts (Response class)
- clap documentation: https://docs.rs/clap/
- Rust CLI best practices

## Notes for PM Subagent
- Verify complete command list from TypeScript CLI
- Confirm no commands are being deprecated
- Ensure project structure supports future modularization
- Flag if enhancement scope is too large

## Notes for Architect Subagent
- Design command enum structure for extensibility
- Consider trait-based command execution pattern for later
- Plan Response type to support all output modes
- Separate CLI parsing from command execution
- Design for testability from the start
- Consider error type hierarchy early

## Notes for Implementer Subagent
- Use clap's derive macros for cleaner code
- Create separate modules for command definitions
- Keep Response formatting simple initially
- Use #[arg(env = "BW_SESSION")] for env var fallbacks
- Add comprehensive doc comments for public APIs
- Follow Rust naming conventions
- Use workspace to share dependencies

## Notes for Testing Subagent
- Compare help output with TypeScript CLI
- Test all command variations
- Verify environment variable behavior
- Test output format modes exhaustively
- Ensure error messages are user-friendly
- Test with actual shell invocations
- Cross-platform testing essential