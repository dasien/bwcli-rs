# API Reference: Storage Compatibility Layer

This document provides API documentation for the storage compatibility components introduced in Enhancement 10.

## Module: `bw_core::services::storage::keys`

Type-safe storage key generation for TypeScript CLI compatibility.

### Enum: `StorageKey`

Represents all known storage key patterns.

```rust
pub enum StorageKey {
    // Global keys (no user ID required)
    StateVersion,
    GlobalAppId,
    GlobalAccounts,
    GlobalActiveAccountId,
    DeviceId,
    SessionKeyHint,

    // User-namespaced keys (require user ID)
    UserAccessToken,
    UserRefreshToken,
    UserPrivateKey,
    UserMasterKeyHash,
    UserEnvironment,
    UserVaultTimeout,
    UserVaultTimeoutAction,
    UserKdfConfig,
    UserKey,
}
```

#### Method: `format`

Formats the key for storage operations.

```rust
pub fn format(&self, user_id: Option<&str>) -> String
```

**Parameters:**
- `user_id` - Required for user-namespaced keys, ignored for global keys

**Returns:** The formatted key string

**Panics:** If `user_id` is `None` for a user-namespaced key

**Examples:**

```rust
use bw_core::services::storage::StorageKey;

// Global keys don't need user_id
let key = StorageKey::StateVersion.format(None);
assert_eq!(key, "stateVersion");

let key = StorageKey::GlobalActiveAccountId.format(None);
assert_eq!(key, "global_account_activeAccountId");

// User keys require user_id
let key = StorageKey::UserAccessToken.format(Some("abc-123"));
assert_eq!(key, "user_abc-123_token_accessToken");

let key = StorageKey::UserKdfConfig.format(Some("abc-123"));
assert_eq!(key, "user_abc-123_kdf_config");
```

#### Method: `requires_user_id`

Check if this key type requires a user ID.

```rust
pub fn requires_user_id(&self) -> bool
```

**Returns:** `true` for user-namespaced keys, `false` for global keys

**Example:**

```rust
assert!(!StorageKey::StateVersion.requires_user_id());
assert!(StorageKey::UserAccessToken.requires_user_id());
```

### Constant: `SUPPORTED_STATE_VERSION`

The minimum supported state version.

```rust
pub const SUPPORTED_STATE_VERSION: u64 = 73;
```

Currently 73 as of December 2025. Data.json files with lower versions are rejected.

---

## Module: `bw_core::services::storage::account`

Account registry management for multi-account support.

### Struct: `AccountInfo`

Account information stored in the global accounts registry.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountInfo {
    pub email: String,
    pub email_verified: bool,
    pub name: Option<String>,
}
```

**Fields:**
- `email` - User's email address
- `email_verified` - Whether the email has been verified
- `name` - Optional display name

### Struct: `AccountManager`

Manages the account registry and active account resolution.

```rust
pub struct AccountManager {
    storage: Arc<Mutex<JsonFileStorage>>,
}
```

#### Method: `new`

Create a new AccountManager instance.

```rust
pub fn new(storage: Arc<Mutex<JsonFileStorage>>) -> Self
```

**Parameters:**
- `storage` - Shared reference to the JSON file storage

**Example:**

```rust
use bw_core::services::storage::{AccountManager, JsonFileStorage};
use std::sync::Arc;
use tokio::sync::Mutex;

