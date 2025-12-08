use anyhow::Result;
use serde::{Deserialize, Serialize};
use url::Url;

/// Environment configuration for Bitwarden services
///
/// Resolves URLs for all Bitwarden server endpoints.
/// Supports self-hosted installations with custom base URLs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Environment {
    /// Base URL (e.g., "https://vault.bitwarden.com")
    base: String,

    /// Resolved service URLs
    urls: ServiceUrls,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ServiceUrls {
    /// API server URL (default: {base}/api)
    api: String,

    /// Identity server URL (default: {base}/identity)
    identity: String,

    /// Web vault URL (default: {base})
    web_vault: String,

    /// Icons server URL (default: {base}/icons)
    icons: String,

    /// Notifications server URL (default: {base}/notifications)
    notifications: String,

    /// Events server URL (default: {base}/events)
    events: Option<String>,
}

impl Environment {
    /// Create environment from base URL
    ///
    /// # Arguments
    /// * `base_url` - Base URL for all services (e.g., "https://vault.bitwarden.com")
    ///
    /// # Validation
    /// - URL must be well-formed
    /// - Must use HTTPS (or HTTP for localhost)
    /// - No trailing slash
    pub fn from_base_url(base_url: &str) -> Result<Self> {
        let base = Self::validate_and_normalize_url(base_url)?;

        let urls = ServiceUrls {
            api: format!("{}/api", base),
            identity: format!("{}/identity", base),
            web_vault: base.clone(),
            icons: format!("{}/icons", base),
            notifications: format!("{}/notifications", base),
            events: Some(format!("{}/events", base)),
        };

        Ok(Self { base, urls })
    }

    /// Create environment with custom service URLs
    ///
    /// Allows overriding individual service URLs for advanced configurations.
    #[allow(clippy::too_many_arguments)]
    pub fn custom(
        base_url: &str,
        api_url: Option<String>,
        identity_url: Option<String>,
        web_vault_url: Option<String>,
        icons_url: Option<String>,
        notifications_url: Option<String>,
        events_url: Option<String>,
    ) -> Result<Self> {
        let base = Self::validate_and_normalize_url(base_url)?;

        let urls = ServiceUrls {
            api: api_url.unwrap_or_else(|| format!("{}/api", base)),
            identity: identity_url.unwrap_or_else(|| format!("{}/identity", base)),
            web_vault: web_vault_url.unwrap_or_else(|| base.clone()),
            icons: icons_url.unwrap_or_else(|| format!("{}/icons", base)),
            notifications: notifications_url.unwrap_or_else(|| format!("{}/notifications", base)),
            events: events_url.or_else(|| Some(format!("{}/events", base))),
        };

        Ok(Self { base, urls })
    }

    /// Default cloud environment
    pub fn default_cloud() -> Self {
        Self::from_base_url("https://vault.bitwarden.com").expect("Default cloud URL is valid")
    }

    /// Get API base URL
    pub fn api_url(&self) -> &str {
        &self.urls.api
    }

    /// Get Identity server URL
    pub fn identity_url(&self) -> &str {
        &self.urls.identity
    }

    /// Get Web Vault URL
    pub fn web_vault_url(&self) -> &str {
        &self.urls.web_vault
    }

    /// Get Icons server URL
    pub fn icons_url(&self) -> &str {
        &self.urls.icons
    }

    /// Get Notifications server URL
    pub fn notifications_url(&self) -> &str {
        &self.urls.notifications
    }

    /// Get Events server URL
    pub fn events_url(&self) -> Option<&str> {
        self.urls.events.as_deref()
    }

    /// Validate and normalize URL
    ///
    /// # Validation Rules
    /// - Must parse as valid URL
    /// - Must use HTTPS (exception: HTTP allowed for localhost/127.0.0.1)
    /// - Remove trailing slash
    fn validate_and_normalize_url(url_str: &str) -> Result<String> {
        let url =
            Url::parse(url_str).map_err(|e| anyhow::anyhow!("Invalid URL '{}': {}", url_str, e))?;

        // Validate HTTPS requirement (except localhost)
        if url.scheme() == "http" {
            let host = url.host_str().unwrap_or("");
            if !host.starts_with("localhost") && !host.starts_with("127.0.0.1") {
                return Err(anyhow::anyhow!(
                    "HTTPS required for remote servers. URL: {}",
                    url_str
                ));
            }
        }

        // Remove trailing slash
        let mut normalized = url.to_string();
        if normalized.ends_with('/') {
            normalized.pop();
        }

        Ok(normalized)
    }
}

impl Default for Environment {
    fn default() -> Self {
        Self::default_cloud()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_cloud_environment() {
        let env = Environment::default_cloud();
        assert_eq!(env.api_url(), "https://vault.bitwarden.com/api");
        assert_eq!(env.identity_url(), "https://vault.bitwarden.com/identity");
    }

    #[test]
    fn test_custom_base_url() {
        let env = Environment::from_base_url("https://my.server.com").unwrap();
        assert_eq!(env.api_url(), "https://my.server.com/api");
        assert_eq!(env.identity_url(), "https://my.server.com/identity");
    }

    #[test]
    fn test_https_validation() {
        let result = Environment::from_base_url("http://remote.server.com");
        assert!(result.is_err());
    }

    #[test]
    fn test_localhost_http_allowed() {
        let env = Environment::from_base_url("http://localhost:8080").unwrap();
        assert!(env.api_url().starts_with("http://localhost"));
    }

    #[test]
    fn test_trailing_slash_removal() {
        let env = Environment::from_base_url("https://vault.bitwarden.com/").unwrap();
        assert!(!env.api_url().ends_with("//api"));
    }
}
