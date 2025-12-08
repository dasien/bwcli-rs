//! Organization data models

use serde::{Deserialize, Serialize};

/// Organization (team/company)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Organization {
    /// Organization ID (UUID)
    pub id: String,

    /// Organization name (plain text)
    pub name: String,

    /// Status: 0=Invited, 1=Accepted, 2=Confirmed
    pub status: u8,

    /// Organization type: 0=Owner, 1=Admin, 2=User, 3=Manager
    #[serde(rename = "type")]
    pub org_type: u8,

    /// Whether user is enabled
    pub enabled: bool,

    /// Available features
    #[serde(default)]
    pub use_policies: bool,
    #[serde(default)]
    pub use_groups: bool,
    #[serde(default)]
    pub use_directory: bool,
    #[serde(default)]
    pub use_events: bool,
    #[serde(default)]
    pub use_totp: bool,
    #[serde(default)]
    pub use_api: bool,
    #[serde(default)]
    pub self_host: bool,

    /// Permissions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<OrganizationPermissions>,
}

/// Organization permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationPermissions {
    pub access_business_portal: bool,
    pub access_event_logs: bool,
    pub access_import_export: bool,
    pub access_reports: bool,
    pub manage_all_collections: bool,
    pub manage_assigned_collections: bool,
    pub manage_groups: bool,
    pub manage_policies: bool,
    pub manage_sso: bool,
    pub manage_users: bool,
    pub manage_reset_password: bool,
}
