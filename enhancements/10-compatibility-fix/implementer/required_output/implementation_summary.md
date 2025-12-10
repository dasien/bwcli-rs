---
enhancement: 10-compatibility-fix
agent: implementer
task_id: task_1765299229_42399
timestamp: 2025-12-09T19:15:00Z
status: READY_FOR_TESTING
---

# Implementation Summary: TypeScript CLI Compatibility Fix

## Overview

This implementation addresses the critical compatibility issue that prevented the Rust CLI from functioning as a drop-in replacement for the TypeScript CLI. The core problem was a fundamental mismatch in storage format: the Rust CLI used a flat key-value structure while the TypeScript CLI uses a namespaced key format with user IDs.

## Changes Made

### New Files Created

| File | Purpose |
|------|---------|
| `crates/bw-core/src/services/storage/keys.rs` | StorageKey enum for type-safe key generation |
| `crates/bw-core/src/services/storage/account.rs` | AccountManager for account registry management |

### Modified Files

| File | Changes |
|------|---------|
| `crates/bw-core/src/services/storage/mod.rs` | Export new components (StorageKey, AccountManager, AccountInfo) |
| `crates/bw-core/src/services/storage/json_storage.rs` | Added state version validation and ensure_state_version method |
| `crates/bw-core/src/services/storage/errors.rs` | Added UnsupportedStateVersion, NoActiveAccount, AccountNotFound errors |
| `crates/bw-core/src/services/auth/auth_service.rs` | Updated to use namespaced keys and AccountManager |
| `crates/bw-core/src/services/auth/session_manager.rs` | Updated to use namespaced keys for device ID and login checks |
| `crates/bw-core/src/models/vault/cipher.rs` | Added `object` and `archived_date` fields |
| `crates/bw-core/src/models/vault/sync_response.rs` | Added `extra` field for forward compatibility |
| `crates/bw-core/src/services/vault/cipher_service.rs` | Updated Cipher initialization with new fields |

## Implementation Details

### 1. Storage Key System (`keys.rs`)

Implemented a type-safe `StorageKey` enum that generates TypeScript CLI compatible key patterns:

**Global Keys (no user ID):**
- `stateVersion` - Storage format version (73)
- `global_applicationId_appId` - Application UUID
- `global_account_accounts` - Account registry
- `global_account_activeAccountId` - Active user ID
- `global_deviceId` - Device identifier

**User-Namespaced Keys:**
- `user_{id}_token_accessToken` - OAuth access token
- `user_{id}_token_refreshToken` - OAuth refresh token
- `user_{id}_crypto_privateKey` - Encrypted RSA private key
- `user_{id}_crypto_userKey` - Encrypted user key
- `user_{id}_kdf_config` - KDF configuration

### 2. Account Manager (`account.rs`)

New component providing:
- `get_active_user_id()` - Resolve current user from storage
- `set_active_user_id()` - Set active account
- `register_account()` - Add account to global registry
- `get_account()` / `get_all_accounts()` - Query account info
- `clear_active_account()` - Clear on logout (preserves registry entry)
- `is_logged_in()` - Check for valid session

### 3. State Version Handling

- Validates state version on storage load
- Rejects version < 73 with clear error message
- Automatically sets version 73 for new storage files
- Allows future versions (forward compatible)

### 4. Auth Service Updates

**persist_auth_state():**
1. Ensures state version is set
2. Registers account in global registry
3. Sets active account ID
4. Stores tokens/keys with user-namespaced keys

**logout():**
1. Gets active user ID
2. Sets tokens to `null` (not removed) - matches TypeScript behavior
3. Clears active account ID
4. Preserves account in registry

**unlock():**
1. Uses AccountManager to get active user
2. Loads KDF config from namespaced key
3. Loads user key from namespaced key

### 5. Session Manager Updates

- Updated `is_logged_in()` to check namespaced keys first, fall back to legacy
- Added `get_access_token()` for active user token retrieval
- Updated `get_or_create_device_id()` to use new key format with legacy fallback

