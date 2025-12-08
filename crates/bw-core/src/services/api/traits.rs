use super::environment::Environment;
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Abstract API client interface for Bitwarden server communication
///
/// Provides methods for HTTP operations with and without authentication.
/// Implementations handle:
/// - Request serialization and response deserialization
/// - Bearer token injection for authenticated requests
/// - Automatic token refresh on 401 responses
/// - Error mapping to typed error enums
#[async_trait]
pub trait ApiClient: Send + Sync {
    /// Make an unauthenticated GET request
    ///
    /// # Arguments
    /// * `path` - API path relative to base URL (e.g., "/public/version")
    ///
    /// # Returns
    /// Deserialized response body of type T
    ///
    /// # Example
    /// ```rust,ignore
    /// let version: VersionResponse = client.get("/public/version").await?;
    /// ```
    async fn get<T>(&self, path: &str) -> Result<T>
    where
        T: for<'de> Deserialize<'de>;

    /// Make an authenticated GET request
    ///
    /// Automatically includes Bearer token in Authorization header.
    /// On 401 response, attempts token refresh and retries request.
    ///
    /// # Arguments
    /// * `path` - API path relative to base URL
    ///
    /// # Returns
    /// Deserialized response body of type T
    ///
    /// # Errors
    /// - `ApiError::Authentication` if token missing or refresh fails
    /// - `ApiError::Network` for connection failures
    /// - `ApiError::Server` for 5xx responses
    async fn get_with_auth<T>(&self, path: &str) -> Result<T>
    where
        T: for<'de> Deserialize<'de>;

    /// Make an unauthenticated POST request
    ///
    /// # Arguments
    /// * `path` - API path relative to base URL
    /// * `body` - Request body to serialize as JSON
    ///
    /// # Returns
    /// Deserialized response body of type R
    async fn post<T, R>(&self, path: &str, body: &T) -> Result<R>
    where
        T: Serialize + Send + Sync,
        R: for<'de> Deserialize<'de>;

    /// Make an authenticated POST request
    ///
    /// Automatically includes Bearer token in Authorization header.
    /// On 401 response, attempts token refresh and retries request.
    async fn post_with_auth<T, R>(&self, path: &str, body: &T) -> Result<R>
    where
        T: Serialize + Send + Sync,
        R: for<'de> Deserialize<'de>;

    /// Make an authenticated PUT request
    ///
    /// Updates an existing resource with provided data.
    async fn put_with_auth<T, R>(&self, path: &str, body: &T) -> Result<R>
    where
        T: Serialize + Send + Sync,
        R: for<'de> Deserialize<'de>;

    /// Make an authenticated DELETE request
    ///
    /// Deletes a resource. Returns empty result on success (204 No Content).
    async fn delete_with_auth(&self, path: &str) -> Result<()>;

    /// Get the current environment URLs
    ///
    /// Returns URLs for all Bitwarden services (api, identity, web vault, etc.)
    fn environment(&self) -> &Environment;

    /// Check if currently authenticated (has valid access token)
    ///
    /// Does not validate token with server, only checks local storage.
    async fn is_authenticated(&self) -> bool;
}
