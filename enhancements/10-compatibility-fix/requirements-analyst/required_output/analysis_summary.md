---
enhancement: 10-compatibility-fix
agent: requirements-analyst
task_id: task_1765298523_35334
timestamp: 2025-12-09T17:15:00Z
status: READY_FOR_ARCHITECTURE
---

# Requirements Analysis: TypeScript CLI Compatibility Fix

## Executive Summary

This enhancement addresses a **critical compatibility issue** that prevents the Rust CLI from functioning as a drop-in replacement for the TypeScript CLI. The root cause is a fundamental mismatch in storage format: the Rust CLI uses a flat key-value structure while the TypeScript CLI uses a namespaced key format with user IDs.

**Classification**: Critical bug fix (not new feature)
**Priority**: Highest - blocks all vault operations (sync, list, get)
**Impact**: Without this fix, users cannot share sessions between CLIs

## Problem Analysis

### Root Cause Identification

Based on codebase exploration and analysis of an actual TypeScript CLI data.json file, the following issues were identified:

1. **Storage Format Mismatch**
   - Current Rust CLI: Flat keys (e.g., `accessToken`, `refreshToken`)
   - TypeScript CLI: Namespaced keys (e.g., `user_{userId}_token_accessToken`)

2. **State Version**
   - Enhancement document stated version 72
   - **Finding**: Actual TypeScript CLI uses state version **73** (verified from real data.json)
   - This discrepancy needs verification with latest TypeScript CLI releases

3. **Multi-Account Architecture**
   - TypeScript CLI stores multiple user accounts in `global_account_accounts`
   - Active account determined by `global_account_activeAccountId`
   - Each user has namespaced keys: `user_{userId}_{category}_{key}`

### What Was Missed in Previous Enhancements

| Enhancement | What Was Specified | What Was Actually Needed |
|-------------|-------------------|--------------------------|
| 02-storage-layer | "Flat key-value structure" | Namespaced key format with user IDs |
| 03-api-client | Basic response models | Complete field coverage for API responses |
| 04-auth-commands | Token storage | User-namespaced token storage |

## Functional Requirements

### FR-1: Storage Key Format Compatibility

**Description**: Read and write storage keys using TypeScript CLI's namespaced format

**Key Format Patterns** (verified from actual data.json):
| Pattern | Example | Purpose |
|---------|---------|---------|
| `stateVersion` | `73` | Storage format version |
| `global_account_accounts` | `{userId: {email, emailVerified}}` | Account registry |
| `global_account_activeAccountId` | `{userId}` or `null` | Currently active account |
| `global_applicationId_appId` | `{uuid}` | Application instance ID |
| `user_{userId}_token_accessToken` | JWT or `null` | Access token (nullable when logged out) |
| `user_{userId}_token_refreshToken` | Token string or `null` | Refresh token |
| `user_{userId}_crypto_privateKey` | EncString or `null` | Encrypted private key |
| `user_{userId}_environment_environment` | `{}` | Environment settings |
| `user_{userId}_vaultTimeoutSettings_*` | Various | Vault timeout configuration |
| `user_{userId}_masterPassword_masterKeyHash` | Hash or `null` | Master key hash |
| `{userId}` (bare) | `{keys, profile}` | Legacy format compatibility |

**Acceptance Criteria**:
- [ ] Can parse all key formats found in actual TypeScript CLI data.json
- [ ] Can write keys in same format
- [ ] Preserves unknown keys when writing (forward compatibility)

### FR-2: Active Account Resolution

**Description**: Determine which user account is currently active

**Logic**:
1. Read `global_account_activeAccountId`
2. If null, no account is active (logged out state)
3. If set, use that userId to read user-namespaced keys

**Acceptance Criteria**:
- [ ] Returns correct active user ID when one is set
- [ ] Handles null/missing activeAccountId gracefully
- [ ] Handles multiple accounts in storage

### FR-3: Token Storage Format

**Description**: Store and retrieve tokens using correct namespaced keys

