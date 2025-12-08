//! Validation error types for vault write operations

use thiserror::Error;

/// Validation errors for cipher and folder operations
#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("Required field '{0}' is missing")]
    MissingField(String),

    #[error("Field '{0}' is empty")]
    EmptyField(String),

    #[error("Field '{field}' is too long (max {max}, actual {actual})")]
    FieldTooLong {
        field: String,
        max: usize,
        actual: usize,
    },

    #[error("Invalid UUID format for '{field}': {value}")]
    InvalidUuid { field: String, value: String },

    #[error("Invalid format for '{field}': expected {expected}, got {actual}")]
    InvalidFormat {
        field: String,
        expected: String,
        actual: String,
    },

    #[error("Invalid cipher type: {0}")]
    InvalidCipherType(u8),

    #[error("Type mismatch: cipher type {cipher_type} requires {field}")]
    TypeMismatch { cipher_type: String, field: String },
}
