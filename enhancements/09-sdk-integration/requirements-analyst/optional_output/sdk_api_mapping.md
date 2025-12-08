# SDK API Mapping Reference

This document provides detailed mapping between CLI mock implementations and SDK equivalents.

## 1. Client Initialization

### Current Mock (`sdk.rs`)
```rust
pub struct Client {
    api_url: String,
    identity_url: String,
}

pub fn create_sdk_client(api_url: Option<String>, identity_url: Option<String>) -> Result<Client>
```

### SDK Equivalent
```rust
use bitwarden_core::{Client, ClientSettings, DeviceType};

pub fn create_sdk_client(api_url: Option<String>, identity_url: Option<String>) -> Result<Client> {
    let settings = ClientSettings {
        api_url: api_url.unwrap_or_else(|| "https://api.bitwarden.com".to_string()),
        identity_url: identity_url.unwrap_or_else(|| "https://identity.bitwarden.com".to_string()),
        user_agent: "Bitwarden CLI".to_string(),
        device_type: DeviceType::LinuxCLI, // Platform-specific
        bitwarden_client_version: Some(env!("CARGO_PKG_VERSION").to_string()),
    };

    Ok(Client::new(Some(settings)))
}
```

## 2. Master Key Derivation

### Current Mock (`mock_crypto.rs`)
```rust
pub fn derive_master_key(
    password: &Secret<String>,
    email: &str,
    kdf_config: &KdfConfig,
) -> Result<MasterKey>
```

### SDK Equivalent
```rust
use bitwarden_crypto::{MasterKey, Kdf};

pub fn derive_master_key(
    password: &str,
    email: &str,
    kdf: &Kdf,
) -> Result<MasterKey, CryptoError> {
    MasterKey::derive(password, email, kdf)
}
```

### KDF Configuration Conversion

```rust
impl From<&KdfConfig> for Kdf {
    fn from(config: &KdfConfig) -> Self {
        match config.kdf {
            KdfType::PBKDF2SHA256 => Kdf::PBKDF2 {
                iterations: NonZeroU32::new(
                    config.kdf_iterations.unwrap_or(600_000)
                ).expect("iterations must be > 0"),
            },
            KdfType::Argon2id => Kdf::Argon2id {
                iterations: NonZeroU32::new(
                    config.kdf_iterations.unwrap_or(3)
                ).expect("iterations must be > 0"),
                memory: NonZeroU32::new(
                    config.kdf_memory.unwrap_or(64)
                ).expect("memory must be > 0"),
                parallelism: NonZeroU32::new(
                    config.kdf_parallelism.unwrap_or(4)
                ).expect("parallelism must be > 0"),
            },
        }
    }
}
```

## 3. Password Hashing

### Current Mock
```rust
pub fn hash_password(password: &Secret<String>, master_key: &MasterKey) -> Result<String>
```

### SDK Equivalent
```rust
use bitwarden_crypto::HashPurpose;

let hash = master_key.derive_master_key_hash(
    password.expose_secret().as_bytes(),
    HashPurpose::ServerAuthorization
);
// hash is of type B64, convert to String
let hash_string = hash.to_string();
```

## 4. User Key Decryption

### Current Mock
```rust
pub fn decrypt_user_key(encrypted_key: &str, master_key: &MasterKey) -> Result<UserKey>
```

### SDK Equivalent
```rust
use bitwarden_crypto::{EncString, SymmetricCryptoKey};

// Parse the encrypted string
let enc_string: EncString = encrypted_key.parse()
    .map_err(|e| CryptoError::EncodingError(e))?;

// Decrypt using master key
let symmetric_key: SymmetricCryptoKey = master_key.decrypt_user_key(enc_string)?;

// Wrap in UserKey if needed
let user_key = UserKey::new(symmetric_key);
```

## 5. DeviceType Selection

```rust
use bitwarden_core::DeviceType;

fn get_device_type() -> DeviceType {
    #[cfg(target_os = "linux")]
    { DeviceType::LinuxCLI }

    #[cfg(target_os = "macos")]
    { DeviceType::MacOsCLI }

    #[cfg(target_os = "windows")]
    { DeviceType::WindowsCLI }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    { DeviceType::SDK }
}
```

## 6. Auth Client Access

```rust
use bitwarden_auth::{AuthClient, AuthClientExt};

// From a Client instance
let auth_client: AuthClient = client.auth_new();

// Access identity operations
let identity_client = auth_client.identity();

// Access registration operations (if needed)
let registration_client = auth_client.registration();
```

## 7. Error Type Mapping

### SDK Errors
- `bitwarden_crypto::CryptoError` - Cryptographic operation failures
- `bitwarden_crypto::EncodingError` - Base64/encoding issues

### Mapping to CLI Errors
```rust
impl From<CryptoError> for AuthError {
    fn from(e: CryptoError) -> Self {
        AuthError::CryptoError {
            message: e.to_string()
        }
    }
}
```

## 8. Test Vectors

### PBKDF2 Test Vector (from SDK tests)
```
Password: "asdfasdf"
Email: "test@bitwarden.com" (normalized to lowercase, trimmed)
KDF: PBKDF2 { iterations: 100_000 }
Expected Hash: "wmyadRMyBZOH7P/a/ucTCbSghKgdzDpPqUnu/DAVtSw="
```

### Argon2id Test Vector (from SDK tests)
```
Password: "asdfasdf"
Salt: "test_salt"
KDF: Argon2id { iterations: 4, memory: 32, parallelism: 2 }
Expected Hash: "PR6UjYmjmppTYcdyTiNbAhPJuQQOmynKbdEl1oyi/iQ="
```

## 9. Required Imports Summary

```rust
// bitwarden-core
use bitwarden_core::{Client, ClientSettings, DeviceType};

// bitwarden-crypto
use bitwarden_crypto::{
    MasterKey,
    UserKey,
    Kdf,
    HashPurpose,
    EncString,
    SymmetricCryptoKey,
    CryptoError,
};

// bitwarden-auth
use bitwarden_auth::{AuthClient, AuthClientExt};

// bitwarden-encoding (if needed directly)
use bitwarden_encoding::B64;
```

## 10. Feature Requirements

Ensure `Cargo.toml` has:
```toml
bitwarden-core = { workspace = true, features = ["internal"] }
```

The `internal` feature is required for:
- Full client functionality
- Test accounts (for testing)
- Security state management
