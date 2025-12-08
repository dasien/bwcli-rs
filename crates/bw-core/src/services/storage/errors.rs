use std::path::PathBuf;
use thiserror::Error;

/// Storage-specific errors
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("Failed to read storage file {1}: {0}")]
    ReadError(#[source] std::io::Error, PathBuf),

    #[error("Failed to write storage file {1}: {0}")]
    WriteError(#[source] std::io::Error, PathBuf),

    #[error("Failed to parse storage file {1}: {0}")]
    ParseError(#[source] serde_json::Error, PathBuf),

    #[error("Failed to serialize value for key '{1}': {0}")]
    SerializationError(#[source] serde_json::Error, String),

    #[error("Failed to deserialize value for key '{1}': {0}")]
    DeserializationError(#[source] serde_json::Error, String),

    #[error("Failed to create directory {1}: {0}")]
    CreateDirectoryError(#[source] std::io::Error, PathBuf),

    #[error("Permission denied for path {1}: {0}")]
    PermissionError(#[source] std::io::Error, PathBuf),

    #[error("Path is not writable: {0}")]
    NotWritableError(PathBuf),

    #[error("Failed to resolve storage path: {0}")]
    PathResolutionError(String),

    #[error("BW_SESSION environment variable not set or invalid")]
    MissingSessionKey,

    #[error("Failed to decrypt value: {0}")]
    DecryptionError(String),

    #[error("Failed to encrypt value: {0}")]
    EncryptionError(String),

    #[error("Not implemented: {0}")]
    NotImplemented(String),
}
