# TypeScript CLI Storage Format Analysis

## Overview

This document provides detailed analysis of the TypeScript CLI storage format based on examination of an actual `data.json` file from a macOS installation.

## File Location

**Path**: `~/Library/Application Support/Bitwarden CLI/data.json`

## State Version

**Current**: 73 (as of December 2025)

Note: The enhancement document referenced version 72, but the actual file shows version 73. This indicates the TypeScript CLI has undergone a state migration since the enhancement was written.

## Key Categories

### Global Keys

Keys that apply across all accounts:

| Key | Type | Purpose |
|-----|------|---------|
| `stateVersion` | number | Storage format version |
| `global_applicationId_appId` | string (UUID) | Application instance identifier |
| `global_account_accounts` | object | Registry of all known accounts |
| `global_account_activeAccountId` | string/null | Currently active user ID |
| `global_account_activity` | object | Account activity tracking |
| `global_clearEvent_logout` | array | State to clear on logout |
| `global_clearEvent_lock` | array | State to clear on lock |
| `global_tokenDiskLocal_emailTwoFactorTokenRecord` | object | 2FA token cache |
| `global_config_byServer` | object | Per-server configuration cache |

### User-Namespaced Keys

Keys specific to individual user accounts. Format: `user_{userId}_{category}_{key}`

| Category | Key | Type | Purpose |
|----------|-----|------|---------|
| token | accessToken | string/null | OAuth access token |
| token | refreshToken | string/null | OAuth refresh token |
| token | apiKeyClientId | string/null | API key client ID |
| token | apiKeyClientSecret | string/null | API key client secret |
| crypto | privateKey | string/null | Encrypted RSA private key |
| crypto | providerKeys | object/null | Provider encryption keys |
| crypto | organizationKeys | object/null | Organization encryption keys |
| crypto | everHadUserKey | boolean/null | Flag for user key history |
| crypto | userSigningKey | string/null | User signing key |
| masterPassword | masterKeyHash | string/null | Master password hash |
| environment | environment | object | Environment URL settings |
| vaultTimeoutSettings | vaultTimeout | string | Timeout duration ("never", minutes) |
| vaultTimeoutSettings | vaultTimeoutAction | string | Action on timeout ("lock", "logout") |
| userDecryptionOptions | decryptionOptions | object | Decryption capability flags |
| avatar | avatarColor | string/null | User avatar color |
| keyConnector | convertAccountToKeyConnector | boolean/null | Key connector migration flag |
| collection | collections | array/null | User's collections |
| pinUnlock | pinKeyEncryptedUserKeyPersistent | string/null | PIN unlock key |
| pinUnlock | userKeyEncryptedPin | string/null | Encrypted PIN |
| pinUnlock | oldPinKeyEncryptedMasterKey | string/null | Legacy PIN encryption |

### Legacy Bare User ID Keys

The TypeScript CLI also uses bare user IDs as keys for some data:

```json
"{userId}": {
  "keys": {
    "cryptoSymmetricKey": {}
  },
  "profile": {}
}
```

This appears to be a legacy format that coexists with the namespaced format.

## Account Registry Structure

```json
"global_account_accounts": {
  "{userId-1}": {
    "email": "",
    "emailVerified": false
  },
  "{userId-2}": {
    "email": "",
    "emailVerified": false
  }
}
```

Observations:
- Emails are empty strings in the observed file (possibly cleared on logout)
- emailVerified is false for all accounts
- Multiple accounts can exist simultaneously

## Server Configuration Cache

The `global_config_byServer` key contains detailed server configuration:

```json
"global_config_byServer": {
  "https://api.bitwarden.com": {
    "featureStates": { /* many feature flags */ },
    "version": "2025.11.1",
    "gitHash": "62d63716",
    "server": null,
    "utcDate": "2025-12-09T16:00:25.392Z",
    "environment": {
      "cloudRegion": "US",
      "vault": "https://vault.bitwarden.com",
      "api": "https://api.bitwarden.com",
      "identity": "https://identity.bitwarden.com",
      "notifications": "https://notifications.bitwarden.com",
      "sso": "https://sso.bitwarden.com"
    },
    "push": {
      "pushTechnology": 1,
      "vapidPublicKey": "..."
    },
    "settings": {
      "disableUserRegistration": false
    }
  }
}
```

This cache is keyed by API server URL, allowing multi-server support.

## Clear Event Definitions

The TypeScript CLI defines what state to clear on logout and lock:

### Logout Event (47 items)
Clears both disk and memory state including:
- KDF config
- Master password data
- Crypto keys
- Vault data (ciphers, folders, collections)
- Sync state
- Policies
- Event collection

### Lock Event (9 items)
Clears memory-only state:
- Master key
- User key
- Decrypted ciphers
- Search indexes

## Token Observations

**Critical Finding**: In the observed data.json, all token fields are `null`:

```json
"user_{userId}_token_accessToken": null,
"user_{userId}_token_refreshToken": "F4BCF2EC7F...-1"
```

Observations:
1. Access tokens are `null` for all users
2. Some users have refresh tokens, others don't
3. Refresh tokens follow format: `{HEX_STRING}-{VERSION}`

This suggests:
- Access tokens may be stored in memory only (not persisted)
- Or stored in system keychain
- Refresh tokens are persisted but may also use keychain

## Recommendations for Rust CLI

### Must Implement
1. Namespaced key format: `user_{userId}_{category}_{key}`
2. Global keys: `global_account_*`, `stateVersion`
3. Active account resolution via `global_account_activeAccountId`

### Should Implement
1. Server configuration cache (for multi-server support)
2. Clear event definitions (for proper logout/lock behavior)

### May Defer
1. Full clear event handling (complex, many states)
2. Feature flag support (not critical for CLI)
3. Multi-account UI (single account MVP is acceptable)

### Preserve on Write
1. All `global_clearEvent_*` definitions
2. Server configuration cache
3. Other user accounts' data
4. Any unknown keys

## Schema Summary

```
data.json
├── stateVersion: number
├── global_applicationId_appId: string
├── global_account_accounts: { [userId]: AccountInfo }
├── global_account_activeAccountId: string | null
├── global_account_activity: {}
├── global_clearEvent_logout: ClearEventDef[]
├── global_clearEvent_lock: ClearEventDef[]
├── global_tokenDiskLocal_emailTwoFactorTokenRecord: {}
├── global_config_byServer: { [serverUrl]: ServerConfig }
├── user_{userId}_token_accessToken: string | null
├── user_{userId}_token_refreshToken: string | null
├── user_{userId}_crypto_*: various
├── user_{userId}_environment_environment: {}
├── user_{userId}_vaultTimeoutSettings_*: various
├── user_{userId}_masterPassword_masterKeyHash: string | null
├── user_{userId}_userDecryptionOptions_decryptionOptions: {}
├── user_{userId}_avatar_avatarColor: string | null
├── user_{userId}_collection_collections: array | null
├── user_{userId}_pinUnlock_*: various
└── {userId}: { keys: {}, profile: {} }  // Legacy format
```

## Next Steps

1. **Investigate Token Storage**: Determine if tokens use keychain integration
2. **Test with Logged-In State**: Capture data.json when actively logged in
3. **Review TS CLI Source**: Examine `token.service.ts` and storage services
