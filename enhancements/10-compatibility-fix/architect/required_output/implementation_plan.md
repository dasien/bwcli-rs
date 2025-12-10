---
enhancement: 10-compatibility-fix
agent: architect
task_id: task_1765298783_38290
timestamp: 2025-12-09T18:30:00Z
status: READY_FOR_IMPLEMENTATION
---

# Implementation Plan: TypeScript CLI Compatibility Fix

## Executive Summary

This implementation plan addresses the critical compatibility issue preventing the Rust CLI from functioning as a drop-in replacement for the TypeScript CLI. The core problem is a fundamental mismatch in storage format: the Rust CLI uses a flat key-value structure while the TypeScript CLI uses a namespaced key format with user IDs.

**Scope**: Modify the storage layer to read/write TypeScript CLI's namespaced key format, update auth flows to use the correct key patterns, and ensure API models handle all response fields.

**Estimated Complexity**: Medium - primarily storage layer refactoring with targeted changes to auth flow.

## Architecture Overview

### Current State

```
┌─────────────────────────────────────────────────────────────┐
│                    Current Rust CLI                          │
├─────────────────────────────────────────────────────────────┤
│  data.json (Flat Structure)                                  │
│  ├── accessToken: "..."                                      │
│  ├── refreshToken: "..."                                     │
│  ├── userKey: "..."                                          │
│  ├── userProfile: {...}                                      │
│  ├── kdfConfig: {...}                                        │
│  └── deviceId: "..."                                         │
└─────────────────────────────────────────────────────────────┘
```

### Target State

```
┌─────────────────────────────────────────────────────────────┐
│                  TypeScript CLI Compatible                   │
├─────────────────────────────────────────────────────────────┤
│  data.json (Namespaced Structure)                            │
│  ├── stateVersion: 73                                        │
│  ├── global_applicationId_appId: "{uuid}"                    │
│  ├── global_account_accounts: { "{userId}": {...} }          │
│  ├── global_account_activeAccountId: "{userId}" | null       │
│  ├── user_{userId}_token_accessToken: "..." | null           │
│  ├── user_{userId}_token_refreshToken: "..." | null          │
│  ├── user_{userId}_crypto_privateKey: "..." | null           │
│  ├── user_{userId}_environment_environment: {}               │
│  ├── user_{userId}_masterPassword_masterKeyHash: "..."       │
│  ├── user_{userId}_vaultTimeoutSettings_vaultTimeout: "..."  │
│  └── [preserved unknown keys]                                │
└─────────────────────────────────────────────────────────────┘
```

### Component Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Storage Layer                             │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌─────────────────┐     ┌─────────────────────────────┐    │
│  │  StorageKey     │────▶│  NamespacedStorage          │    │
│  │  (enum)         │     │  (trait implementation)      │    │
│  └─────────────────┘     └─────────────────────────────┘    │
│          │                          │                        │
│          │                          ▼                        │
│          │               ┌─────────────────────────────┐    │
│          │               │  JsonFileStorage            │    │
│          │               │  (updated impl)             │    │
│          │               └─────────────────────────────┘    │
│          │                          │                        │
│          ▼                          ▼                        │
│  ┌─────────────────┐     ┌─────────────────────────────┐    │
│  │  ActiveAccount  │────▶│  data.json                  │    │
│  │  Resolution     │     │  (preserves unknown keys)   │    │
│  └─────────────────┘     └─────────────────────────────┘    │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

## Design Decisions

### D1: Storage Key Abstraction

**Decision**: Introduce a `StorageKey` enum to represent all known key patterns, with methods to format keys for the active user.

**Rationale**:
- Type-safe key generation prevents typos
- Centralizes key format knowledge
- Enables easy testing of key formatting

