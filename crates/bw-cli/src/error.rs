//! CLI error types for consistent error handling
//!
//! Provides typed errors that distinguish between user-facing errors
//! (which become Response::error) and infrastructure errors (which propagate).

use crate::output::Response;
use thiserror::Error;

/// CLI-specific error types
#[derive(Error, Debug)]
pub enum CliError {
    /// User is not authenticated (needs to login)
    #[error("You are not logged in")]
    NotAuthenticated,

    /// Vault is locked (needs to unlock)
    #[error("Vault is locked. Run 'bw unlock' and set BW_SESSION environment variable")]
    VaultLocked,

    /// Requested item was not found
    #[error("Item not found: {0}")]
    NotFound(String),

    /// Invalid user input
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Feature not yet implemented
    #[error("Not yet implemented: {0}")]
    NotImplemented(String),

    /// Infrastructure/internal error (should propagate)
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl CliError {
    /// Convert error to appropriate result type
    ///
    /// Business logic errors become Response::error (user-friendly output)
    /// Infrastructure errors propagate as Err (shows full error chain)
    pub fn into_response(self) -> Result<Response, anyhow::Error> {
        match self {
            CliError::NotAuthenticated
            | CliError::VaultLocked
            | CliError::NotFound(_)
            | CliError::InvalidInput(_)
            | CliError::NotImplemented(_) => Ok(Response::error(self.to_string())),

            CliError::Internal(e) => Err(e),
        }
    }

    /// Create a not found error
    pub fn not_found(item: impl Into<String>) -> Self {
        CliError::NotFound(item.into())
    }

    /// Create an invalid input error
    pub fn invalid_input(message: impl Into<String>) -> Self {
        CliError::InvalidInput(message.into())
    }

    /// Create a not implemented error
    pub fn not_implemented(feature: impl Into<String>) -> Self {
        CliError::NotImplemented(feature.into())
    }
}

/// Extension trait to convert errors to CliError
pub trait IntoCliError<T> {
    fn cli_error(self) -> Result<T, CliError>;
}

impl<T, E: Into<anyhow::Error>> IntoCliError<T> for Result<T, E> {
    fn cli_error(self) -> Result<T, CliError> {
        self.map_err(|e| CliError::Internal(e.into()))
    }
}
