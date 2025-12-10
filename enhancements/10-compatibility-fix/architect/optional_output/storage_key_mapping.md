# Storage Key Mapping Reference

This document provides a complete mapping between the current Rust CLI storage keys and the required TypeScript CLI-compatible namespaced keys.

## Key Format Transformation

### Current Rust CLI → TypeScript CLI Compatible

| Current Key | New Key Pattern | Example |
|-------------|-----------------|---------|
| `accessToken` | `user_{userId}_token_accessToken` | `user_abc123_token_accessToken` |
| `refreshToken` | `user_{userId}_token_refreshToken` | `user_abc123_token_refreshToken` |
| `userKey` | `user_{userId}_crypto_privateKey` | `user_abc123_crypto_privateKey` |
| `userProfile` | Split across multiple keys | See below |
| `kdfConfig` | `user_{userId}_kdf_config` | `user_abc123_kdf_config` |
| `deviceId` | `global_applicationId_appId` | (unchanged format) |

### UserProfile Decomposition

The current `userProfile` object needs to be split:

```json
// Current flat structure
{
  "userProfile": {
    "id": "abc123",
    "email": "user@example.com",
    "name": "John Doe",
    "emailVerified": true,
    "premium": false
  }
}
```

Becomes:

```json
// TypeScript CLI namespaced structure
{
  "global_account_accounts": {
    "abc123": {
      "email": "user@example.com",
      "emailVerified": true
    }
  },
  "global_account_activeAccountId": "abc123"
}
```

## Complete Key Registry

### Global Keys

| Key | Type | Purpose | Required |
|-----|------|---------|----------|
| `stateVersion` | `number` | Storage format version | Yes |
| `global_applicationId_appId` | `string` (UUID) | Application instance ID | Yes |
| `global_account_accounts` | `object` | Account registry | Yes |
| `global_account_activeAccountId` | `string \| null` | Active user ID | Yes |
| `global_account_activity` | `object` | Account activity tracking | No |
| `global_clearEvent_logout` | `array` | State to clear on logout | No |
| `global_clearEvent_lock` | `array` | State to clear on lock | No |
| `global_tokenDiskLocal_emailTwoFactorTokenRecord` | `object` | 2FA token cache | No |
| `global_config_byServer` | `object` | Per-server config cache | No |

### User-Namespaced Keys

Format: `user_{userId}_{category}_{key}`

| Category | Key | Type | Purpose | Required |
|----------|-----|------|---------|----------|
| token | accessToken | `string \| null` | OAuth access token | Yes |
| token | refreshToken | `string \| null` | OAuth refresh token | Yes |
| token | apiKeyClientId | `string \| null` | API key client ID | No |
| token | apiKeyClientSecret | `string \| null` | API key secret | No |
| crypto | privateKey | `string \| null` | Encrypted RSA private key | Yes |
| crypto | providerKeys | `object \| null` | Provider encryption keys | No |
| crypto | organizationKeys | `object \| null` | Organization keys | No |
| crypto | everHadUserKey | `boolean \| null` | User key history flag | No |
| crypto | userSigningKey | `string \| null` | User signing key | No |
| masterPassword | masterKeyHash | `string \| null` | Master password hash | No |
| environment | environment | `object` | Environment URL settings | Yes |
| vaultTimeoutSettings | vaultTimeout | `string` | Timeout duration | No |
| vaultTimeoutSettings | vaultTimeoutAction | `string` | Timeout action | No |
| userDecryptionOptions | decryptionOptions | `object` | Decryption capability flags | No |
| avatar | avatarColor | `string \| null` | User avatar color | No |
| keyConnector | convertAccountToKeyConnector | `boolean \| null` | Key connector flag | No |
| collection | collections | `array \| null` | User's collections | No |
| pinUnlock | pinKeyEncryptedUserKeyPersistent | `string \| null` | PIN unlock key | No |
| pinUnlock | userKeyEncryptedPin | `string \| null` | Encrypted PIN | No |
| pinUnlock | oldPinKeyEncryptedMasterKey | `string \| null` | Legacy PIN key | No |