**Implementation**:
```rust
/// Storage key patterns for TypeScript CLI compatibility
pub enum StorageKey {
    // Global keys (no user ID)
    StateVersion,
    GlobalAppId,
    GlobalAccounts,
    GlobalActiveAccountId,

    // User-namespaced keys (require user ID)
    UserAccessToken,
    UserRefreshToken,
    UserPrivateKey,
    UserMasterKeyHash,
    UserEnvironment,
    UserVaultTimeout,
    UserVaultTimeoutAction,

    // Legacy flat keys (for migration/fallback)
    LegacyAccessToken,
    LegacyRefreshToken,
}

impl StorageKey {
    /// Format key for storage (with optional user_id for user-namespaced keys)
    pub fn format(&self, user_id: Option<&str>) -> String {
        match self {
            // Global keys
            Self::StateVersion => "stateVersion".to_string(),
            Self::GlobalAppId => "global_applicationId_appId".to_string(),
            Self::GlobalAccounts => "global_account_accounts".to_string(),
            Self::GlobalActiveAccountId => "global_account_activeAccountId".to_string(),

            // User-namespaced keys
            Self::UserAccessToken => format!("user_{}_token_accessToken", user_id.unwrap()),
            Self::UserRefreshToken => format!("user_{}_token_refreshToken", user_id.unwrap()),
            Self::UserPrivateKey => format!("user_{}_crypto_privateKey", user_id.unwrap()),
            Self::UserMasterKeyHash => format!("user_{}_masterPassword_masterKeyHash", user_id.unwrap()),
            Self::UserEnvironment => format!("user_{}_environment_environment", user_id.unwrap()),
            Self::UserVaultTimeout => format!("user_{}_vaultTimeoutSettings_vaultTimeout", user_id.unwrap()),
            Self::UserVaultTimeoutAction => format!("user_{}_vaultTimeoutSettings_vaultTimeoutAction", user_id.unwrap()),

            // Legacy keys
            Self::LegacyAccessToken => "accessToken".to_string(),
            Self::LegacyRefreshToken => "refreshToken".to_string(),
        }
    }
}
```

### D2: Account Management

**Decision**: Add an `AccountManager` component to handle active account resolution and account registry management.

**Rationale**:
- Separates account concerns from general storage
- Provides clean API for auth service
- Enables future multi-account support

**Implementation**:
```rust
/// Account information stored in global_account_accounts
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountInfo {
    pub email: String,
    pub email_verified: bool,
}

/// Manages account registry and active account resolution
pub struct AccountManager {
    storage: Arc<Mutex<JsonFileStorage>>,
}

impl AccountManager {
    /// Get the active user ID, if any
    pub async fn get_active_user_id(&self) -> Result<Option<String>> {
        let storage = self.storage.lock().await;
        storage.get::<String>(&StorageKey::GlobalActiveAccountId.format(None))
    }

    /// Set the active user ID
    pub async fn set_active_user_id(&self, user_id: &str) -> Result<()> {
        let mut storage = self.storage.lock().await;
        storage.set(
            &StorageKey::GlobalActiveAccountId.format(None),
            &user_id.to_string()
        ).await
    }

    /// Register an account in the global accounts registry
    pub async fn register_account(&self, user_id: &str, email: &str) -> Result<()> {
        let mut storage = self.storage.lock().await;

        // Get existing accounts or create new map
        let key = StorageKey::GlobalAccounts.format(None);
        let mut accounts: HashMap<String, AccountInfo> =
            storage.get(&key)?.unwrap_or_default();

        // Add/update this account
        accounts.insert(user_id.to_string(), AccountInfo {
            email: email.to_string(),
            email_verified: true,
        });

        storage.set(&key, &accounts).await
    }

    /// Clear active account on logout
    pub async fn clear_active_account(&self) -> Result<()> {
        let mut storage = self.storage.lock().await;
        storage.set(
            &StorageKey::GlobalActiveAccountId.format(None),
            &serde_json::Value::Null
        ).await
    }
}
```

### D3: State Version Management

**Decision**: Support state version 73+ only. Error with clear message if version is lower.

**Rationale**:
- State version 73 is current as of December 2025
- Supporting older versions would require implementing state migrations
- Users can upgrade their data.json by running the TypeScript CLI once

**Implementation**:
```rust
const SUPPORTED_STATE_VERSION: u64 = 73;

/// Validate state version on storage load
pub fn validate_state_version(version: Option<u64>) -> Result<()> {
    match version {
        Some(v) if v >= SUPPORTED_STATE_VERSION => Ok(()),
        Some(v) => Err(StorageError::UnsupportedStateVersion {
            found: v,
            required: SUPPORTED_STATE_VERSION,
        }.into()),
        None => {
            // No version = new storage, we'll write version 73
            Ok(())
        }
    }
}
```

