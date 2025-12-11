//! API endpoint path constants
//!
//! Centralizes all Bitwarden API paths for easier maintenance and consistency.
//! Paths are relative to their respective base URLs (identity or api server).

/// Identity server endpoints (authentication)
pub mod identity {
    /// KDF configuration lookup (prelogin)
    pub const PRELOGIN: &str = "/identity/accounts/prelogin";

    /// OAuth2 token endpoint for password and API key login
    pub const TOKEN: &str = "/identity/connect/token";
}

/// API server endpoints
pub mod api {
    /// User profile
    pub const PROFILE: &str = "/accounts/profile";

    /// Full vault sync
    pub const SYNC: &str = "/sync";

    /// Ciphers endpoints
    pub mod ciphers {
        /// Base path for cipher operations (POST to create)
        pub const BASE: &str = "/ciphers";

        /// Get cipher by ID path
        pub fn by_id(id: &str) -> String {
            format!("/ciphers/{}", id)
        }

        /// Soft delete cipher path
        pub fn delete(id: &str) -> String {
            format!("/ciphers/{}/delete", id)
        }

        /// Restore cipher from trash path
        pub fn restore(id: &str) -> String {
            format!("/ciphers/{}/restore", id)
        }
    }

    /// Folders endpoints
    pub mod folders {
        /// Base path for folder operations (POST to create)
        pub const BASE: &str = "/folders";

        /// Get/update/delete folder by ID path
        pub fn by_id(id: &str) -> String {
            format!("/folders/{}", id)
        }
    }
}
