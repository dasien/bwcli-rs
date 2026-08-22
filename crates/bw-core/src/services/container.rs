use super::{
    api::{BitwardenApiClient, Environment},
    create_sdk_client,
    key_service::KeyService,
    send_repository::JsonSendRepository,
    sdk::Client,
    storage::{AccountManager, JsonFileStorage},
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
        // Use default cloud environment if no custom URLs provided
        let environment = match (&api_url, &identity_url) {
            (None, None) => Environment::default_cloud(),
            _ => {
                let base_url = api_url
                    .or(identity_url)
                    .unwrap_or_else(|| "https://vault.bitwarden.com".to_string());
                Environment::from_base_url(&base_url)?
            }
        };

        // Initialize API client (shares the same storage instance)
        let api_client = Arc::new(BitwardenApiClient::new(
            environment,
            Arc::clone(&storage),
            timeout_seconds,
        )?);

        // Point the SDK's send CRUD at our state file. Without this it falls
        // back to an in-memory database that starts empty every invocation, so
        // `bw send list` would never return anything.
        let account_manager = Arc::new(AccountManager::new(Arc::clone(&storage)));
        sdk.platform()
            .state()
            .register_client_managed(Arc::new(JsonSendRepository::new(
                Arc::clone(&storage),
                Arc::clone(&account_manager),
            )));

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

    /// Load the user key into the SDK client's key store from a session key.
    ///
    /// The SDK client starts each process with an empty key store, so this must
    /// run before any command performs vault crypto. Safe to call for commands
    /// that don't need crypto — it only touches storage and the key store.
    pub async fn unlock_sdk(&self, session_str: &str) -> Result<()> {
        let account_manager = Arc::new(AccountManager::new(self.storage()));
        let key_service = KeyService::new(self.storage(), account_manager);

        key_service
            .initialize_client_crypto(&self.sdk, session_str)
            .await?;

        Ok(())
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