### D4: Unknown Key Preservation

**Decision**: Preserve all unknown keys when writing to data.json.

**Rationale**:
- Prevents data loss when TypeScript CLI has features we don't implement
- Forward-compatible with future TypeScript CLI changes
- Allows safe interoperability

**Implementation**:
- Load entire JSON into `HashMap<String, Value>`
- Only modify keys we understand
- Serialize entire map back to file

### D5: Token Storage Strategy

**Decision**: Store tokens in data.json using user-namespaced keys (not secure storage for MVP).

**Rationale**:
- TypeScript CLI analysis shows tokens stored in data.json (refresh tokens persist, access tokens may be null)
- Secure storage (keychain) integration adds complexity and platform dependencies
- MVP goal is cross-CLI compatibility, not enhanced security

**Token Observations**:
- Access tokens appear as `null` in data.json when logged out
- Refresh tokens persist with format `{HEX}-{VERSION}`
- TypeScript CLI may use memory-only storage for active access tokens

**Future Enhancement**: Add keychain/secure storage as optional enhancement.

## Component Design

### C1: Updated Storage Layer

**File**: `crates/bw-core/src/services/storage/`

#### New Files:
- `keys.rs` - StorageKey enum and formatting
- `account.rs` - AccountManager component
- `compat.rs` - Migration and compatibility utilities

#### Modified Files:
- `json_storage.rs` - Updated to preserve unknown keys, add state version handling
- `mod.rs` - Export new components
- `traits.rs` - Add namespaced storage methods

**Interface Changes**:
```rust
// Enhanced Storage trait
#[async_trait]
pub trait Storage: Send + Sync {
    // Existing methods unchanged...

    /// Get value for active user (resolves user ID automatically)
    async fn get_for_active_user<T>(&self, key: StorageKey) -> Result<Option<T>>
    where
        T: for<'de> Deserialize<'de>;

    /// Set value for active user
    async fn set_for_active_user<T>(&mut self, key: StorageKey, value: &T) -> Result<()>
    where
        T: Serialize + Send + Sync;

    /// Get active user ID
    fn get_active_user_id(&self) -> Result<Option<String>>;

    /// Set active user ID
    async fn set_active_user_id(&mut self, user_id: &str) -> Result<()>;
}
```

### C2: Updated Auth Service

**File**: `crates/bw-core/src/services/auth/auth_service.rs`

**Changes**:
1. Use `StorageKey` enum for all storage operations
2. Register account in global accounts on login
3. Set active account ID on login
4. Clear active account on logout (preserve account registry entry)
5. Use namespaced keys for token storage

**Key Method Changes**:

```rust
impl AuthService {
    /// Updated persist_auth_state to use namespaced keys
    async fn persist_auth_state(
        &self,
        user_id: &str,
        email: &str,
        access_token: &str,
        refresh_token: &str,
        encrypted_user_key: Option<&str>,
        kdf_config: &KdfConfig,
    ) -> Result<(), AuthError> {
        let mut storage = self.storage.lock().await;

        // Ensure state version is set
        let version: Option<u64> = storage.get("stateVersion")?;
        if version.is_none() {
            storage.set("stateVersion", &73u64).await?;
        }

        // Register account in global accounts
        self.account_manager.register_account(user_id, email).await?;

        // Set as active account
        self.account_manager.set_active_user_id(user_id).await?;

        // Store tokens with user-namespaced keys
        storage.set(
            &StorageKey::UserAccessToken.format(Some(user_id)),
            &access_token.to_string()
        ).await?;

        storage.set(
            &StorageKey::UserRefreshToken.format(Some(user_id)),
            &refresh_token.to_string()
        ).await?;

        // Store encrypted user key
        if let Some(key) = encrypted_user_key {
            storage.set(
                &StorageKey::UserPrivateKey.format(Some(user_id)),
                &key.to_string()
            ).await?;
        }

        // Store KDF config (using environment namespace for compatibility)
        // TypeScript CLI stores this differently, but we need it for unlock
        storage.set(
            &format!("user_{}_kdf_config", user_id),
            &kdf_config
        ).await?;

        storage.flush().await?;
        Ok(())
    }

    /// Updated logout to use namespaced keys
    pub async fn logout(&self) -> Result<(), AuthError> {
        // Get active user ID before clearing
        let user_id = self.account_manager.get_active_user_id().await?
            .ok_or(AuthError::NotLoggedIn)?;

        let mut storage = self.storage.lock().await;

        // Clear user-specific tokens (set to null, don't remove key)
        storage.set(
            &StorageKey::UserAccessToken.format(Some(&user_id)),
            &serde_json::Value::Null
        ).await?;

        storage.set(
            &StorageKey::UserRefreshToken.format(Some(&user_id)),
            &serde_json::Value::Null
        ).await?;

        // Clear active account (but preserve in accounts registry)
        drop(storage);
        self.account_manager.clear_active_account().await?;

        Ok(())
    }
}
```

