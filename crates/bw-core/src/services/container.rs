use super::{
    api::{BitwardenApiClient, Environment},
    create_sdk_client_with_state,
    send_repository::JsonSendRepository,
    sdk::Client,
    storage::{AccountManager, JsonFileStorage, StoragePath},
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
    pub async fn new(
        api_url: Option<String>,
        identity_url: Option<String>,
        storage_path: Option<PathBuf>,
        timeout_seconds: Option<u64>,
    ) -> Result<Self> {
        // Resolve the appdata directory once: both the legacy JSON store and the
        // SDK's SQLite state live there.
        let appdata_dir = StoragePath::resolve(storage_path.clone())?;

        // Create storage wrapped in Mutex since Storage trait methods need &mut self
        let storage = Arc::new(Mutex::new(JsonFileStorage::new(storage_path)?));

        // Determine environment URLs
        // Use default cloud environment if no custom URLs provided
        let environment = match (&api_url, &identity_url) {
            (None, None) => Environment::default_cloud(),
            _ => {
                let base_url = api_url
                    .clone()
                    .or_else(|| identity_url.clone())
                    .unwrap_or_else(|| "https://vault.bitwarden.com".to_string());
                Environment::from_base_url(&base_url)?
            }
        };

        // Login, prelogin and the identity endpoints only; unauthenticated.
        let api_client = Arc::new(BitwardenApiClient::new(environment, timeout_seconds)?);

        // The SDK owns authentication: its token handler reads and renews the
        // tokens persisted in the same state database.
        let sdk =
            create_sdk_client_with_state(api_url.clone(), identity_url.clone(), appdata_dir)
                .await?;

        // Sends still read and write the legacy JSON store: nothing populates the
        // SQLite `Send` table yet, because sync writes to data.json. Registering
        // a client-managed repository takes precedence over the SDK-managed one,
        // so this keeps sends working while the migration proceeds. Remove it
        // once sync writes through the SDK.
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
        crate::services::sdk_session::unlock_with_session(&self.sdk, session_str).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_service_container_creation() {
        let temp = tempfile::tempdir().unwrap();
        let container = ServiceContainer::new(
            None,
            None,
            Some(temp.path().to_path_buf()),
            None,
        )
        .await;
        assert!(container.is_ok(), "Should create service container");
    }

    /// The SDK's state database must actually be created on disk, otherwise
    /// every SDK client silently falls back to an empty in-memory store.
    #[tokio::test]
    async fn creates_the_sqlite_state_database() {
        let temp = tempfile::tempdir().unwrap();
        ServiceContainer::new(None, None, Some(temp.path().to_path_buf()), None)
            .await
            .unwrap();

        assert!(
            temp.path().join("user.sqlite").exists(),
            "expected user.sqlite in {:?}, found: {:?}",
            temp.path(),
            std::fs::read_dir(temp.path())
                .unwrap()
                .filter_map(|e| e.ok().map(|e| e.file_name()))
                .collect::<Vec<_>>()
        );
    }
}