**Key Mapping**:
| Current Rust Key | Required TypeScript Key |
|-----------------|------------------------|
| `__PROTECTED__accessToken` | `user_{userId}_token_accessToken` |
| `__PROTECTED__refreshToken` | `user_{userId}_token_refreshToken` |

**Observations from Real Data**:
- Tokens appear as `null` when user is logged out (not absent)
- TypeScript CLI may use secure storage/keychain for actual token values
- Need to investigate: When logged in, are tokens stored in data.json or keychain?

**Acceptance Criteria**:
- [ ] Tokens stored in user-namespaced keys
- [ ] Can read tokens from existing TypeScript CLI data.json
- [ ] Login preserves other user accounts in storage

### FR-4: State Version Handling

**Description**: Maintain compatibility with TypeScript CLI state versions

**Requirements**:
- Support state version 73 (current as of 2025-12)
- Preserve `stateVersion` field on writes
- Clear error message if state version is unsupported

**Acceptance Criteria**:
- [ ] Preserves stateVersion field
- [ ] Errors clearly if version < 70 (too old)
- [ ] Works with version 73+

### FR-5: API Response Model Completeness

**Description**: Ensure Cipher and related models include all API response fields

**Current Coverage** (from cipher.rs analysis):
The Cipher model appears comprehensive with fields for:
- Core fields: id, organizationId, folderId, type, name, notes
- Flags: favorite, edit, viewPassword
- Permissions object
- All cipher types: Login, SecureNote, Card, Identity, SshKey
- Attachments, fields, passwordHistory
- Additional: organizationUseTotp, reprompt, key

**Potential Gaps to Verify**:
- `object` field (API type indicator, e.g., "cipher")
- `archivedDate` (distinct from deletedDate?)
- Full FIDO2 credential field coverage

**Acceptance Criteria**:
- [ ] Can deserialize actual API sync response without errors
- [ ] All required fields have `#[serde(default)]` for optional handling
- [ ] Unknown fields are ignored (forward compatibility)

### FR-6: Cross-CLI Session Sharing

**Description**: Allow seamless switching between TypeScript and Rust CLIs

**Acceptance Criteria**:
- [ ] Login with TypeScript CLI, use Rust CLI without re-login
- [ ] Login with Rust CLI, use TypeScript CLI without re-login
- [ ] Sync works using existing session
- [ ] List items works using existing session

## Non-Functional Requirements

### NFR-1: Backward Compatibility
- Must not break current login functionality
- Must handle both old (if present) and new storage formats during migration

### NFR-2: Data Integrity
- Preserve all existing TypeScript CLI data on write
- Never corrupt or lose user data
- Atomic writes to prevent partial updates

### NFR-3: Performance
- No degradation in storage read/write performance
- Storage operations should remain < 50ms

### NFR-4: Security
- Continue to handle tokens securely
- Maintain file permissions (0600 on data.json)
- No logging of sensitive values

## User Stories

### US-1: Shared Session (TypeScript First)
**As a** Bitwarden CLI user with existing TypeScript CLI session
**I want to** run Rust CLI commands without re-authenticating
**So that** I can test the Rust CLI with my existing data

**Acceptance Criteria**:
- Given: Logged in with TypeScript CLI
- When: Running `bw sync` with Rust CLI
- Then: Sync succeeds using existing tokens

### US-2: Shared Session (Rust First)
**As a** Bitwarden CLI user
**I want to** log in with Rust CLI and have TypeScript CLI recognize the session
**So that** both CLIs can be used interchangeably

**Acceptance Criteria**:
- Given: Logged in with Rust CLI
- When: Running `bw list items` with TypeScript CLI
- Then: Items are listed using Rust CLI's stored session

### US-3: Vault Operations
**As a** Bitwarden user
**I want to** sync my vault and list items
**So that** I can access my passwords

**Acceptance Criteria**:
- Given: Valid session exists
- When: Running `bw sync` then `bw list items`
- Then: All vault items are returned (3 test items: Login, Card, SecureNote)

## Open Questions & Recommendations