### C3: Updated Session Manager

**File**: `crates/bw-core/src/services/auth/session_manager.rs`

**Changes**:
1. Use namespaced keys for token checks
2. Add active user resolution

```rust
impl SessionManager {
    /// Check if user is logged in (has active account with valid tokens)
    pub async fn is_logged_in(&self) -> Result<bool> {
        let storage = self.storage.lock().await;

        // Get active user ID
        let user_id: Option<String> = storage.get(
            &StorageKey::GlobalActiveAccountId.format(None)
        )?;

        let Some(user_id) = user_id else {
            return Ok(false);
        };

        // Check for access token (may be null if logged out)
        let access_token: Option<Value> = storage.get(
            &StorageKey::UserAccessToken.format(Some(&user_id))
        )?;

        // Token is present and not null
        Ok(matches!(access_token, Some(Value::String(_))))
    }

    /// Get access token for active user
    pub async fn get_access_token(&self) -> Result<Option<String>> {
        let storage = self.storage.lock().await;

        let user_id: Option<String> = storage.get(
            &StorageKey::GlobalActiveAccountId.format(None)
        )?;

        let Some(user_id) = user_id else {
            return Ok(None);
        };

        storage.get(&StorageKey::UserAccessToken.format(Some(&user_id)))
    }
}
```

### C4: Cipher Model Updates

**File**: `crates/bw-core/src/models/vault/cipher.rs`

The current Cipher model appears comprehensive. Add `#[serde(default)]` to additional fields for robustness:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cipher {
    // ... existing fields ...

    /// Object type indicator (e.g., "cipher")
    #[serde(default)]
    pub object: Option<String>,

    /// Archived date (ISO 8601, if archived)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archived_date: Option<String>,
}
```

### C5: API Response Handling

**File**: `crates/bw-core/src/models/vault/sync_response.rs`

Update to use `#[serde(flatten)]` for unknown fields:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncResponse {
    // ... existing fields ...

    /// Capture unknown fields for forward compatibility
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}
```

## Data Flow

### Login Flow (Updated)

```
User Input (email, password)
         │
         ▼
┌─────────────────────┐
│  1. Prelogin        │  GET /identity/accounts/prelogin
│     (get KDF)       │
└─────────────────────┘
         │
         ▼
┌─────────────────────┐
│  2. Derive Master   │  bitwarden-crypto (KDF)
│     Key             │
└─────────────────────┘
         │
         ▼
┌─────────────────────┐
│  3. Authenticate    │  POST /identity/connect/token
│                     │
└─────────────────────┘
         │
         ▼
┌─────────────────────┐
│  4. Fetch Profile   │  GET /accounts/profile
│                     │
└─────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────┐
│  5. Persist State (Namespaced Keys)             │
│                                                  │
│  • Set stateVersion: 73 (if missing)            │
│  • Add to global_account_accounts               │
│  • Set global_account_activeAccountId           │
│  • Set user_{id}_token_accessToken              │
│  • Set user_{id}_token_refreshToken             │
│  • Set user_{id}_crypto_privateKey              │
│  • Set user_{id}_kdf_config                     │
└─────────────────────────────────────────────────┘
         │
         ▼
┌─────────────────────┐
│  6. Generate        │  Return BW_SESSION
│     Session Key     │
└─────────────────────┘
```

### Token Resolution Flow

```
Command Execution (sync, list, get, etc.)
         │
         ▼
