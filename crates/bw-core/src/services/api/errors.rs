use reqwest::StatusCode;
use thiserror::Error;

/// API client errors with context for user-friendly messages
#[derive(Debug, Error)]
pub enum ApiError {
    /// Network connectivity error (DNS, connection refused, timeout)
    #[error("Network error: {message}\n{troubleshooting}")]
    Network {
        message: String,
        troubleshooting: String,
        #[source]
        source: Option<reqwest::Error>,
    },

    /// Authentication error (401, 403, missing token)
    #[error("Authentication error: {message}\n{hint}")]
    Authentication { message: String, hint: String },

    /// Not found error (404)
    #[error("Resource not found: {resource}")]
    NotFound { resource: String },

    /// Rate limit error (429)
    #[error("Rate limit exceeded. {message}")]
    RateLimit {
        message: String,
        retry_after: Option<u64>,
    },

    /// Client error (other 4xx)
    #[error("Client error ({status}): {message}")]
    Client { status: StatusCode, message: String },

    /// Server error (5xx)
    #[error("Server error ({status}): {message}\n{hint}")]
    Server {
        status: StatusCode,
        message: String,
        hint: String,
    },

    /// Request timeout
    #[error("Request timeout: {message}\n{hint}")]
    Timeout { message: String, hint: String },

    /// TLS/certificate error
    #[error("TLS error: {message}\n{hint}")]
    Tls { message: String, hint: String },

    /// JSON serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Invalid URL or configuration
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Generic error for unexpected cases
    #[error("Error: {0}")]
    Other(String),
}

impl ApiError {
    /// Create network error with troubleshooting hints
    pub fn network_error(err: reqwest::Error) -> Self {
        let message = if err.is_timeout() {
            "Request timed out".to_string()
        } else if err.is_connect() {
            "Failed to connect to server".to_string()
        } else {
            format!("Network request failed: {}", err)
        };

        let troubleshooting = if err.is_timeout() {
            "Check your internet connection or increase timeout with --timeout flag".to_string()
        } else if err.is_connect() {
            "Check server URL, DNS settings, and firewall configuration".to_string()
        } else {
            "Check your network connection and proxy settings".to_string()
        };

        Self::Network {
            message,
            troubleshooting,
            source: Some(err),
        }
    }

    /// Create authentication error
    pub fn auth_error(message: String) -> Self {
        let hint = if message.contains("expired") || message.contains("invalid") {
            "Run 'bw login' to authenticate again".to_string()
        } else {
            "Run 'bw unlock' to unlock your vault".to_string()
        };

        Self::Authentication { message, hint }
    }

    /// Create server error with helpful hints
    pub fn server_error(status: StatusCode, message: String) -> Self {
        let hint = match status.as_u16() {
            502 | 503 => {
                "Server temporarily unavailable. Please try again in a few moments.".to_string()
            }
            500 => {
                "Internal server error. If this persists, contact Bitwarden support.".to_string()
            }
            _ => "Server error occurred. Please try again later.".to_string(),
        };

        Self::Server {
            status,
            message,
            hint,
        }
    }

    /// Create rate limit error
    pub fn rate_limit_error(retry_after: Option<u64>) -> Self {
        let message = if let Some(seconds) = retry_after {
            format!("Please wait {} seconds before retrying.", seconds)
        } else {
            "Please wait a few moments before retrying.".to_string()
        };

        Self::RateLimit {
            message,
            retry_after,
        }
    }
}

impl From<reqwest::Error> for ApiError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            Self::Timeout {
                message: "Request timed out".to_string(),
                hint: "Check your network connection or increase timeout".to_string(),
            }
        } else if err.is_connect() {
            Self::network_error(err)
        } else {
            Self::network_error(err)
        }
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        Self::Other(err.to_string())
    }
}
