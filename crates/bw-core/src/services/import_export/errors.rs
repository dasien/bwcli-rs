//! Error types for import/export operations

use thiserror::Error;

/// Validation error during import
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub line: Option<usize>,
    pub field: Option<String>,
    pub message: String,
}

/// Export-specific errors
#[derive(Debug, Error)]
pub enum ExportError {
    #[error("Not authenticated")]
    NotAuthenticated,

    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),

    #[error("Password required for encrypted export")]
    PasswordRequired,

    #[error("Failed to write output file: {0}")]
    FileWriteError(String),

    #[error("Failed to decrypt vault: {0}")]
    DecryptionError(String),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("CSV error: {0}")]
    CsvError(#[from] csv::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Operation cancelled")]
    OperationCancelled,
}

/// Import-specific errors
#[derive(Debug, Error)]
pub enum ImportError {
    #[error("Not authenticated")]
    NotAuthenticated,

    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),

    #[error("Failed to read import file: {0}")]
    FileReadError(String),

    #[error("Failed to parse import data: {0}")]
    ParseError(String),

    #[error("Validation failed with {error_count} error(s)")]
    ValidationError { error_count: usize },

    #[error("Password required for encrypted import")]
    PasswordRequired,

    #[error("Failed to import items: {0}")]
    ImportFailed(String),

    #[error("File too large: {size} bytes (max {max} bytes)")]
    FileTooLarge { size: u64, max: u64 },

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("CSV error: {0}")]
    CsvError(#[from] csv::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
}
