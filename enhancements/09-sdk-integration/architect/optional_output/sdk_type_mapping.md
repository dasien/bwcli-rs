# SDK Type Mapping Reference

This document provides quick reference for type mapping between CLI and SDK.

## Client Types

| CLI (Current) | SDK | Notes |
|---------------|-----|-------|
| `sdk::Client` (mock) | `bitwarden_core::Client` | Main SDK client |
| N/A | `bitwarden_core::ClientSettings` | Client configuration |
| N/A | `bitwarden_core::DeviceType` | Platform identifier |

## Crypto Types

| CLI (Current) | SDK | Notes |
|---------------|-----|-------|
| `mock_crypto::MasterKey` | `bitwarden_crypto::MasterKey` | Password-derived key |
| `mock_crypto::UserKey` | `bitwarden_crypto::SymmetricCryptoKey` | Vault encryption key |
| N/A | `bitwarden_crypto::UserKey` | Wrapper around SymmetricCryptoKey |
| `KdfConfig` | `bitwarden_crypto::Kdf` | KDF parameters |
| N/A | `bitwarden_crypto::HashPurpose` | Hash purpose enum |
| N/A | `bitwarden_crypto::EncString` | Encrypted string format |

## Error Types

| CLI | SDK | Mapping |
|-----|-----|---------|
| `AuthError::CryptoError` | `bitwarden_crypto::CryptoError` | General crypto errors |
| `AuthError::InvalidPassword` | `CryptoError::InvalidMac` | MAC verification failure |
| `AuthError::KdfError` | `CryptoError::InsufficientKdfParameters` | Invalid KDF params |

## Function Mapping

### Master Key Derivation

**Mock:**
```rust
mock_crypto::derive_master_key(
    password: &Secret<String>,
    email: &str,
    kdf_config: &KdfConfig,
) -> Result<MasterKey>
```

**SDK:**
```rust
MasterKey::derive(
    password: &str,
    email: &str,
    kdf: &Kdf,
) -> Result<MasterKey, CryptoError>
```

### Password Hashing

**Mock:**
```rust
mock_crypto::hash_password(
    password: &Secret<String>,
    master_key: &MasterKey,
) -> Result<String>
```

**SDK:**
```rust
master_key.derive_master_key_hash(
    password: &[u8],
    purpose: HashPurpose,
) -> B64

// Convert to string:
hash.to_string()
```

### User Key Decryption

**Mock:**
```rust
mock_crypto::decrypt_user_key(
    encrypted_key: &str,
    master_key: &MasterKey,
) -> Result<UserKey>
```

**SDK:**
```rust
// Parse EncString
let enc_string: EncString = encrypted_key.parse()?;

// Decrypt
let symmetric_key = master_key.decrypt_user_key(enc_string)?;
```

## KDF Conversion

```rust
// CLI KdfConfig
pub struct KdfConfig {
    pub kdf: KdfType,
    pub kdf_iterations: Option<u32>,
    pub kdf_memory: Option<u32>,
    pub kdf_parallelism: Option<u32>,
}

pub enum KdfType {
    PBKDF2SHA256 = 0,
    Argon2id = 1,
}

// SDK Kdf
pub enum Kdf {
    PBKDF2 {
        iterations: NonZeroU32,
    },
    Argon2id {
        iterations: NonZeroU32,
        memory: NonZeroU32,
        parallelism: NonZeroU32,
    },
}
```

### Conversion Implementation

```rust
impl TryFrom<&KdfConfig> for Kdf {
    type Error = anyhow::Error;

    fn try_from(config: &KdfConfig) -> Result<Self, Self::Error> {
        match config.kdf {
            KdfType::PBKDF2SHA256 => Ok(Kdf::PBKDF2 {
                iterations: NonZeroU32::new(config.kdf_iterations.unwrap_or(600_000))
                    .ok_or_else(|| anyhow::anyhow!("iterations must be > 0"))?,
            }),
            KdfType::Argon2id => Ok(Kdf::Argon2id {
                iterations: NonZeroU32::new(config.kdf_iterations.unwrap_or(3))
                    .ok_or_else(|| anyhow::anyhow!("iterations must be > 0"))?,
                memory: NonZeroU32::new(config.kdf_memory.unwrap_or(64))
                    .ok_or_else(|| anyhow::anyhow!("memory must be > 0"))?,
                parallelism: NonZeroU32::new(config.kdf_parallelism.unwrap_or(4))
                    .ok_or_else(|| anyhow::anyhow!("parallelism must be > 0"))?,
            }),
        }
    }
}
```

## DeviceType Mapping

```rust
fn get_device_type() -> DeviceType {
    #[cfg(target_os = "linux")]
    { DeviceType::LinuxCLI }       // = 25

    #[cfg(target_os = "macos")]
    { DeviceType::MacOsCLI }       // = 24

    #[cfg(target_os = "windows")]
    { DeviceType::WindowsCLI }     // = 23

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    { DeviceType::SDK }            // = 21
}
```

## Import Summary

```rust
// bitwarden-core
use bitwarden_core::{Client, ClientSettings, DeviceType};

// bitwarden-crypto
use bitwarden_crypto::{
    CryptoError,
    EncString,
    HashPurpose,
    Kdf,
    MasterKey,
    SymmetricCryptoKey,
    UserKey,
};

// bitwarden-auth (for future use)
use bitwarden_auth::{AuthClient, AuthClientExt};
```
