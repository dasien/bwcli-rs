use super::{
    api::{BitwardenApiClient, Environment},
    create_sdk_client,
    sdk::Client,
    storage::JsonFileStorage,
};
use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Service container for dependency injection
///
/// Provides access to:
/// - SDK client (crypto, vault, auth operations)
/// - Storage (configuration and state persistence)
/// - API client (HTTP communication with Bitwarden servers)
pub struct ServiceContainer {
    /// Bitwarden SDK client - handles all crypto and most business logic
    sdk: Client,

    /// Storage service - configuration and state persistence
    storage: Arc<Mutex<JsonFileStorage>>,

    /// API client - HTTP communication with Bitwarden servers
    api_client: Arc<BitwardenApiClient>,
}

impl ServiceContainer {
    /// Create a new service container
    ///
    /// # Arguments
    /// * `api_url` - Optional API server URL
    /// * `identity_url` - Optional Identity server URL
    /// * `storage_path` - Optional custom storage directory path
    /// * `timeout_seconds` - Optional API request timeout
    pub fn new(
        api_url: Option<String>,
        identity_url: Option<String>,
        storage_path: Option<PathBuf>,
        timeout_seconds: Option<u64>,
    ) -> Result<Self> {
        let sdk = create_sdk_client(api_url.clone(), identity_url.clone())?;

        // Create storage wrapped in Mutex since Storage trait methods need &mut self
        let storage = Arc::new(Mutex::new(JsonFileStorage::new(storage_path)?));

        // Determine environment URLs
        let base_url = api_url
            .or(identity_url)
            .unwrap_or_else(|| "https://vault.bitwarden.com".to_string());
        let environment = Environment::from_base_url(&base_url)?;

        // Initialize API client (shares the same storage instance)
        let api_client = Arc::new(BitwardenApiClient::new(
            environment,
            Arc::clone(&storage),
            timeout_seconds,
        )?);

        Ok(Self {
            sdk,
            storage,
            api_client,
        })
    }

    /// Get reference to SDK client
    ///
    /// Use this for all crypto operations (encrypt, decrypt, key derivation)
    /// and vault operations (sync, cipher operations, etc.)
    pub fn sdk(&self) -> &Client {
        &self.sdk
    }

    /// Get reference to storage service
    ///
    /// Use this for configuration and state persistence
    pub fn storage(&self) -> Arc<Mutex<JsonFileStorage>> {
        Arc::clone(&self.storage)
    }

    /// Get reference to API client
    ///
    /// Use this for HTTP communication with Bitwarden servers
    pub fn api_client(&self) -> Arc<BitwardenApiClient> {
        Arc::clone(&self.api_client)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_container_creation() {
        let container = ServiceContainer::new(None, None, None, None);
        assert!(container.is_ok(), "Should create service container");
    }
}
