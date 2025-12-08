use base64::{Engine as _, engine::general_purpose::STANDARD};
use rand::{RngCore, rngs::OsRng};
use thiserror::Error;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Session key for encrypting local storage
///
/// Consists of 64 bytes (512 bits):
/// - 32 bytes encryption key (AES-256)
/// - 32 bytes MAC key (HMAC-SHA256)
#[derive(Clone, ZeroizeOnDrop)]
pub struct SessionKey {
    encryption_key: [u8; 32],
    mac_key: [u8; 32],
}

impl SessionKey {
    /// Generate a new random session key using cryptographically secure RNG
    pub fn generate() -> Self {
        let mut key_bytes = [0u8; 64];
        OsRng.fill_bytes(&mut key_bytes);

        Self {
            encryption_key: key_bytes[0..32].try_into().unwrap(),
            mac_key: key_bytes[32..64].try_into().unwrap(),
        }
    }

    /// Encode session key as base64 for export to BW_SESSION
    pub fn to_base64(&self) -> String {
        let mut bytes = Vec::with_capacity(64);
        bytes.extend_from_slice(&self.encryption_key);
        bytes.extend_from_slice(&self.mac_key);
        let encoded = STANDARD.encode(&bytes);
        bytes.zeroize();
        encoded
    }

    /// Parse session key from base64 string
    pub fn from_base64(encoded: &str) -> Result<Self, SessionKeyError> {
        let mut bytes = STANDARD
            .decode(encoded)
            .map_err(|_| SessionKeyError::InvalidFormat("Invalid base64 encoding".into()))?;

        if bytes.len() != 64 {
            bytes.zeroize();
            return Err(SessionKeyError::InvalidLength {
                expected: 64,
                actual: bytes.len(),
            });
        }

        let key = Self {
            encryption_key: bytes[0..32].try_into().unwrap(),
            mac_key: bytes[32..64].try_into().unwrap(),
        };

        bytes.zeroize();
        Ok(key)
    }

    /// Get encryption key slice (for internal use only)
    #[allow(dead_code)]
    pub(crate) fn encryption_key(&self) -> &[u8; 32] {
        &self.encryption_key
    }

    /// Get MAC key slice (for internal use only)
    #[allow(dead_code)]
    pub(crate) fn mac_key(&self) -> &[u8; 32] {
        &self.mac_key
    }
}

/// Session key errors
#[derive(Debug, Error)]
pub enum SessionKeyError {
    #[error("Invalid session key format: {0}")]
    InvalidFormat(String),

    #[error("Invalid session key length: expected {expected} bytes, got {actual}")]
    InvalidLength { expected: usize, actual: usize },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_key_generation() {
        let key1 = SessionKey::generate();
        let key2 = SessionKey::generate();

        // Keys should be unique
        assert_ne!(key1.to_base64(), key2.to_base64());
    }

    #[test]
    fn test_session_key_encoding() {
        let key = SessionKey::generate();
        let encoded = key.to_base64();

        // Should be base64 encoded 64 bytes
        let decoded = STANDARD.decode(&encoded).unwrap();
        assert_eq!(decoded.len(), 64);
    }

    #[test]
    fn test_session_key_roundtrip() {
        let original = SessionKey::generate();
        let encoded = original.to_base64();
        let decoded = SessionKey::from_base64(&encoded).unwrap();

        assert_eq!(original.to_base64(), decoded.to_base64());
    }

    #[test]
    fn test_session_key_invalid_base64() {
        let result = SessionKey::from_base64("not-valid-base64!");
        assert!(result.is_err());
    }

    #[test]
    fn test_session_key_invalid_length() {
        let short_key = STANDARD.encode(b"too short");
        let result = SessionKey::from_base64(&short_key);
        assert!(matches!(result, Err(SessionKeyError::InvalidLength { .. })));
    }
}
