//! SDK Client Integration
//!
//! This module provides the SDK client for all cryptographic and vault operations.
//! It re-exports types from the Bitwarden SDK for use throughout the CLI.

use anyhow::Result;

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
    // If no custom URLs provided, use SDK defaults which are correct for Bitwarden cloud
    let settings = match (&api_url, &identity_url) {
        (None, None) => {
            // Use SDK defaults but with CLI-specific device type and user agent
            Some(ClientSettings {
                device_type: get_device_type(),
                user_agent: format!("Bitwarden CLI/{}", env!("CARGO_PKG_VERSION")),
                bitwarden_client_version: Some(env!("CARGO_PKG_VERSION").to_string()),
                ..ClientSettings::default()
            })
        }
        _ => {
            // Custom URLs provided (self-hosted)
            Some(ClientSettings {
                api_url: api_url.unwrap_or_else(|| "https://api.bitwarden.com".to_string()),
                identity_url: identity_url
                    .unwrap_or_else(|| "https://identity.bitwarden.com".to_string()),
                user_agent: format!("Bitwarden CLI/{}", env!("CARGO_PKG_VERSION")),
                device_type: get_device_type(),
                bitwarden_client_version: Some(env!("CARGO_PKG_VERSION").to_string()),
            })
        }
    };

    Ok(Client::new(settings))
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
