use super::environment::Environment;
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Abstract API client interface for the endpoints the SDK does not cover.
///
/// Unauthenticated only — see [`super::BitwardenApiClient`] for why.
/// Implementations handle:
/// - Request serialization and response deserialization
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

    /// Get the current environment URLs
    ///
    /// Returns URLs for all Bitwarden services (api, identity, web vault, etc.)
    fn environment(&self) -> &Environment;
}