## StorageKey Enum Implementation

```rust
/// Storage key patterns for TypeScript CLI compatibility
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageKey {
    // Global keys (no user ID required)
    StateVersion,
    GlobalAppId,
    GlobalAccounts,
    GlobalActiveAccountId,
    GlobalActivity,
    GlobalClearEventLogout,
    GlobalClearEventLock,
    GlobalTwoFactorTokenRecord,
    GlobalConfigByServer,

    // User-namespaced token keys
    UserAccessToken,
    UserRefreshToken,
    UserApiKeyClientId,
    UserApiKeyClientSecret,

    // User-namespaced crypto keys
    UserPrivateKey,
    UserProviderKeys,
    UserOrganizationKeys,
    UserEverHadUserKey,
    UserSigningKey,

    // User-namespaced password keys
    UserMasterKeyHash,

    // User-namespaced environment keys
    UserEnvironment,

    // User-namespaced timeout keys
    UserVaultTimeout,
    UserVaultTimeoutAction,

    // User-namespaced decryption options
    UserDecryptionOptions,

    // User-namespaced avatar
    UserAvatarColor,

    // User-namespaced key connector
    UserKeyConnector,

    // User-namespaced collections
    UserCollections,

    // User-namespaced PIN unlock
    UserPinKeyPersistent,
    UserEncryptedPin,
    UserOldPinKey,

    // Custom key for CLI-specific data
    UserKdfConfig,
}

impl StorageKey {
    /// Returns true if this key requires a user ID
    pub fn requires_user_id(&self) -> bool {
        matches!(self,
            Self::UserAccessToken
            | Self::UserRefreshToken
            | Self::UserApiKeyClientId
            | Self::UserApiKeyClientSecret
            | Self::UserPrivateKey
            | Self::UserProviderKeys
            | Self::UserOrganizationKeys
            | Self::UserEverHadUserKey
            | Self::UserSigningKey
            | Self::UserMasterKeyHash
            | Self::UserEnvironment
            | Self::UserVaultTimeout
            | Self::UserVaultTimeoutAction
            | Self::UserDecryptionOptions
            | Self::UserAvatarColor
            | Self::UserKeyConnector
            | Self::UserCollections
            | Self::UserPinKeyPersistent
            | Self::UserEncryptedPin
            | Self::UserOldPinKey
            | Self::UserKdfConfig
        )
    }

    /// Format the key for storage
    ///
    /// # Panics
    /// Panics if key requires user ID but none provided
    pub fn format(&self, user_id: Option<&str>) -> String {
        if self.requires_user_id() && user_id.is_none() {
            panic!("StorageKey {:?} requires user_id", self);
        }

        match self {
            // Global keys
            Self::StateVersion => "stateVersion".to_string(),
            Self::GlobalAppId => "global_applicationId_appId".to_string(),
            Self::GlobalAccounts => "global_account_accounts".to_string(),
            Self::GlobalActiveAccountId => "global_account_activeAccountId".to_string(),
            Self::GlobalActivity => "global_account_activity".to_string(),
            Self::GlobalClearEventLogout => "global_clearEvent_logout".to_string(),
            Self::GlobalClearEventLock => "global_clearEvent_lock".to_string(),
            Self::GlobalTwoFactorTokenRecord => "global_tokenDiskLocal_emailTwoFactorTokenRecord".to_string(),
            Self::GlobalConfigByServer => "global_config_byServer".to_string(),

            // User token keys
            Self::UserAccessToken => format!("user_{}_token_accessToken", user_id.unwrap()),
            Self::UserRefreshToken => format!("user_{}_token_refreshToken", user_id.unwrap()),
            Self::UserApiKeyClientId => format!("user_{}_token_apiKeyClientId", user_id.unwrap()),
            Self::UserApiKeyClientSecret => format!("user_{}_token_apiKeyClientSecret", user_id.unwrap()),

            // User crypto keys
            Self::UserPrivateKey => format!("user_{}_crypto_privateKey", user_id.unwrap()),
            Self::UserProviderKeys => format!("user_{}_crypto_providerKeys", user_id.unwrap()),
            Self::UserOrganizationKeys => format!("user_{}_crypto_organizationKeys", user_id.unwrap()),
            Self::UserEverHadUserKey => format!("user_{}_crypto_everHadUserKey", user_id.unwrap()),
            Self::UserSigningKey => format!("user_{}_crypto_userSigningKey", user_id.unwrap()),

            // User password keys
            Self::UserMasterKeyHash => format!("user_{}_masterPassword_masterKeyHash", user_id.unwrap()),

            // User environment keys
            Self::UserEnvironment => format!("user_{}_environment_environment", user_id.unwrap()),

            // User timeout keys
            Self::UserVaultTimeout => format!("user_{}_vaultTimeoutSettings_vaultTimeout", user_id.unwrap()),
            Self::UserVaultTimeoutAction => format!("user_{}_vaultTimeoutSettings_vaultTimeoutAction", user_id.unwrap()),

            // User decryption options
            Self::UserDecryptionOptions => format!("user_{}_userDecryptionOptions_decryptionOptions", user_id.unwrap()),

            // User avatar
            Self::UserAvatarColor => format!("user_{}_avatar_avatarColor", user_id.unwrap()),

            // User key connector
            Self::UserKeyConnector => format!("user_{}_keyConnector_convertAccountToKeyConnector", user_id.unwrap()),

            // User collections
            Self::UserCollections => format!("user_{}_collection_collections", user_id.unwrap()),

            // User PIN unlock
            Self::UserPinKeyPersistent => format!("user_{}_pinUnlock_pinKeyEncryptedUserKeyPersistent", user_id.unwrap()),
            Self::UserEncryptedPin => format!("user_{}_pinUnlock_userKeyEncryptedPin", user_id.unwrap()),
            Self::UserOldPinKey => format!("user_{}_pinUnlock_oldPinKeyEncryptedMasterKey", user_id.unwrap()),

            // Custom CLI key
            Self::UserKdfConfig => format!("user_{}_kdf_config", user_id.unwrap()),
        }
    }

    /// Parse a storage key string back to enum (if recognized)
    pub fn parse(key: &str) -> Option<(Self, Option<String>)> {
        // Global keys
        match key {
            "stateVersion" => return Some((Self::StateVersion, None)),
            "global_applicationId_appId" => return Some((Self::GlobalAppId, None)),
            "global_account_accounts" => return Some((Self::GlobalAccounts, None)),
            "global_account_activeAccountId" => return Some((Self::GlobalActiveAccountId, None)),
            "global_account_activity" => return Some((Self::GlobalActivity, None)),
            _ => {}
        }

        // User-namespaced keys
        if key.starts_with("user_") {
            // Extract user ID and remaining key
            let parts: Vec<&str> = key.splitn(3, '_').collect();
            if parts.len() >= 3 {
                let user_id = parts[1].to_string();
                let rest = parts[2..].join("_");

                let key_type = match rest.as_str() {
                    "token_accessToken" => Some(Self::UserAccessToken),
                    "token_refreshToken" => Some(Self::UserRefreshToken),
                    "crypto_privateKey" => Some(Self::UserPrivateKey),
                    "masterPassword_masterKeyHash" => Some(Self::UserMasterKeyHash),
                    "environment_environment" => Some(Self::UserEnvironment),
                    "kdf_config" => Some(Self::UserKdfConfig),
                    _ => None,
                };

                if let Some(kt) = key_type {
                    return Some((kt, Some(user_id)));
                }
            }
        }

        None
    }
}
```

