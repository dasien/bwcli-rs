---
slug: compatibility-fix
status: NEW
created: 2025-12-09
author: Claude
priority: critical
---

# Enhancement: TypeScript CLI Compatibility Fix

## Overview
**Goal:** Fix storage format and API model compatibility so the Rust CLI can read/write the same data.json as the official TypeScript CLI, enabling true drop-in replacement functionality.

**User Story:**
As a Bitwarden CLI user, I want to seamlessly switch between the TypeScript and Rust CLIs so that I don't have to re-login or lose my existing session when using the Rust version.

## Context & Background

**Current State:**
- The Rust CLI currently uses a **flat key-value storage format** (e.g., `accessToken`, `refreshToken` at root level)
- The TypeScript CLI uses a **namespaced key format** with user IDs (e.g., `user_{userId}_token_accessToken`)
- Login works in the Rust CLI but stores tokens in an incompatible format
- Sync fails because API response models don't match the actual API response structure
- The two CLIs cannot share the same data.json file

**What Was Missed:**
Enhancements 01, 02, and 03 specified "compatibility with TypeScript CLI" but:
1. Nobody examined an actual TypeScript CLI `data.json` file to see the real format
2. The architect assumed "flat key-value structure" which is incorrect
3. API response models were created without verifying against actual API responses
4. The state migration system (72 versions) was not discovered or documented

**Technical Context:**
- Storage location: `~/Library/Application Support/Bitwarden CLI/data.json` (macOS)
- TypeScript CLI uses state version 72 with complex namespaced keys
- Multi-account support requires tracking `global_account_activeAccountId`
- API responses include fields not in our models (causing JSON parse failures)

**Dependencies:**
- Enhancement 02 (Storage Layer) - needs significant rework
- Enhancement 03 (API Client) - models need updates
- Enhancement 04 (Auth Commands) - token storage needs changes
- Must not break: Current login functionality (can store in new format)

## Requirements

### Functional Requirements
1. **Read TypeScript CLI storage format** - Parse namespaced keys like `user_{userId}_token_accessToken`
2. **Write TypeScript CLI storage format** - Store data using same key structure
3. **Support active account lookup** - Read `global_account_activeAccountId` to find current user
4. **Parse API responses correctly** - Update models to include all fields returned by Bitwarden API
5. **Maintain state version** - Preserve `stateVersion: 72` in data.json

### Non-Functional Requirements
- **Compatibility:** Must read/write identical format to TypeScript CLI v2024.x
- **Performance:** No degradation in storage read/write performance
- **Reliability:** Graceful handling of missing or malformed data
- **Backwards Compatibility:** Login functionality must continue to work

### Must Have (MVP)
- [ ] Update storage layer to use namespaced key format
- [ ] Read `global_account_activeAccountId` to determine active user
- [ ] Read tokens from `user_{userId}_token_accessToken` format
- [ ] Write tokens to same namespaced format
- [ ] Update Cipher model to match actual API response (all fields)
- [ ] Update SyncResponse model for correct parsing
- [ ] Sync command successfully fetches and stores vault data
- [ ] List items command shows decrypted vault contents
- [ ] Support latest state version (72) only

### Should Have (if time permits)
- [ ] Multi-account switching support
- [ ] Preserve all existing TypeScript CLI data on write

### Won't Have (out of scope)
- Implementing all 72 state migrations (reason: complexity, can require TS CLI to upgrade)
- Full multi-account management UI (reason: not MVP)
- Backwards compatibility with state versions < 60 (reason: too old)

## Open Questions
> These need answers before architecture review

1. **State Version Handling:** Should we require state version 72, or support a range? Recommendation: Require 72+, error clearly if older.
2. **Write Strategy:** When writing, should we preserve unknown keys or only write what we understand? Recommendation: Preserve unknown keys.
3. **Token Storage Location:** The TS CLI has `user_{userId}_token_accessToken` as `null` in the file but tokens work - where are tokens actually stored? Need to investigate secure storage / keychain integration.
4. **Session Key Usage:** How does BW_SESSION relate to the stored tokens? Is it used for decryption?

## Constraints & Limitations

**Technical Constraints:**
- Must use exact same key naming as TypeScript CLI
- Must preserve `stateVersion` field
- Must not corrupt existing TypeScript CLI data
- JSON field names must match API exactly (camelCase)

**Business/Timeline Constraints:**
- This is blocking all vault operations (sync, list, get)
- Should be prioritized before other enhancements

