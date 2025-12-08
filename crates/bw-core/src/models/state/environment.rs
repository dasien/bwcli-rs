use serde::{Deserialize, Serialize};

/// Environment server URLs configuration
///
/// Compatible with TypeScript CLI storage format
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentUrls {
    /// Base API URL (default: https://bitwarden.com)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base: Option<String>,

    /// API server URL (default: https://api.bitwarden.com)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api: Option<String>,

    /// Identity server URL (default: https://identity.bitwarden.com)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identity: Option<String>,

    /// Web vault URL (default: https://vault.bitwarden.com)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub web_vault: Option<String>,

    /// Icons server URL (default: https://icons.bitwarden.com)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icons: Option<String>,

    /// Notifications server URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notifications: Option<String>,

    /// Events server URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub events: Option<String>,
}

impl Default for EnvironmentUrls {
    fn default() -> Self {
        Self {
            base: Some("https://bitwarden.com".to_string()),
            api: Some("https://api.bitwarden.com".to_string()),
            identity: Some("https://identity.bitwarden.com".to_string()),
            web_vault: Some("https://vault.bitwarden.com".to_string()),
            icons: Some("https://icons.bitwarden.com".to_string()),
            notifications: Some("https://notifications.bitwarden.com".to_string()),
            events: Some("https://events.bitwarden.com".to_string()),
        }
    }
}
