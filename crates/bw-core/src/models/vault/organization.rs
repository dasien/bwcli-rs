//! Organization data models

use bitwarden_api_api::models::{
    OrganizationUserStatusType, OrganizationUserType, Permissions,
    ProfileOrganizationResponseModel,
};
use serde::{Deserialize, Serialize};

/// Organization (team/company)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Organization {
    /// Organization ID (UUID)
    pub id: String,

    /// Organization name (plain text)
    pub name: String,

    /// Status: 0=Invited, 1=Accepted, 2=Confirmed, 3=Staged, 4=Revoked
    pub status: u8,

    /// Organization type: 0=Owner, 1=Admin, 2=User, 3=Custom
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
///
/// Mirrors the server's `Permissions` model. Every field is `#[serde(default)]`
/// so state files written by older versions (which carried the retired
/// `accessBusinessPortal` / `manageAllCollections` / `manageAssignedCollections`
/// fields) still deserialize.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OrganizationPermissions {
    pub access_event_logs: bool,
    pub access_import_export: bool,
    pub access_reports: bool,
    pub create_new_collections: bool,
    pub edit_any_collection: bool,
    pub delete_any_collection: bool,
    pub manage_groups: bool,
    pub manage_policies: bool,
    pub manage_sso: bool,
    pub manage_users: bool,
    pub manage_reset_password: bool,
    pub manage_scim: bool,
    pub manage_access_rules: bool,
}

impl From<&Permissions> for OrganizationPermissions {
    fn from(p: &Permissions) -> Self {
        Self {
            access_event_logs: p.access_event_logs.unwrap_or(false),
            access_import_export: p.access_import_export.unwrap_or(false),
            access_reports: p.access_reports.unwrap_or(false),
            create_new_collections: p.create_new_collections.unwrap_or(false),
            edit_any_collection: p.edit_any_collection.unwrap_or(false),
            delete_any_collection: p.delete_any_collection.unwrap_or(false),
            manage_groups: p.manage_groups.unwrap_or(false),
            manage_policies: p.manage_policies.unwrap_or(false),
            manage_sso: p.manage_sso.unwrap_or(false),
            manage_users: p.manage_users.unwrap_or(false),
            manage_reset_password: p.manage_reset_password.unwrap_or(false),
            manage_scim: p.manage_scim.unwrap_or(false),
            manage_access_rules: p.manage_access_rules.unwrap_or(false),
        }
    }
}

/// Map the API status enum onto the numeric form the state file uses.
///
/// Matched explicitly rather than cast so an unrecognized server value is
/// visible instead of silently becoming `Invited`.
fn status_to_u8(status: Option<OrganizationUserStatusType>) -> u8 {
    match status {
        Some(OrganizationUserStatusType::Invited) => 0,
        Some(OrganizationUserStatusType::Accepted) => 1,
        Some(OrganizationUserStatusType::Confirmed) => 2,
        Some(OrganizationUserStatusType::Staged) => 3,
        Some(OrganizationUserStatusType::Revoked) => 4,
        Some(OrganizationUserStatusType::__Unknown(other)) => {
            tracing::warn!("Unknown organization user status from server: {other}");
            u8::try_from(other).unwrap_or(0)
        }
        None => 0,
    }
}

/// Map the API member-type enum onto the numeric form the state file uses.
fn org_type_to_u8(org_type: Option<OrganizationUserType>) -> u8 {
    match org_type {
        Some(OrganizationUserType::Owner) => 0,
        Some(OrganizationUserType::Admin) => 1,
        Some(OrganizationUserType::User) => 2,
        Some(OrganizationUserType::Custom) => 3,
        Some(OrganizationUserType::__Unknown(other)) => {
            tracing::warn!("Unknown organization user type from server: {other}");
            u8::try_from(other).unwrap_or(2)
        }
        None => 2,
    }
}

impl Organization {
    /// Build from a sync/profile response entry.
    ///
    /// Returns `None` when the server omits the id or name, since neither is
    /// usable without them.
    /// Extract the wrapped organization keys from a sync response's profile.
    ///
    /// Lives beside [`Self::from_api`] because both read the same models and
    /// both are needed on every sync. An organization whose key cannot be read
    /// is skipped rather than failing the sync: its items simply stay
    /// undecryptable, which is the same outcome as before and better than no
    /// sync at all.
    pub fn keys_from_api(
        models: &[ProfileOrganizationResponseModel],
    ) -> std::collections::HashMap<bitwarden_core::OrganizationId, bitwarden_crypto::UnsignedSharedKey>
    {
        models
            .iter()
            .filter_map(|o| {
                let id = bitwarden_core::OrganizationId::new(o.id?);
                match o
                    .key
                    .as_deref()?
                    .parse::<bitwarden_crypto::UnsignedSharedKey>()
                {
                    Ok(key) => Some((id, key)),
                    Err(e) => {
                        tracing::warn!("Skipping unreadable key for organization {id}: {e}");
                        None
                    }
                }
            })
            .collect()
    }

    pub fn from_api(model: &ProfileOrganizationResponseModel) -> Option<Self> {
        let id = model.id?;
        let name = model.name.clone()?;

        Some(Self {
            id: id.to_string(),
            name,
            status: status_to_u8(model.status),
            org_type: org_type_to_u8(model.r#type),
            enabled: model.enabled.unwrap_or(false),
            use_policies: model.use_policies.unwrap_or(false),
            use_groups: model.use_groups.unwrap_or(false),
            use_directory: model.use_directory.unwrap_or(false),
            use_events: model.use_events.unwrap_or(false),
            use_totp: model.use_totp.unwrap_or(false),
            use_api: model.use_api.unwrap_or(false),
            self_host: model.self_host.unwrap_or(false),
            permissions: model.permissions.as_deref().map(Into::into),
        })
    }
}
