# Code Changes Detail

## Storage Key Patterns Reference

### Global Keys
| Pattern | Example | Description |
|---------|---------|-------------|
| `stateVersion` | `73` | Storage format version |
| `global_applicationId_appId` | `"uuid"` | Application UUID |
| `global_account_accounts` | `{"user-id": {...}}` | Account registry |
| `global_account_activeAccountId` | `"user-id"` or `null` | Active user |
| `global_deviceId` | `"uuid"` | Device identifier |

### User-Namespaced Keys
| Pattern | Example Key | Description |
|---------|-------------|-------------|
| `user_{id}_token_accessToken` | `user_abc123_token_accessToken` | OAuth access token |
| `user_{id}_token_refreshToken` | `user_abc123_token_refreshToken` | OAuth refresh token |
| `user_{id}_crypto_privateKey` | `user_abc123_crypto_privateKey` | RSA private key |
| `user_{id}_crypto_userKey` | `user_abc123_crypto_userKey` | User encryption key |
| `user_{id}_kdf_config` | `user_abc123_kdf_config` | KDF configuration |

## Example data.json Structure

After Rust CLI login, data.json will look like:

```json
{
  "stateVersion": 73,
  "global_applicationId_appId": "550e8400-e29b-41d4-a716-446655440000",
  "global_account_accounts": {
    "user-id-uuid": {
      "email": "user@example.com",
      "emailVerified": true
    }
  },
  "global_account_activeAccountId": "user-id-uuid",
  "global_deviceId": "device-uuid",
  "user_user-id-uuid_token_accessToken": "eyJ...",
  "user_user-id-uuid_token_refreshToken": "abc123-refresh",
  "user_user-id-uuid_crypto_userKey": "2.encrypted...",
  "user_user-id-uuid_kdf_config": {
    "kdf": "Argon2id",
    "kdfIterations": 3,
    "kdfMemory": 64,
    "kdfParallelism": 4
  }
}
```

## API Changes

### New Public Types

```rust
// From storage module
pub use account::{AccountInfo, AccountManager};
pub use keys::{StorageKey, SUPPORTED_STATE_VERSION};

// AccountInfo struct
pub struct AccountInfo {
    pub email: String,
    pub email_verified: bool,
    pub name: Option<String>,
}
```

### New Methods on JsonFileStorage

```rust
impl JsonFileStorage {
    /// Ensure state version is set (call during login)
    pub async fn ensure_state_version(&mut self) -> Result<()>;

    /// Get current state version
    pub fn get_state_version(&self) -> Option<u64>;
}
```

### New Methods on AccountManager

```rust
impl AccountManager {
    pub fn new(storage: Arc<Mutex<JsonFileStorage>>) -> Self;
    pub async fn get_active_user_id(&self) -> Result<Option<String>>;
    pub async fn set_active_user_id(&self, user_id: &str) -> Result<()>;
    pub async fn clear_active_account(&self) -> Result<()>;
    pub async fn register_account(&self, user_id: &str, email: &str) -> Result<()>;
    pub async fn get_account(&self, user_id: &str) -> Result<Option<AccountInfo>>;
    pub async fn get_all_accounts(&self) -> Result<HashMap<String, AccountInfo>>;
    pub async fn remove_account(&self, user_id: &str) -> Result<bool>;
    pub async fn is_logged_in(&self) -> Result<bool>;
}
```

### New Methods on SessionManager

```rust
impl SessionManager {
    /// Get access token for active user
    pub async fn get_access_token(&self) -> Result<Option<String>>;
}
```

### New Methods on AuthService

```rust
impl AuthService {
    /// Get reference to account manager
    pub fn account_manager(&self) -> &Arc<AccountManager>;
}
```

## Error Types Added

```rust
pub enum StorageError {
    // ... existing ...

    #[error("Unsupported state version {found}. This CLI requires version {required}+.")]
    UnsupportedStateVersion { found: u64, required: u64 },

    #[error("No active account. Please log in first.")]
    NoActiveAccount,

    #[error("Account not found: {user_id}")]
    AccountNotFound { user_id: String },
}
```

## Migration Path

### For Existing Rust CLI Users

Old data.json (flat keys):
```json
{
  "accessToken": "...",
  "refreshToken": "...",
  "userKey": "...",
  "userProfile": {...},
  "kdfConfig": {...}
}
```

After re-login, will become (namespaced):
```json
{
  "stateVersion": 73,
  "global_account_accounts": {...},
  "global_account_activeAccountId": "...",
  "user_..._token_accessToken": "...",
  ...
}
```

The old keys will remain (not removed) but will be ignored.

### For TypeScript CLI Users

No migration needed - Rust CLI now reads/writes compatible format.