## Example Data Structures

### AccountInfo

```rust
/// Account information stored in global_account_accounts
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountInfo {
    /// User email address
    pub email: String,

    /// Whether email is verified
    #[serde(default)]
    pub email_verified: bool,
}

/// Account registry type
pub type AccountRegistry = HashMap<String, AccountInfo>;
```

### Environment Settings

```rust
/// Environment URL configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UserEnvironment {
    /// Custom API URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api: Option<String>,

    /// Custom identity URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identity: Option<String>,

    /// Custom vault URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vault: Option<String>,

    /// Custom icons URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icons: Option<String>,

    /// Custom notifications URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notifications: Option<String>,
}
```

## Migration Pseudocode

```rust
/// Migrate from flat keys to namespaced keys
pub async fn migrate_storage(storage: &mut JsonFileStorage) -> Result<()> {
    // Check if already migrated
    if storage.has("stateVersion")? {
        return Ok(());
    }

    // Get user ID from old profile
    let profile: Option<UserProfile> = storage.get("userProfile")?;
    let Some(profile) = profile else {
        // No profile = nothing to migrate
        return Ok(());
    };

    let user_id = &profile.id;

    // Read old values
    let access_token: Option<String> = storage.get("accessToken")?;
    let refresh_token: Option<String> = storage.get("refreshToken")?;
    let user_key: Option<String> = storage.get("userKey")?;
    let kdf_config: Option<KdfConfig> = storage.get("kdfConfig")?;

    // Set state version
    storage.set("stateVersion", &73u64).await?;

    // Create account registry
    let mut accounts = HashMap::new();
    accounts.insert(user_id.clone(), AccountInfo {
        email: profile.email.clone(),
        email_verified: profile.email_verified,
    });
    storage.set(&StorageKey::GlobalAccounts.format(None), &accounts).await?;

    // Set active account
    storage.set(
        &StorageKey::GlobalActiveAccountId.format(None),
        &user_id
    ).await?;

    // Migrate tokens
    if let Some(token) = access_token {
        storage.set(
            &StorageKey::UserAccessToken.format(Some(user_id)),
            &token
        ).await?;
    }

    if let Some(token) = refresh_token {
        storage.set(
            &StorageKey::UserRefreshToken.format(Some(user_id)),
            &token
        ).await?;
    }

    // Migrate user key
    if let Some(key) = user_key {
        storage.set(
            &StorageKey::UserPrivateKey.format(Some(user_id)),
            &key
        ).await?;
    }

    // Migrate KDF config
    if let Some(config) = kdf_config {
        storage.set(
            &StorageKey::UserKdfConfig.format(Some(user_id)),
            &config
        ).await?;
    }

    // Remove old keys
    storage.remove("accessToken").await?;
    storage.remove("refreshToken").await?;
    storage.remove("userKey").await?;
    storage.remove("userProfile").await?;
    storage.remove("kdfConfig").await?;

    storage.flush().await?;

    Ok(())
}
```

