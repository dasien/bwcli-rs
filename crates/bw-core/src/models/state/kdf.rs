use std::num::NonZeroU32;

use bitwarden_crypto::Kdf;
use serde::{Deserialize, Serialize};

/// Key Derivation Function configuration
///
/// Matches TypeScript CLI storage format:
/// - `kdfType`: 0 = PBKDF2-SHA256, 1 = Argon2id
/// - `iterations`: iteration count
/// - `memory`: Argon2 memory in MB (optional)
/// - `parallelism`: Argon2 parallelism (optional)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KdfConfig {
    /// KDF type: 0 = PBKDF2-SHA256, 1 = Argon2id
    pub kdf_type: KdfType,

    /// Iteration count (default: 600000 for PBKDF2, 3 for Argon2)
    #[serde(default)]
    pub iterations: Option<u32>,

    /// Argon2 memory in MB (default: 64)
    /// Used when kdf_type = Argon2id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory: Option<u32>,

    /// Argon2 parallelism (default: 4)
    /// Used when kdf_type = Argon2id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parallelism: Option<u32>,
}

/// KDF type enum
///
/// TypeScript CLI stores this as an integer (0 or 1)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum KdfType {
    PBKDF2SHA256 = 0,
    Argon2id = 1,
}

impl serde::Serialize for KdfType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u8(*self as u8)
    }
}

impl<'de> serde::Deserialize<'de> for KdfType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = u8::deserialize(deserializer)?;
        match value {
            0 => Ok(KdfType::PBKDF2SHA256),
            1 => Ok(KdfType::Argon2id),
            _ => Err(serde::de::Error::custom(format!(
                "unknown KDF type: {}",
                value
            ))),
        }
    }
}

/// Convert CLI KdfConfig to SDK Kdf type
impl TryFrom<&KdfConfig> for Kdf {
    type Error = anyhow::Error;

    fn try_from(config: &KdfConfig) -> Result<Self, Self::Error> {
        match config.kdf_type {
            KdfType::PBKDF2SHA256 => {
                let iterations = config.iterations.unwrap_or(600_000);
                Ok(Kdf::PBKDF2 {
                    iterations: NonZeroU32::new(iterations)
                        .ok_or_else(|| anyhow::anyhow!("KDF iterations must be > 0"))?,
                })
            }
            KdfType::Argon2id => {
                let iterations = config.iterations.unwrap_or(3);
                let memory = config.memory.unwrap_or(64);
                let parallelism = config.parallelism.unwrap_or(4);

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pbkdf2_conversion() {
        let config = KdfConfig {
            kdf_type: KdfType::PBKDF2SHA256,
            iterations: Some(600_000),
            memory: None,
            parallelism: None,
        };

        let kdf: Kdf = (&config).try_into().expect("Should convert PBKDF2");
        match kdf {
            Kdf::PBKDF2 { iterations } => {
                assert_eq!(iterations.get(), 600_000);
            }
            _ => panic!("Expected PBKDF2"),
        }
    }

    #[test]
    fn test_pbkdf2_default_iterations() {
        let config = KdfConfig {
            kdf_type: KdfType::PBKDF2SHA256,
            iterations: None,
            memory: None,
            parallelism: None,
        };

        let kdf: Kdf = (&config).try_into().expect("Should convert PBKDF2");
        match kdf {
            Kdf::PBKDF2 { iterations } => {
                assert_eq!(iterations.get(), 600_000);
            }
            _ => panic!("Expected PBKDF2"),
        }
    }

    #[test]
    fn test_argon2id_conversion() {
        let config = KdfConfig {
            kdf_type: KdfType::Argon2id,
            iterations: Some(4),
            memory: Some(64),
            parallelism: Some(4),
        };

        let kdf: Kdf = (&config).try_into().expect("Should convert Argon2id");
        match kdf {
            Kdf::Argon2id {
                iterations,
                memory,
                parallelism,
            } => {
                assert_eq!(iterations.get(), 4);
                assert_eq!(memory.get(), 64);
                assert_eq!(parallelism.get(), 4);
            }
            _ => panic!("Expected Argon2id"),
        }
    }

    #[test]
    fn test_argon2id_default_params() {
        let config = KdfConfig {
            kdf_type: KdfType::Argon2id,
            iterations: None,
            memory: None,
            parallelism: None,
        };

        let kdf: Kdf = (&config).try_into().expect("Should convert Argon2id");
        match kdf {
            Kdf::Argon2id {
                iterations,
                memory,
                parallelism,
            } => {
                assert_eq!(iterations.get(), 3);
                assert_eq!(memory.get(), 64);
                assert_eq!(parallelism.get(), 4);
            }
            _ => panic!("Expected Argon2id"),
        }
    }

    #[test]
    fn test_typescript_cli_format_deserialization() {
        // This is the exact format stored by TypeScript CLI
        let json = r#"{"iterations": 600000, "kdfType": 0}"#;
        let config: KdfConfig = serde_json::from_str(json).expect("Should deserialize");
        assert_eq!(config.kdf_type, KdfType::PBKDF2SHA256);
        assert_eq!(config.iterations, Some(600_000));
    }
}