### 6. Model Updates

**Cipher:**
- Added `object: Option<String>` - Type indicator (e.g., "cipher")
- Added `archived_date: Option<String>` - Archive timestamp

**SyncResponse:**
- Added `extra: HashMap<String, serde_json::Value>` - Forward compatibility

## Key Design Decisions

### D1: Read Both, Write New
- On read: Try namespaced keys first, fall back to legacy flat keys
- On write: Always use namespaced keys
- First login migrates data automatically

### D2: Token Null vs Remove
- Tokens set to `null` on logout (not removed)
- Matches TypeScript CLI behavior
- Account stays in registry for quick re-login

### D3: State Version Requirement
- Only support version 73+ (current as of Dec 2025)
- Clear error message guides users to upgrade
- New storage files initialized at version 73

### D4: Unknown Key Preservation
- HashMap-based storage preserves all keys
- Only modifies keys we understand
- Forward compatible with TypeScript CLI changes

## Build Verification

```
cargo fmt --check          # Pass
cargo build --release      # Pass
cargo test --lib           # 101 tests passed
cargo clippy               # Warnings in pre-existing code only
```

## Test Results

All 101 existing tests pass. New tests added:
- `test_global_key_formatting` - Verify global key patterns
- `test_user_key_formatting` - Verify user-namespaced key patterns
- `test_requires_user_id` - Key type classification
- `test_user_key_without_user_id_panics` - Panic safety
- `test_no_active_user_initially` - Initial state
- `test_set_and_get_active_user` - Active user management
- `test_clear_active_account` - Logout behavior
- `test_register_and_get_account` - Account registry
- `test_get_all_accounts` - Multi-account support
- `test_remove_account` - Account removal
- `test_is_not_logged_in_without_active_account` - Login state

## Files Changed Summary

```
New:
  crates/bw-core/src/services/storage/keys.rs       (170 lines)
  crates/bw-core/src/services/storage/account.rs    (175 lines)

Modified:
  crates/bw-core/src/services/storage/mod.rs        (+6 lines)
  crates/bw-core/src/services/storage/json_storage.rs (+45 lines)
  crates/bw-core/src/services/storage/errors.rs     (+9 lines)
  crates/bw-core/src/services/auth/auth_service.rs  (~100 lines changed)
  crates/bw-core/src/services/auth/session_manager.rs (+50 lines)
  crates/bw-core/src/models/vault/cipher.rs         (+9 lines)
  crates/bw-core/src/models/vault/sync_response.rs  (+6 lines)
  crates/bw-core/src/services/vault/cipher_service.rs (+2 lines)
```

## Known Limitations

1. **Legacy Migration**: First login with namespaced storage will write new format. Legacy data.json files from old Rust CLI will need re-login.

2. **Token Location**: Implementation assumes tokens in data.json. If TypeScript CLI uses keychain for access tokens, sync will fail with clear error.

3. **Multi-Account**: Infrastructure exists but not exposed to CLI commands yet.

## Recommended Testing

1. **Cross-CLI Login Test**:
   - Login with TypeScript CLI
   - Run `bw sync` with Rust CLI
   - Verify vault data accessible

2. **Rust CLI Login Test**:
   - Login with Rust CLI
   - Verify data.json has namespaced keys
   - Run `bw list items` with TypeScript CLI (if possible)

3. **Logout/Re-login Test**:
   - Login with Rust CLI
   - Logout
   - Verify tokens are null (not removed)
   - Login again
   - Verify works correctly

4. **State Version Test**:
   - Create data.json with stateVersion: 50
   - Try to load - should error with clear message
   - Create fresh storage - should be version 73

## Next Steps

1. Integration testing with actual TypeScript CLI data.json
2. Manual verification of cross-CLI compatibility
3. Consider adding `bw status` command to show account/storage info
4. Future: Implement secure storage (keychain) integration