### OQ-1: Token Storage Location (CRITICAL)
**Question**: In the observed data.json, `user_{userId}_token_accessToken` is `null` for all users. Where are tokens actually stored when logged in?

**Possible Answers**:
1. Keychain/secure storage (macOS Keychain, Windows Credential Manager)
2. Memory only (lost on CLI exit)
3. Different file location

**Recommendation**: Investigate TypeScript CLI source code for `token.service.ts` and `secure-storage.service.ts`

### OQ-2: State Version Support Range
**Question**: Should we support state versions 70-73 or only 73+?

**Recommendation**: Support 73+ only for MVP. Error with helpful message if < 73, suggesting user run TypeScript CLI to upgrade.

### OQ-3: Write Strategy for Unknown Keys
**Question**: When Rust CLI writes to data.json, should it preserve keys it doesn't understand?

**Recommendation**: Yes, preserve all unknown keys. This provides forward compatibility and prevents data loss from TypeScript CLI features we don't implement.

### OQ-4: BW_SESSION Integration
**Question**: How does BW_SESSION relate to the namespaced token storage?

**Recommendation**: Investigate if BW_SESSION is used for encryption of stored values or as a separate session mechanism. Current Rust CLI uses it for encryption.

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Token location differs from data.json | High | Critical | Investigate TS CLI source before implementation |
| State version changes in future TS releases | Medium | Medium | Design for version flexibility |
| Unknown keys overwritten | Medium | High | Preserve-all-keys strategy |
| API response model gaps | Medium | Medium | Use `#[serde(deny_unknown_fields)]` in tests to catch gaps |
| Multi-account bugs | Low | High | Focus on single-account MVP first |

## Dependencies

| Enhancement | Dependency Type | What Needs to Change |
|-------------|----------------|---------------------|
| 02-storage-layer | Breaking | Key format changes |
| 03-api-client | Additive | Model field coverage |
| 04-auth-commands | Breaking | Token storage location |
| 09-sdk-integration | None | SDK handles decryption separately |

## Testing Requirements

### Unit Tests
- Storage key parsing/formatting
- Active account resolution
- Token read/write with namespaced keys
- Cipher model deserialization with all fields

### Integration Tests
- Full login -> sync -> list flow
- Cross-CLI compatibility (login with TS, use Rust)
- Token refresh with namespaced storage
- Multiple account handling

### Manual Test Scenarios
1. Login with TypeScript CLI, run `bw list items` with Rust CLI
2. Login with Rust CLI, verify data.json format matches TypeScript
3. Run sync, verify vault data stored correctly
4. List items and verify all test items appear

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Cross-CLI session sharing | 100% success | Both direction login/use tests pass |
| Sync operation success | 100% | Can sync with existing TS session |
| List items success | 100% | Returns all vault items |
| Data corruption incidents | 0 | No data loss when switching CLIs |

## Implementation Phases

### Phase 1: Storage Format (Blocking)
- Update storage layer to use namespaced keys
- Implement active account resolution
- Update token storage to namespaced format

### Phase 2: API Models (If needed)
- Verify Cipher model against actual API responses
- Add any missing fields with `#[serde(default)]`
- Test sync response parsing

### Phase 3: Testing & Validation
- Cross-CLI compatibility tests
- Regression tests for existing functionality
- Manual validation with real Bitwarden account

## Conclusion

This enhancement is a critical fix required before the Rust CLI can be considered a viable TypeScript CLI replacement. The core issue is well-understood: storage format incompatibility. The solution requires updating the storage layer to use namespaced keys matching the TypeScript CLI format.

**Key Finding**: The actual state version in use is 73 (not 72 as documented in the enhancement). This should be verified against multiple TypeScript CLI installations.

**Critical Investigation Needed**: The location of actual token values when logged in needs investigation before architecture begins. The tokens appearing as `null` in data.json suggests they may be stored in secure system storage (Keychain/Credential Manager).

---

**Status**: READY_FOR_ARCHITECTURE

This analysis provides sufficient detail for the architect to design the implementation. The main uncertainty is the token storage location, which should be investigated during the architecture phase.
