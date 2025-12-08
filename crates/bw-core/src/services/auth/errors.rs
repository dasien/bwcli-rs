use crate::models::auth::TwoFactorMethod;
use crate::services::{api::ApiError, storage::StorageError};
use bitwarden_crypto::CryptoError;
use thiserror::Error;

/// Authentication errors
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
    CryptoOperationFailed { message: String },

    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),

    #[error("API error: {0}")]
    Api(#[from] ApiError),

    #[error("SDK error: {0}")]
    Sdk(String),

    #[error("{0}")]
    Other(String),
}

/// Convert SDK CryptoError to AuthError
impl From<CryptoError> for AuthError {
    fn from(e: CryptoError) -> Self {
        match e {
            CryptoError::InvalidKey => AuthError::CryptoOperationFailed {
                message: "Invalid encryption key".to_string(),
            },
            CryptoError::InvalidMac => AuthError::InvalidPassword,
            CryptoError::InsufficientKdfParameters => AuthError::KdfError {
                message: "Insufficient KDF parameters".to_string(),
            },
            other => AuthError::CryptoOperationFailed {
                message: other.to_string(),
            },
        }
    }
}

impl AuthError {
    /// Get user-facing error message with actionable hints
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
            Self::CryptoOperationFailed { message } => {
                format!("Encryption error: {}\n\nThis may indicate corrupted data. Try logging out and back in.", message)
            }
            Self::Storage(e) => {
                format!("Storage error: {}\n\nCheck file permissions and disk space.", e)
            }
            Self::Api(e) => {
                format!("API error: {}", e)
            }
            Self::Sdk(e) => {
                format!("SDK error: {}", e)
            }
            Self::Other(msg) => msg.clone(),
        }
    }
}

/// Convert anyhow errors to AuthError
impl From<anyhow::Error> for AuthError {
    fn from(err: anyhow::Error) -> Self {
        AuthError::Other(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crypto_error_invalid_key_conversion() {
        let crypto_err = CryptoError::InvalidKey;
        let auth_err: AuthError = crypto_err.into();

        match auth_err {
            AuthError::CryptoOperationFailed { message } => {
                assert_eq!(message, "Invalid encryption key");
            }
            _ => panic!("Expected CryptoOperationFailed, got {:?}", auth_err),
        }
    }

    #[test]
    fn test_crypto_error_invalid_mac_conversion() {
        let crypto_err = CryptoError::InvalidMac;
        let auth_err: AuthError = crypto_err.into();

        match auth_err {
            AuthError::InvalidPassword => {
                // Expected - InvalidMac indicates wrong password
            }
            _ => panic!("Expected InvalidPassword, got {:?}", auth_err),
        }
    }

    #[test]
    fn test_crypto_error_insufficient_kdf_conversion() {
        let crypto_err = CryptoError::InsufficientKdfParameters;
        let auth_err: AuthError = crypto_err.into();

        match auth_err {
            AuthError::KdfError { message } => {
                assert_eq!(message, "Insufficient KDF parameters");
            }
            _ => panic!("Expected KdfError, got {:?}", auth_err),
        }
    }

    #[test]
    fn test_user_message_invalid_password() {
        let err = AuthError::InvalidPassword;
        let msg = err.user_message();
        assert!(msg.contains("Invalid master password"));
        assert!(msg.contains("bw login"));
    }

    #[test]
    fn test_user_message_kdf_error() {
        let err = AuthError::KdfError {
            message: "Test KDF error".to_string(),
        };
        let msg = err.user_message();
        assert!(msg.contains("Key derivation error"));
        assert!(msg.contains("Test KDF error"));
    }

    #[test]
    fn test_user_message_crypto_operation_failed() {
        let err = AuthError::CryptoOperationFailed {
            message: "Test crypto error".to_string(),
        };
        let msg = err.user_message();
        assert!(msg.contains("Encryption error"));
        assert!(msg.contains("Test crypto error"));
    }
}
