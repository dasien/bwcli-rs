//! SDK Client Integration
//!
//! This module provides the SDK client for all cryptographic and vault operations.
//! It re-exports types from the Bitwarden SDK for use throughout the CLI.

use anyhow::Result;
use bitwarden_auth::token_management::PasswordManagerTokenHandler;
use bitwarden_core::client::persisted_state::{BASE_URLS, BaseUrls, OrganizationSharedKey};
use bitwarden_core::key_management::LocalUserDataKeyState;
use bitwarden_send::Send as SendItem;
use bitwarden_state::SettingItem;
use bitwarden_state::repository::{RepositoryItem, RepositoryMigrationStep, RepositoryMigrations};
use bitwarden_vault::{Cipher, Folder};
use bitwarden_state::registry::StateRegistry;
use bitwarden_state::DatabaseConfiguration;
use std::path::PathBuf;
use std::sync::Arc;

// Re-export SDK types for use throughout the crate
pub use bitwarden_core::{Client, ClientSettings, DeviceType};

/// Get the appropriate DeviceType for the current platform
pub fn get_device_type() -> DeviceType {
    #[cfg(target_os = "linux")]
    {
        DeviceType::LinuxCLI
    }

    #[cfg(target_os = "macos")]
    {
        DeviceType::MacOsCLI
    }

    #[cfg(target_os = "windows")]
    {
        DeviceType::WindowsCLI
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        DeviceType::SDK
    }
}

/// Create the SDK client for all crypto and vault operations
///
/// # Arguments
/// * `api_url` - Optional API server URL (default: https://api.bitwarden.com)
/// * `identity_url` - Optional Identity server URL (default: https://identity.bitwarden.com)
///
/// # Returns
/// Configured SDK client ready for authentication and vault operations
pub fn create_sdk_client(api_url: Option<String>, identity_url: Option<String>) -> Result<Client> {
    Ok(Client::new(Some(client_settings(api_url, identity_url))))
}

/// Open the SDK's state database.
///
/// State lives in `{appdata}/user.sqlite`, one table per registered repository
/// (`Cipher`, `Folder`, `Send`, `SettingItem`, `OrganizationSharedKey`). This is
/// the store the SDK's own clients read and write, so registering it is what
/// lets us hand vault CRUD, sync and unlock over to the SDK instead of
/// maintaining parallel implementations.
///
/// `db_name` is fixed rather than per-user: the CLI has a single active account
/// at a time, and `logout` wipes the registry.
///
/// Separate from [`create_sdk_client_with_state`] because the server URLs the
/// client is built with are themselves *in* this database (`BASE_URLS`), so it
/// has to be readable before the client exists.
pub async fn open_state(appdata_dir: PathBuf) -> Result<StateRegistry> {
    StateRegistry::new_with_db(
        DatabaseConfiguration::Sqlite {
            db_name: "user".to_string(),
            folder_path: appdata_dir,
        },
        state_migrations(),
    )
    .await
    .map_err(|e| anyhow::anyhow!("could not open the local state database: {e}"))
}

/// The server URLs recorded at login, if any.
///
/// Read back so a self-hosted user does not have to repeat `--server` on every
/// invocation — and, more importantly, so token renewal targets the server the
/// refresh token actually came from.
pub async fn stored_base_urls(registry: &StateRegistry) -> Option<BaseUrls> {
    registry.setting(BASE_URLS).ok()?.get().await.ok().flatten()
}

/// Create the SDK client over an already-open state database.
///
/// Authentication is [`PasswordManagerTokenHandler`], which reads and writes the
/// `AUTHENTICATION_TOKENS` setting in that same database. It attaches the bearer
/// token to requests the generated clients mark as authenticated, renews it
/// proactively (5-minute margin) and once more on a 401, and serializes
/// concurrent renewals behind a mutex. It replaces our `TokenManager`, whose
/// hand-rolled equivalent of all three had bugs in each.
pub fn create_sdk_client_with_state(
    api_url: Option<String>,
    identity_url: Option<String>,
    registry: StateRegistry,
) -> Client {
    Client::builder()
        .with_settings(client_settings(api_url, identity_url))
        .with_token_handler(Arc::new(PasswordManagerTokenHandler::default()))
        .with_state(registry)
        .build()
}

/// Repository tables to create in the state database.
///
/// This mirrors `bitwarden_pm::migrations::get_sdk_managed_migrations` and adds
/// `LocalUserDataKeyState`, which that list omits even though
/// `initialize_user_crypto` tries to initialize it — without the table, every
/// unlock logs `Unable to initialize local user data key` at ERROR level.
/// Upstream's own `bw` hits the same thing.
///
/// Order matters and is append-only: removing a repository needs an explicit
/// `Remove` step, not a deletion from this list. Keep the shared entries in the
/// same order as the SDK's list so the two stay compatible.
fn state_migrations() -> RepositoryMigrations {
    use RepositoryMigrationStep::*;

    RepositoryMigrations::new(vec![
        Add(Cipher::data()),
        Add(Folder::data()),
        Add(SettingItem::data()),
        Add(OrganizationSharedKey::data()),
        Add(SendItem::data()),
        Add(LocalUserDataKeyState::data()),
    ])
}

/// Settings shared by every client we build.
fn client_settings(api_url: Option<String>, identity_url: Option<String>) -> ClientSettings {
    // Start from the SDK defaults (correct for Bitwarden cloud) and override only
    // what the CLI is responsible for. Spreading `..default()` rather than listing
    // every field keeps this compiling when the SDK adds settings, which is how
    // this broke on the 2.0 -> 3.0 upgrade.
    let mut settings = ClientSettings {
        device_type: get_device_type(),
        user_agent: format!("Bitwarden CLI/{}", env!("CARGO_PKG_VERSION")),
        bitwarden_client_version: Some(env!("CARGO_PKG_VERSION").to_string()),
        bitwarden_package_type: Some("cli".to_string()),
        ..ClientSettings::default()
    };

    // Self-hosted overrides.
    if let Some(url) = api_url {
        settings.api_url = url;
    }
    if let Some(url) = identity_url {
        settings.identity_url = url;
    }

    settings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_sdk_client_defaults() {
        let client = create_sdk_client(None, None);
        assert!(client.is_ok(), "Should create client with default URLs");
    }

    #[test]
    fn test_create_sdk_client_custom_urls() {
        let client = create_sdk_client(
            Some("https://api.example.com".to_string()),
            Some("https://identity.example.com".to_string()),
        );
        assert!(client.is_ok(), "Should create client with custom URLs");
    }

    #[test]
    fn test_get_device_type() {
        let device_type = get_device_type();
        // Should return a CLI device type for the current platform
        #[cfg(target_os = "linux")]
        assert!(matches!(device_type, DeviceType::LinuxCLI));

        #[cfg(target_os = "macos")]
        assert!(matches!(device_type, DeviceType::MacOsCLI));

        #[cfg(target_os = "windows")]
        assert!(matches!(device_type, DeviceType::WindowsCLI));
    }
}
