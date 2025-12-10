//! Vault service error types

use crate::models::vault::ValidationError;
use thiserror::Error;

/// Vault service errors
#[derive(Debug, Error)]
pub enum VaultError {
    #[error("Not authenticated. Run 'bw login' first.")]
    NotAuthenticated,

    #[error("Vault not synced. Run 'bw sync' first.")]
    NotSynced,

    #[error("Item not found")]
    ItemNotFound,

    #[error("Field '{0}' not found on item")]
    FieldNotFound(&'static str),

    #[error("TOTP not configured for this item")]
    TotpNotConfigured,

    #[error("TOTP error: {0}")]
    TotpError(String),

    #[error("Folder not found")]
    FolderNotFound,

    #[error("Item is not in trash")]
    ItemNotDeleted,

    #[error("Validation error: {0}")]
    ValidationError(#[from] ValidationError),

    #[error("Encryption error: {0}")]
    EncryptionError(String),

    #[error("Operation cancelled by user")]
    OperationCancelled,

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("API error: {0}")]
    ApiError(String),

    #[error("Decryption error: {0}")]
    DecryptionError(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("IO error: {0}")]
    IoError(String),
}
