use thiserror::Error;

#[derive(Debug, Error)]
pub enum SendError {
    #[error("Encryption error: {0}")]
    Encryption(String),

    #[error("API error: {0}")]
    Api(String),

    #[error("Send not found: {0}")]
    NotFound(String),

    #[error("Send expired")]
    Expired,

    #[error("Send access limit exceeded")]
    AccessLimitExceeded,

    #[error("Invalid password")]
    InvalidPassword,

    #[error("Invalid Send URL: {0}")]
    InvalidUrl(String),

    #[error("File error: {0}")]
    FileError(#[from] std::io::Error),

    #[error("Not yet implemented")]
    NotImplemented,
}
