use super::{
    api::{BitwardenApiClient, Environment},
    create_sdk_client_with_state, open_state, stored_base_urls,
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

        // Open state before resolving URLs: the ones recorded at login live in
        // it, and both the SDK client and our own HTTP client need them.
        let registry = open_state(appdata_dir).await?;

        // Explicit arguments win; otherwise fall back to what login recorded.
        // Without the fallback a self-hosted user who omits `--server` silently
        // targets Bitwarden cloud — including token renewal, which would send a
        // self-hosted refresh token to identity.bitwarden.com.
        let stored = stored_base_urls(&registry).await;
        let api_url = api_url.or_else(|| stored.as_ref().map(|u| u.api_url.clone()));
        let identity_url = identity_url.or_else(|| stored.as_ref().map(|u| u.identity_url.clone()));

        let environment = resolve_environment(api_url.as_deref(), identity_url.as_deref())?;

        // Login, prelogin and the identity endpoints only; unauthenticated.
        let api_client = Arc::new(BitwardenApiClient::new(environment, timeout_seconds)?);

        // The SDK owns authentication: its token handler reads and renews the
        // tokens persisted in the same state database.
        let sdk = create_sdk_client_with_state(api_url, identity_url, registry);

        // An install that predates SQLite state keeps its login in `data.json`,
        // where nothing reads it any more. Carry it over before anything asks
        // whether we are authenticated. No-op once migrated, and never fatal.
        crate::services::state_import::migrate_if_needed(&sdk, &storage).await;

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

/// Build the service-URL set from an api/identity pair.
///
/// `Environment` models more than the SDK does (icons, notifications, events,
/// web vault), and derives them from a single base URL. A self-hosted deployment
/// puts api and identity under one base, so recovering that base from the api URL
/// is what lets the rest be derived.
fn resolve_environment(api_url: Option<&str>, identity_url: Option<&str>) -> Result<Environment> {
    let (Some(api), Some(identity)) = (api_url, identity_url) else {
        // A single URL, or neither: the existing single-base behaviour.
        return match api_url.or(identity_url) {
            Some(url) => Environment::from_base_url(url),
            None => Ok(Environment::default_cloud()),
        };
    };

    // Cloud uses unrelated hostnames per service, so its api URL is not a base
    // to derive anything from.
    let cloud = Environment::default_cloud();
    if api == cloud.api_url() && identity == cloud.identity_url() {
        return Ok(cloud);
    }

    let base = api
        .strip_suffix("/api")
        .or_else(|| api.strip_suffix('/'))
        .unwrap_or(api);

    Environment::custom(
        base,
        Some(api.to_string()),
        Some(identity.to_string()),
        None,
        None,
        None,
        None,
    )
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

    /// No URLs anywhere means Bitwarden cloud, whose services live on unrelated
    /// hostnames rather than under one base.
    #[test]
    fn no_urls_resolves_to_cloud() {
        let env = resolve_environment(None, None).unwrap();
        assert_eq!(env.api_url(), "https://api.bitwarden.com");
        assert_eq!(env.identity_url(), "https://identity.bitwarden.com");
    }

    /// Cloud's own api/identity pair must round-trip to the cloud environment,
    /// not get treated as a self-hosted base — `https://api.bitwarden.com` is not
    /// a base you can derive an icons URL from.
    #[test]
    fn the_cloud_url_pair_resolves_back_to_cloud() {
        let cloud = Environment::default_cloud();
        let env = resolve_environment(Some(cloud.api_url()), Some(cloud.identity_url())).unwrap();
        assert_eq!(env.web_vault_url(), cloud.web_vault_url());
        assert_eq!(env.icons_url(), cloud.icons_url());
    }

    /// A self-hosted pair keeps both URLs verbatim and recovers the base for the
    /// services `Environment` models but `BASE_URLS` does not carry.
    #[test]
    fn a_self_hosted_pair_keeps_both_urls_and_derives_the_base() {
        let env = resolve_environment(
            Some("https://vault.example.com/api"),
            Some("https://vault.example.com/identity"),
        )
        .unwrap();

        assert_eq!(env.api_url(), "https://vault.example.com/api");
        assert_eq!(env.identity_url(), "https://vault.example.com/identity");
        assert_eq!(env.icons_url(), "https://vault.example.com/icons");
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
