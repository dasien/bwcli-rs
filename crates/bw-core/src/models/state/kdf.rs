use std::num::NonZeroU32;

use bitwarden_crypto::Kdf;
use serde::{Deserialize, Serialize};

/// Key Derivation Function configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KdfConfig {
    /// KDF type: 0 = PBKDF2-SHA256, 1 = Argon2id
    pub kdf: KdfType,

    /// PBKDF2 iterations (default: 600000)
    /// Used when kdf = PBKDF2
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kdf_iterations: Option<u32>,

    /// Argon2 memory in MB (default: 64)
    /// Used when kdf = Argon2id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kdf_memory: Option<u32>,

    /// Argon2 parallelism (default: 4)
    /// Used when kdf = Argon2id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kdf_parallelism: Option<u32>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[repr(u8)]
pub enum KdfType {
    #[serde(rename = "0")]
    PBKDF2SHA256 = 0,
    #[serde(rename = "1")]
    Argon2id = 1,
}

/// Convert CLI KdfConfig to SDK Kdf type
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pbkdf2_conversion() {
        let config = KdfConfig {
            kdf: KdfType::PBKDF2SHA256,
            kdf_iterations: Some(600_000),
            kdf_memory: None,
            kdf_parallelism: None,
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
            kdf: KdfType::PBKDF2SHA256,
            kdf_iterations: None,
            kdf_memory: None,
            kdf_parallelism: None,
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
            kdf: KdfType::Argon2id,
            kdf_iterations: Some(4),
            kdf_memory: Some(64),
            kdf_parallelism: Some(4),
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
            kdf: KdfType::Argon2id,
            kdf_iterations: None,
            kdf_memory: None,
            kdf_parallelism: None,
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
}
