---
enhancement: 10-compatibility-fix
agent: documenter
task_id: task_1765300142_52009
timestamp: 2025-12-09T20:00:00Z
status: DOCUMENTATION_COMPLETE
---

# Documentation Summary: TypeScript CLI Compatibility Fix

## Overview

This document summarizes the documentation created or updated for Enhancement 10: TypeScript CLI Compatibility Fix. The enhancement enables the Rust CLI to share the same `data.json` storage file with the official TypeScript Bitwarden CLI, allowing users to seamlessly switch between the two implementations.

## Documentation Created

### 1. API Documentation

New modules have been documented with Rust doc comments following project conventions:

#### `StorageKey` Enum (`crates/bw-core/src/services/storage/keys.rs`)

**Purpose**: Type-safe generation of storage key patterns for TypeScript CLI compatibility.

**Key Types**:
| Key Type | Example Output | Description |
|----------|---------------|-------------|
| `StateVersion` | `stateVersion` | Storage format version (currently 73) |
| `GlobalAppId` | `global_applicationId_appId` | Application UUID |
| `GlobalAccounts` | `global_account_accounts` | Account registry map |
| `GlobalActiveAccountId` | `global_account_activeAccountId` | Active user ID |
| `UserAccessToken` | `user_{id}_token_accessToken` | OAuth access token |
| `UserRefreshToken` | `user_{id}_token_refreshToken` | OAuth refresh token |
| `UserPrivateKey` | `user_{id}_crypto_privateKey` | Encrypted RSA private key |
| `DeviceId` | `global_deviceId` | Device identifier |

**Usage**:
```rust
// Global keys - no user ID needed
let key = StorageKey::StateVersion.format(None);
assert_eq!(key, "stateVersion");

// User-namespaced keys - require user ID
let key = StorageKey::UserAccessToken.format(Some("abc-123"));
assert_eq!(key, "user_abc-123_token_accessToken");
```

#### `AccountManager` Struct (`crates/bw-core/src/services/storage/account.rs`)

**Purpose**: Manages the account registry and active account resolution for multi-account support.

**Public API**:

| Method | Parameters | Returns | Description |
|--------|------------|---------|-------------|
| `new` | `storage: Arc<Mutex<JsonFileStorage>>` | `Self` | Create new manager |
| `get_active_user_id` | - | `Result<Option<String>>` | Get active user ID |
| `set_active_user_id` | `user_id: &str` | `Result<()>` | Set active account |
| `clear_active_account` | - | `Result<()>` | Clear on logout |
| `register_account` | `user_id: &str, email: &str` | `Result<()>` | Add to registry |
| `get_account` | `user_id: &str` | `Result<Option<AccountInfo>>` | Get account info |
| `get_all_accounts` | - | `Result<HashMap<String, AccountInfo>>` | List all accounts |
| `remove_account` | `user_id: &str` | `Result<bool>` | Remove from registry |
| `is_logged_in` | - | `Result<bool>` | Check login state |

### 2. Error Documentation

New error types added to `crates/bw-core/src/services/storage/errors.rs`:

| Error | When Raised | User Message |
|-------|-------------|--------------|
| `UnsupportedStateVersion` | data.json has version < 73 | "Unsupported state version {found}. This CLI requires version {required}+. Run the TypeScript CLI to upgrade your data." |
| `NoActiveAccount` | Operation requires active account but none set | "No active account. Please log in first." |
| `AccountNotFound` | Requested account not in registry | "Account not found: {user_id}" |

### 3. Model Documentation

#### Cipher Model Updates (`crates/bw-core/src/models/vault/cipher.rs`)

New fields added for API compatibility:

| Field | Type | Description |
|-------|------|-------------|
| `object` | `Option<String>` | Object type indicator (e.g., "cipher") |
| `archived_date` | `Option<String>` | ISO 8601 timestamp when archived |

#### SyncResponse Updates (`crates/bw-core/src/models/vault/sync_response.rs`)

| Field | Type | Description |
|-------|------|-------------|
| `extra` | `HashMap<String, serde_json::Value>` | Captures unknown API fields for forward compatibility |

