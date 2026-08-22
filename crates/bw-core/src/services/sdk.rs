//! SDK Client Integration
//!
//! This module provides the SDK client for all cryptographic and vault operations.
//! It re-exports types from the Bitwarden SDK for use throughout the CLI.

use anyhow::Result;
use bitwarden_core::auth::{ClientManagedTokenHandler, ClientManagedTokens};
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

/// Create the SDK client with access to our stored access token.
///
/// Prefer this over [`create_sdk_client`] anywhere the client will make
/// authenticated calls: without a token handler the SDK's generated API clients
/// send no credentials, which is why the CLI historically maintained a second,
/// separate HTTP stack.
///
/// `ClientManagedTokenHandler` attaches the bearer token but does not refresh
/// it — refresh remains ours.
pub fn create_sdk_client_with_tokens(
    api_url: Option<String>,
    identity_url: Option<String>,
    tokens: Arc<dyn ClientManagedTokens>,
) -> Result<Client> {
    Ok(Client::new_with_token_handler(
        Some(client_settings(api_url, identity_url)),
        ClientManagedTokenHandler::new(tokens),
    ))
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