## Testing Reference Data

Sample TypeScript CLI data.json for testing:

```json
{
  "stateVersion": 73,
  "global_applicationId_appId": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "global_account_accounts": {
    "f7e6d5c4-b3a2-1098-fedc-ba0987654321": {
      "email": "test@example.com",
      "emailVerified": true
    }
  },
  "global_account_activeAccountId": "f7e6d5c4-b3a2-1098-fedc-ba0987654321",
  "global_account_activity": {},
  "user_f7e6d5c4-b3a2-1098-fedc-ba0987654321_token_accessToken": "eyJhbGciOiJSUzI1NiIsImtpZCI6...",
  "user_f7e6d5c4-b3a2-1098-fedc-ba0987654321_token_refreshToken": "ABC123DEF456-1",
  "user_f7e6d5c4-b3a2-1098-fedc-ba0987654321_crypto_privateKey": "2.abc123...",
  "user_f7e6d5c4-b3a2-1098-fedc-ba0987654321_environment_environment": {},
  "user_f7e6d5c4-b3a2-1098-fedc-ba0987654321_vaultTimeoutSettings_vaultTimeout": "never",
  "user_f7e6d5c4-b3a2-1098-fedc-ba0987654321_vaultTimeoutSettings_vaultTimeoutAction": "lock"
}
```
