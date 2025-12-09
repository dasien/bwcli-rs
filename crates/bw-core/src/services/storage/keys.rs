//! Storage key definitions for TypeScript CLI compatibility
//!
//! The TypeScript CLI uses namespaced key patterns for storage.
//! This module provides type-safe key generation to ensure compatibility.

/// Storage key patterns for TypeScript CLI compatibility
///
/// Keys fall into two categories:
/// - Global keys: No user ID prefix (e.g., `stateVersion`, `global_account_accounts`)
/// - User-namespaced keys: Prefixed with user ID (e.g., `user_{id}_token_accessToken`)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageKey {
    // ============================================
    // Global keys (no user ID required)
    // ============================================
    /// Storage format version (currently 73)
    StateVersion,

    /// Application instance UUID
    GlobalAppId,

    /// Account registry: HashMap<UserId, AccountInfo>
    GlobalAccounts,

    /// Currently active user ID (or null)
    GlobalActiveAccountId,

    // ============================================
    // User-namespaced keys (require user ID)
    // ============================================
    /// OAuth access token
    UserAccessToken,

    /// OAuth refresh token
    UserRefreshToken,

    /// Encrypted RSA private key
    UserPrivateKey,

    /// Master password hash
    UserMasterKeyHash,

    /// Environment settings
    UserEnvironment,

    /// Vault timeout value
    UserVaultTimeout,

    /// Vault timeout action (lock/logout)
    UserVaultTimeoutAction,

    /// KDF configuration (custom key for our CLI)
    UserKdfConfig,

    /// Encrypted user key
    UserKey,

    // ============================================
    // Device/misc keys (global)
    // ============================================
    /// Device identifier UUID
    DeviceId,

    /// Session key hint (for lock/unlock flow)
    SessionKeyHint,
}

impl StorageKey {
    /// Format key for storage
    ///
    /// # Arguments
    /// * `user_id` - Optional user ID for user-namespaced keys
    ///
    /// # Panics
    /// Panics if `user_id` is None for user-namespaced keys
    ///
    /// # Examples
    /// ```ignore
    /// // Global keys don't need user_id
    /// let key = StorageKey::StateVersion.format(None);
    /// assert_eq!(key, "stateVersion");
    ///
    /// // User keys require user_id
    /// let key = StorageKey::UserAccessToken.format(Some("abc-123"));
    /// assert_eq!(key, "user_abc-123_token_accessToken");
    /// ```
    pub fn format(&self, user_id: Option<&str>) -> String {
        match self {
            // Global keys
            Self::StateVersion => "stateVersion".to_string(),
            Self::GlobalAppId => "global_applicationId_appId".to_string(),
            Self::GlobalAccounts => "global_account_accounts".to_string(),
            Self::GlobalActiveAccountId => "global_account_activeAccountId".to_string(),
            Self::DeviceId => "global_deviceId".to_string(),
            Self::SessionKeyHint => "sessionKeyHint".to_string(),

            // User-namespaced keys
            Self::UserAccessToken => {
                let uid = user_id.expect("UserAccessToken requires user_id");
                format!("user_{}_token_accessToken", uid)
            }
            Self::UserRefreshToken => {
                let uid = user_id.expect("UserRefreshToken requires user_id");
                format!("user_{}_token_refreshToken", uid)
            }
            Self::UserPrivateKey => {
                let uid = user_id.expect("UserPrivateKey requires user_id");
                format!("user_{}_crypto_privateKey", uid)
            }
            Self::UserMasterKeyHash => {
                let uid = user_id.expect("UserMasterKeyHash requires user_id");
                format!("user_{}_masterPassword_masterKeyHash", uid)
            }
            Self::UserEnvironment => {
                let uid = user_id.expect("UserEnvironment requires user_id");
                format!("user_{}_environment_environment", uid)
            }
            Self::UserVaultTimeout => {
                let uid = user_id.expect("UserVaultTimeout requires user_id");
                format!("user_{}_vaultTimeoutSettings_vaultTimeout", uid)
            }
            Self::UserVaultTimeoutAction => {
                let uid = user_id.expect("UserVaultTimeoutAction requires user_id");
                format!("user_{}_vaultTimeoutSettings_vaultTimeoutAction", uid)
            }
            Self::UserKdfConfig => {
                let uid = user_id.expect("UserKdfConfig requires user_id");
                format!("user_{}_kdfConfig_kdfConfig", uid)
            }
            Self::UserKey => {
                let uid = user_id.expect("UserKey requires user_id");
                format!("user_{}_masterPassword_masterKeyEncryptedUserKey", uid)
            }
        }
    }

    /// Check if this key requires a user ID
    pub fn requires_user_id(&self) -> bool {
        matches!(
            self,
            Self::UserAccessToken
                | Self::UserRefreshToken
                | Self::UserPrivateKey
                | Self::UserMasterKeyHash
                | Self::UserEnvironment
                | Self::UserVaultTimeout
                | Self::UserVaultTimeoutAction
                | Self::UserKdfConfig
                | Self::UserKey
        )
    }
}

/// Current supported state version
///
/// The TypeScript CLI uses state version 73 as of December 2025.
/// We only support version 73+ to avoid implementing state migrations.
pub const SUPPORTED_STATE_VERSION: u64 = 73;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_global_key_formatting() {
        assert_eq!(StorageKey::StateVersion.format(None), "stateVersion");
        assert_eq!(
            StorageKey::GlobalAppId.format(None),
            "global_applicationId_appId"
        );
        assert_eq!(
            StorageKey::GlobalAccounts.format(None),
            "global_account_accounts"
        );
        assert_eq!(
            StorageKey::GlobalActiveAccountId.format(None),
            "global_account_activeAccountId"
        );
    }

    #[test]
    fn test_user_key_formatting() {
        let user_id = "abc-123-def";

        assert_eq!(
            StorageKey::UserAccessToken.format(Some(user_id)),
            "user_abc-123-def_token_accessToken"
        );
        assert_eq!(
            StorageKey::UserRefreshToken.format(Some(user_id)),
            "user_abc-123-def_token_refreshToken"
        );
        assert_eq!(
            StorageKey::UserPrivateKey.format(Some(user_id)),
            "user_abc-123-def_crypto_privateKey"
        );
        assert_eq!(
            StorageKey::UserKdfConfig.format(Some(user_id)),
            "user_abc-123-def_kdfConfig_kdfConfig"
        );
    }

    #[test]
    #[should_panic(expected = "UserAccessToken requires user_id")]
    fn test_user_key_without_user_id_panics() {
        StorageKey::UserAccessToken.format(None);
    }

    #[test]
    fn test_requires_user_id() {
        // Global keys
        assert!(!StorageKey::StateVersion.requires_user_id());
        assert!(!StorageKey::GlobalAppId.requires_user_id());
        assert!(!StorageKey::GlobalAccounts.requires_user_id());
        assert!(!StorageKey::GlobalActiveAccountId.requires_user_id());
        assert!(!StorageKey::DeviceId.requires_user_id());

        // User keys
        assert!(StorageKey::UserAccessToken.requires_user_id());
        assert!(StorageKey::UserRefreshToken.requires_user_id());
        assert!(StorageKey::UserPrivateKey.requires_user_id());
        assert!(StorageKey::UserKdfConfig.requires_user_id());
    }
}