┌─────────────────────────────────────────────────┐
│  1. Resolve Active User                          │
│     Read: global_account_activeAccountId         │
│     If null → Error: "Not logged in"             │
└─────────────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────┐
│  2. Get Access Token                             │
│     Read: user_{userId}_token_accessToken        │
│     If null → Error: "Session expired"           │
└─────────────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────┐
│  3. Check Token Expiry                           │
│     If expired → Refresh Token Flow              │
└─────────────────────────────────────────────────┘
         │
         ▼
┌─────────────────────┐
│  4. Execute API     │
│     Request         │
└─────────────────────┘
```

## File Changes Summary

### New Files

| File | Purpose |
|------|---------|
| `crates/bw-core/src/services/storage/keys.rs` | StorageKey enum and formatting |
| `crates/bw-core/src/services/storage/account.rs` | AccountManager component |
| `crates/bw-core/src/services/storage/compat.rs` | Migration and compatibility utilities |

### Modified Files

| File | Changes |
|------|---------|
| `crates/bw-core/src/services/storage/mod.rs` | Export new components |
| `crates/bw-core/src/services/storage/json_storage.rs` | Preserve unknown keys, state version handling |
| `crates/bw-core/src/services/storage/traits.rs` | Add namespaced storage methods |
| `crates/bw-core/src/services/auth/auth_service.rs` | Use namespaced keys, account management |
| `crates/bw-core/src/services/auth/session_manager.rs` | Use namespaced keys for token resolution |
| `crates/bw-core/src/models/vault/cipher.rs` | Add `object` and `archived_date` fields |
| `crates/bw-core/src/models/vault/sync_response.rs` | Add `#[serde(flatten)]` for unknown fields |

## Migration Strategy

### Approach: Read Both, Write New

1. **On Read**: Try namespaced keys first, fall back to legacy flat keys
2. **On Write**: Always use namespaced keys
3. **First Login**: Migrates data to new format automatically

```rust
impl JsonFileStorage {
    /// Get value with fallback to legacy keys
    pub async fn get_with_fallback<T>(
        &self,
        namespaced_key: &str,
        legacy_key: Option<&str>,
    ) -> Result<Option<T>>
    where
        T: for<'de> Deserialize<'de>,
    {
        // Try namespaced key first
        if let Some(value) = self.get(namespaced_key)? {
            return Ok(Some(value));
        }

        // Fall back to legacy key if provided
        if let Some(legacy) = legacy_key {
            return self.get(legacy);
        }

        Ok(None)
    }
}
```

## Error Handling