## Success Criteria

**Definition of Done:**
- [ ] Can login with Rust CLI and immediately use TypeScript CLI (shared session)
- [ ] Can login with TypeScript CLI and immediately use Rust CLI (shared session)
- [ ] `bw sync` successfully fetches vault data from API
- [ ] `bw list items` shows all vault items (3 test items: Login, Card, SecureNote)
- [ ] No data corruption when both CLIs access same data.json
- [ ] Existing login functionality still works

**Acceptance Tests:**
1. Given TypeScript CLI is logged in, when running Rust CLI `bw sync`, then sync succeeds using existing tokens
2. Given Rust CLI is logged in, when running TypeScript CLI `bw list items`, then items are listed
3. Given empty data.json, when logging in with Rust CLI, then data.json matches TypeScript CLI format
4. Given vault with 3 items, when running `bw list items`, then all 3 items are returned with correct types

## Security & Safety Considerations
- Token handling must remain secure (no logging of tokens)
- Session key (BW_SESSION) handling must match TypeScript CLI behavior
- Must not expose sensitive data in error messages
- Preserve file permissions (0600) on data.json

## Testing Strategy

**Unit Tests:**
- Storage key formatting: `user_{userId}_token_accessToken`
- Active account ID lookup
- Cipher model deserialization with all fields
- SyncResponse parsing

**Integration Tests:**
- Full login -> sync -> list flow
- Cross-CLI compatibility (login with TS, use Rust)
- Token refresh with namespaced storage

**Manual Test Scenarios:**
1. Login with TypeScript CLI, run `bw list items` with Rust CLI
2. Login with Rust CLI, verify data.json format matches TypeScript
3. Run sync, verify vault data stored correctly
4. List items and verify all 3 test items appear

## References & Research

**TypeScript CLI Source Files to Study:**
- `libs/common/src/platform/services/state.service.ts` - State key definitions
- `libs/common/src/state-migrations/migrations/*.ts` - State format evolution
- `libs/common/src/vault/models/response/cipher.response.ts` - Cipher model
- `apps/cli/src/platform/services/lowdb-storage.service.ts` - Storage implementation

**Actual TypeScript CLI data.json Structure:**
```json
{
  "stateVersion": 72,
  "global_account_accounts": {
    "{userId}": { "email": "...", "emailVerified": true }
  },
  "global_account_activeAccountId": "{userId}",
  "user_{userId}_token_accessToken": "...",
  "user_{userId}_token_refreshToken": "...",
  "user_{userId}_crypto_privateKey": "...",
  ...
}
```

**Actual API Cipher Response Fields (from /tmp/cipher0.json):**
- `edit`, `viewPassword`, `permissions`, `organizationUseTotp`
- `reprompt`, `key`, `archivedDate`, `object`, `sshKey`
- `login.uri`, `login.passwordRevisionDate`, `login.fido2Credentials`

## Notes for PM Subagent
> Instructions for how to process this enhancement

- This is a **critical bug fix**, not a new feature - prioritize accordingly
- The root cause was insufficient verification of TypeScript CLI format in earlier enhancements
- Ensure architect actually examines real TypeScript CLI data.json and API responses
- Flag if proposed changes would break current login functionality

## Notes for Architect Subagent
> Key architectural considerations

- **CRITICAL:** Actually read the TypeScript CLI source code and real data.json files
- Do NOT assume formats - verify against actual data
- Study `state.service.ts` for key naming conventions
- Study `cipher.response.ts` for complete field list
- Consider: Should we use a compatibility layer or rewrite storage?
- Evaluate: Is there a TypeScript CLI storage abstraction we can mirror?
- Pay special attention to how tokens are stored (may involve keychain/secure storage)

## Notes for Implementer Subagent
> Implementation guidance

- Start by writing a test that reads actual TypeScript CLI data.json
- Update storage layer incrementally - don't break login
- Add all missing Cipher fields with `#[serde(default)]` for optional ones
- Test against real API responses, not mocked data
- Keep debug logging for API responses during development

## Notes for Testing Subagent
> Testing and validation guidance

- **Primary focus:** Cross-CLI compatibility testing
- Test with real Bitwarden account (test account: genbwtest@gmail.com)
- Verify data.json format byte-for-byte compatible where possible
- Test error handling for older state versions
- Verify no data loss when Rust CLI writes to existing data.json
- Include regression tests for login functionality