let storage = Arc::new(Mutex::new(JsonFileStorage::new(None)?));
let manager = AccountManager::new(storage);
```

#### Method: `get_active_user_id`

Get the currently active user ID.

```rust
pub async fn get_active_user_id(&self) -> Result<Option<String>>
```

**Returns:**
- `Ok(Some(id))` - Active user ID
- `Ok(None)` - No user is logged in
- `Err(_)` - Storage error

**Example:**

```rust
let user_id = manager.get_active_user_id().await?;
if let Some(id) = user_id {
    println!("Active user: {}", id);
} else {
    println!("No active user");
}
```

#### Method: `set_active_user_id`

Set the active user ID.

```rust
pub async fn set_active_user_id(&self, user_id: &str) -> Result<()>
```

**Parameters:**
- `user_id` - The user ID to set as active

**Example:**

```rust
manager.set_active_user_id("abc-123-def").await?;
```

#### Method: `clear_active_account`

Clear the active account (used during logout).

```rust
pub async fn clear_active_account(&self) -> Result<()>
```

Sets `global_account_activeAccountId` to `null` but preserves the account in the registry.

**Example:**

```rust
// During logout
manager.clear_active_account().await?;
```

#### Method: `register_account`

Add or update an account in the registry.

```rust
pub async fn register_account(&self, user_id: &str, email: &str) -> Result<()>
```

**Parameters:**
- `user_id` - Unique identifier for the account
- `email` - User's email address

**Example:**

```rust
// During login
manager.register_account("abc-123", "user@example.com").await?;
```

#### Method: `get_account`

Get account info for a specific user.

```rust
pub async fn get_account(&self, user_id: &str) -> Result<Option<AccountInfo>>
```

**Parameters:**
- `user_id` - The user ID to look up

**Returns:** Account info if found

**Example:**

```rust
if let Some(account) = manager.get_account("abc-123").await? {
    println!("Email: {}", account.email);
}
```

#### Method: `get_all_accounts`

Get all registered accounts.

```rust
pub async fn get_all_accounts(&self) -> Result<HashMap<String, AccountInfo>>
```

**Returns:** Map of user ID to account info

**Example:**

```rust
let accounts = manager.get_all_accounts().await?;
for (id, info) in accounts {
    println!("{}: {}", id, info.email);
}
```

#### Method: `remove_account`

Remove an account from the registry.

```rust
pub async fn remove_account(&self, user_id: &str) -> Result<bool>
```

**Parameters:**
- `user_id` - The user ID to remove

**Returns:** `true` if the account was removed, `false` if not found

**Example:**

```rust
let removed = manager.remove_account("abc-123").await?;
if removed {
    println!("Account removed");
}
```

#### Method: `is_logged_in`

Check if there's an active session.

```rust
pub async fn is_logged_in(&self) -> Result<bool>
```

Checks:
1. There is an active account ID
2. The active account has a non-null access token

**Returns:** `true` if logged in with valid tokens

**Example:**

```rust
if manager.is_logged_in().await? {
    // Proceed with vault operations
} else {
    println!("Please login first");
}
```

---

## Module: `bw_core::services::storage::errors`

Storage-related error types.

### Enum Variant: `UnsupportedStateVersion`

Returned when data.json has an incompatible state version.

```rust
#[error("Unsupported state version {found}. This CLI requires version {required}+. Run the TypeScript CLI to upgrade your data.")]
UnsupportedStateVersion {
    found: u64,
    required: u64,
}
```

### Enum Variant: `NoActiveAccount`

Returned when an operation requires an active account but none is set.

```rust
#[error("No active account. Please log in first.")]
NoActiveAccount,
```

### Enum Variant: `AccountNotFound`

Returned when a requested account doesn't exist in the registry.

```rust
#[error("Account not found: {user_id}")]
AccountNotFound {
    user_id: String,
}
```

---

## Usage Patterns

### Reading User Data

```rust
use bw_core::services::storage::{StorageKey, AccountManager};

async fn get_user_token(manager: &AccountManager, storage: &Storage) -> Result<Option<String>> {
    // Get active user
    let user_id = manager.get_active_user_id().await?
        .ok_or(StorageError::NoActiveAccount)?;

    // Build namespaced key
    let key = StorageKey::UserAccessToken.format(Some(&user_id));

    // Read from storage
    storage.get(&key)
}
```

### Storing User Data

```rust
async fn store_user_token(
    manager: &AccountManager,
    storage: &mut Storage,
    token: &str,
) -> Result<()> {
    let user_id = manager.get_active_user_id().await?
        .ok_or(StorageError::NoActiveAccount)?;

    let key = StorageKey::UserAccessToken.format(Some(&user_id));
    storage.set(&key, &token.to_string()).await
}
```

### Login Flow

```rust
async fn login(
    manager: &AccountManager,
    storage: &mut Storage,
    user_id: &str,
    email: &str,
    tokens: Tokens,
) -> Result<()> {
    // 1. Ensure state version
    storage.ensure_state_version().await?;

    // 2. Register account
    manager.register_account(user_id, email).await?;

    // 3. Set as active
    manager.set_active_user_id(user_id).await?;

    // 4. Store tokens
    let token_key = StorageKey::UserAccessToken.format(Some(user_id));
    storage.set(&token_key, &tokens.access_token).await?;

    Ok(())
}
```

### Logout Flow

```rust
async fn logout(manager: &AccountManager, storage: &mut Storage) -> Result<()> {
    let user_id = manager.get_active_user_id().await?
        .ok_or(StorageError::NoActiveAccount)?;

    // Set tokens to null (not removed)
    let token_key = StorageKey::UserAccessToken.format(Some(&user_id));
    storage.set(&token_key, &serde_json::Value::Null).await?;

    // Clear active account (but keep in registry)
    manager.clear_active_account().await?;

    Ok(())
}
```
