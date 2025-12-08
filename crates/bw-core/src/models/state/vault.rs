use serde::{Deserialize, Serialize};

/// Vault synchronization state
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultState {
    /// Last sync timestamp (ISO 8601)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_sync: Option<String>,

    /// Encrypted master key - ENCRYPTED
    /// Stored as: __PROTECTED__keys.masterKey
    #[serde(skip)]
    pub master_key: Option<String>,

    /// Encrypted private key (RSA) - ENCRYPTED
    /// Stored as: __PROTECTED__keys.privateKey
    #[serde(skip)]
    pub private_key: Option<String>,

    /// Organization keys (encrypted) - ENCRYPTED
    /// Stored as: __PROTECTED__keys.orgKeys
    #[serde(skip)]
    pub org_keys: Option<Vec<OrgKey>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrgKey {
    /// Organization ID (GUID)
    pub org_id: String,

    /// Encrypted organization key
    pub key: String,
}