### New Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    // Existing errors...

    #[error("Unsupported state version {found}. This CLI requires version {required}+. Run the TypeScript CLI to upgrade your data.")]
    UnsupportedStateVersion {
        found: u64,
        required: u64,
    },

    #[error("No active account. Please log in first.")]
    NoActiveAccount,

    #[error("Account not found: {user_id}")]
    AccountNotFound {
        user_id: String,
    },
}
```

## Testing Strategy

### Unit Tests

1. **StorageKey Formatting**
   - Test all key patterns produce correct strings
   - Test user ID substitution

2. **AccountManager**
   - Test account registration
   - Test active account get/set
   - Test account clearing on logout

3. **State Version Validation**
   - Test version 73+ accepted
   - Test version < 73 rejected with clear error
   - Test missing version (new storage) accepted

4. **Unknown Key Preservation**
   - Write known keys, verify unknown keys preserved
   - Test round-trip with TypeScript CLI data

### Integration Tests

1. **Cross-CLI Compatibility**
   - Login with TypeScript CLI, read with Rust CLI
   - Login with Rust CLI, verify data.json format

2. **Full Flow Tests**
   - Login → Sync → List items
   - Login → Logout → Verify state cleared correctly

### Test Data

Create test fixture with real TypeScript CLI data.json structure:

```json
{
  "stateVersion": 73,
  "global_applicationId_appId": "test-app-id",
  "global_account_accounts": {
    "test-user-id": {
      "email": "test@example.com",
      "emailVerified": true
    }
  },
  "global_account_activeAccountId": "test-user-id",
  "user_test-user-id_token_accessToken": "test-access-token",
  "user_test-user-id_token_refreshToken": "test-refresh-token"
}
```

## Security Considerations

### S1: Token Handling
- Tokens stored in data.json (matching TypeScript CLI behavior)
- File permissions maintained at 0600
- No tokens logged (existing protection maintained)

### S2: State Version
- Prevents accidental use with incompatible older data formats
- Clear error message guides user to upgrade path

### S3: Logout Behavior
- Tokens set to `null` (not deleted) - matches TypeScript CLI
- Active account cleared
- Account remains in registry (enables re-login without re-entering email)

## Performance Considerations

### P1: Storage Operations
- All operations remain O(1) hash map lookups
- No performance regression expected
- Unknown key preservation adds minimal overhead

### P2: Active User Resolution
- Cached in memory after first resolution within session
- Single hash lookup per operation

## Implementation Order

### Phase 1: Storage Infrastructure (Priority: Highest)
1. Create `keys.rs` with StorageKey enum
2. Create `account.rs` with AccountManager
3. Update `json_storage.rs` for unknown key preservation
4. Add state version handling

### Phase 2: Auth Integration (Priority: High)
1. Update `auth_service.rs` to use namespaced keys
2. Update `session_manager.rs` for active user resolution
3. Add account registration on login
4. Update logout to clear properly

### Phase 3: Model Updates (Priority: Medium)
1. Add missing Cipher fields with defaults
2. Update SyncResponse for unknown field handling

### Phase 4: Testing & Validation (Priority: High)
1. Unit tests for all new components
2. Integration tests with real TypeScript CLI data
3. Manual cross-CLI compatibility testing

## Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Token location differs from data.json | Medium | High | TypeScript CLI analysis shows refresh tokens in data.json. If access tokens are in keychain, sync will fail but we'll get clear error. |
| State version changes in future TS releases | Medium | Medium | Preserve unknown keys, design for version flexibility |
| Unknown keys overwritten | Low | High | Preserve-all-keys strategy, comprehensive tests |
| Breaking existing Rust CLI users | Low | Medium | Migration strategy reads both old and new formats |

## Success Criteria

- [ ] Login with TypeScript CLI, run `bw sync` with Rust CLI - succeeds
- [ ] Login with Rust CLI, run `bw list items` with TypeScript CLI - succeeds
- [ ] data.json format matches TypeScript CLI structure
- [ ] No data corruption when both CLIs access same file
- [ ] All existing tests continue to pass
- [ ] State version validation provides clear error message

## Dependencies

| Dependency | Status | Notes |
|------------|--------|-------|
| Enhancement 02 (Storage Layer) | Exists | Will be modified |
| Enhancement 04 (Auth Commands) | Exists | Will be modified |
| Bitwarden SDK | Integrated | No changes needed |
| serde_json | Available | For unknown key handling |

## Appendix A: Key Format Reference

### Global Keys (No User ID)
| Key | Format | Description |
|-----|--------|-------------|
| State Version | `stateVersion` | Storage format version (73) |
| App ID | `global_applicationId_appId` | Application instance UUID |
| Accounts | `global_account_accounts` | Account registry object |
| Active Account | `global_account_activeAccountId` | Currently active user ID |

### User Keys (Require User ID)
| Category | Key Pattern | Description |
|----------|-------------|-------------|
| token | `user_{id}_token_accessToken` | OAuth access token |
| token | `user_{id}_token_refreshToken` | OAuth refresh token |
| crypto | `user_{id}_crypto_privateKey` | Encrypted RSA private key |
| masterPassword | `user_{id}_masterPassword_masterKeyHash` | Master password hash |
| environment | `user_{id}_environment_environment` | Environment settings |
| vaultTimeoutSettings | `user_{id}_vaultTimeoutSettings_*` | Timeout configuration |

## Appendix B: TypeScript CLI Source References

For implementation reference, study these TypeScript CLI files:
- `libs/common/src/platform/state/` - State definitions
- `libs/common/src/auth/services/token.service.ts` - Token management
- `apps/cli/src/platform/services/lowdb-storage.service.ts` - Storage implementation
- `libs/common/src/vault/models/response/cipher.response.ts` - Cipher model

---

**Status**: READY_FOR_IMPLEMENTATION