## User-Facing Documentation

### Cross-CLI Compatibility Guide

**How It Works**:

1. The Rust CLI now uses the same storage format as the TypeScript CLI
2. Both CLIs read/write to `~/Library/Application Support/Bitwarden CLI/data.json` (macOS)
3. You can login with either CLI and use the session with the other

**Prerequisites**:
- TypeScript CLI data.json must be at state version 73 or higher
- If you have an older version, run `bw login` with the TypeScript CLI to upgrade

**Switching Between CLIs**:

```bash
# Login with TypeScript CLI
npx @bitwarden/cli login

# Use Rust CLI for subsequent operations
./bw sync
./bw list items

# Or vice versa - login with Rust CLI
./bw login

# TypeScript CLI can read the session
npx @bitwarden/cli list items
```

**Logout Behavior**:
- Logout sets tokens to `null` (not removed)
- Account stays in registry for quick re-login
- This matches TypeScript CLI behavior

### Error Messages

| Scenario | Error Message | Resolution |
|----------|---------------|------------|
| Old data.json | "Unsupported state version 50. This CLI requires version 73+." | Run TypeScript CLI `bw login` to upgrade |
| Not logged in | "No active account. Please log in first." | Run `bw login` |
| Session expired | "Session expired or invalid" | Run `bw unlock` or `bw login` |

## Technical Architecture Documentation

### Storage Format

The TypeScript CLI uses namespaced keys in data.json:

```json
{
  "stateVersion": 73,
  "global_applicationId_appId": "{uuid}",
  "global_account_accounts": {
    "{userId}": {
      "email": "user@example.com",
      "emailVerified": true
    }
  },
  "global_account_activeAccountId": "{userId}",
  "user_{userId}_token_accessToken": "...",
  "user_{userId}_token_refreshToken": "...",
  "user_{userId}_crypto_privateKey": "...",
  "user_{userId}_crypto_userKey": "..."
}
```

### Key Design Decisions

1. **State Version 73+**: Only supports current format to avoid migration complexity
2. **Read Both, Write New**: Falls back to legacy flat keys for device ID migration
3. **Null vs Remove**: Tokens set to `null` on logout (not deleted) per TypeScript CLI behavior
4. **Unknown Key Preservation**: All unknown keys preserved for forward compatibility

## Documentation Quality Checklist

- [x] All new modules have doc comments
- [x] All public APIs documented with parameters and return types
- [x] Error conditions documented
- [x] Usage examples provided in code
- [x] Cross-CLI workflow documented
- [x] Storage format documented
- [x] Design decisions explained

## Files Created/Updated

| File | Action | Description |
|------|--------|-------------|
| `keys.rs` | New | StorageKey enum with full doc comments |
| `account.rs` | New | AccountManager with method documentation |
| `errors.rs` | Updated | New error types documented |
| `cipher.rs` | Updated | New fields documented |
| `sync_response.rs` | Updated | Extra field documented |

## Recommended README Updates

The project README should be updated to reflect the new compatibility:

```markdown
## TypeScript CLI Compatibility

This Rust CLI is compatible with the official TypeScript Bitwarden CLI (v2024.x+).
Both CLIs can share the same data.json file, allowing seamless switching between them.

**Requirements:**
- State version 73+ (current TypeScript CLI format)
- macOS: `~/Library/Application Support/Bitwarden CLI/data.json`
```

## Future Documentation Needs

1. **Multi-Account Guide**: When multi-account commands are exposed to CLI
2. **Migration Guide**: If legacy Rust CLI users need to migrate data
3. **Troubleshooting Guide**: Expanded error scenarios and resolutions
4. **API Reference**: Generated via `cargo doc` when public API stabilizes

## Conclusion

All implementation from Enhancement 10 has been documented:
- Module-level doc comments in new Rust files
- API documentation for public structs and methods
- Error documentation with resolution guidance
- User-facing compatibility guide
- Architecture and design decision documentation

The documentation follows project conventions (Rust doc comments, markdown in enhancements directory) and is ready for integration.

**Status: DOCUMENTATION_COMPLETE**
