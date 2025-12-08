---
enhancement: 05-vault-read-commands
agent: requirements-analyst
task_id: task_1764948881_77060
timestamp: 2025-12-05T00:00:00Z
status: READY_FOR_ARCHITECTURE
---

# Vault Read Commands - Requirements Analysis

## Executive Summary

This enhancement implements vault read operations (`sync`, `list`, `get`) for the Bitwarden CLI Rust migration. These commands enable users to download their encrypted vault from the server, list vault items with filtering, and retrieve specific item details including field extraction (passwords, usernames, TOTP codes).

**Project Scope**: Core vault read operations - critical path for user value delivery
**Dependencies**: Enhancements 1-4 (project bootstrap, storage layer, API client, authentication)
**Estimated Complexity**: High (multiple commands, SDK integration, search, filtering)
**Risk Level**: Medium (SDK decryption, compatibility with TypeScript CLI, performance with large vaults)

## User-Centric Problem Statement

As a Bitwarden CLI user, I need to:
1. **Sync my vault** from the server to access my latest passwords and secure notes
2. **List items** with filters to find specific entries without opening each one
3. **Retrieve specific data** like passwords or TOTP codes for use in scripts and automation
4. **Search across items** to quickly locate credentials by name or URL

**Current Pain Points**:
- Cannot access vault data without sync
- Need efficient filtering for large vaults (1000+ items)
- Must extract specific fields (password, TOTP) without manual parsing
- Require exact compatibility with existing TypeScript CLI for migration

## Functional Requirements

### FR-1: Vault Synchronization
**Priority**: Must Have (MVP)

**Description**: Download complete encrypted vault data from Bitwarden servers and cache locally.

**User Stories**:
- **US-1.1**: As a CLI user, I want to run `bw sync` to download my complete vault, so that I can access my passwords offline
- **US-1.2**: As a CLI user, I want to run `bw sync --last` to see when I last synced, so that I know if my data is current
- **US-1.3**: As a CLI user, I want to run `bw sync --force` to force a complete re-sync, so that I can resolve sync issues

**Acceptance Criteria**:
- [ ] `bw sync` downloads all ciphers, folders, collections, and organizations from API
- [ ] Sync stores encrypted data in local storage using storage layer (enhancement 2)
- [ ] Sync stores last sync timestamp in ISO 8601 format
- [ ] `bw sync --last` returns only the timestamp without syncing
- [ ] `bw sync --force` performs full sync regardless of cached data
- [ ] Sync requires valid authenticated session
- [ ] Sync handles API pagination for large vaults
- [ ] Sync shows progress indicator for vaults with 100+ items
- [ ] Failed sync preserves previous cached data (doesn't corrupt storage)
- [ ] Sync completes in <10 seconds for typical vault (100 items)

**API Integration**:
- Uses `GET /api/sync` endpoint (from API client enhancement 3)
- Returns: `{ ciphers: [], folders: [], collections: [], organizations: [] }`
- Requires Bearer token authentication

**Data Stored** (via storage layer):
```json
{
  "lastSync": "2025-12-05T12:00:00Z",
  "ciphers": [...],
  "folders": [...],
  "collections": [...],
  "organizations": [...]
}
```

### FR-2: List Vault Items
**Priority**: Must Have (MVP)

**Description**: Display vault items with filtering options for efficient searching.

**User Stories**:
- **US-2.1**: As a CLI user, I want to run `bw list items` to see all my vault items, so that I can browse my credentials
- **US-2.2**: As a CLI user, I want to filter items by folder using `--folderid`, so that I can view only work credentials
- **US-2.3**: As a CLI user, I want to filter items by collection using `--collectionid`, so that I can see shared team passwords
- **US-2.4**: As a CLI user, I want to search items using `--search`, so that I can find entries by name quickly
- **US-2.5**: As a CLI user, I want to filter by URL using `--url`, so that I can find credentials for a specific website
- **US-2.6**: As a CLI user, I want to list folders, collections, and organizations separately, so that I can understand my vault structure

**Acceptance Criteria**:
- [ ] `bw list items` returns all items in JSON format
- [ ] List requires prior successful sync (error if not synced)
- [ ] `bw list items --folderid <id>` filters items to specific folder
- [ ] `bw list items --collectionid <id>` filters items to specific collection
- [ ] `bw list items --organizationid <id>` filters items to specific organization
- [ ] `bw list items --search <term>` filters items by name/notes (case-insensitive)
- [ ] `bw list items --url <url>` matches items by URI field
- [ ] `bw list items --trash` shows only deleted items
- [ ] Multiple filters can be combined (AND logic)
- [ ] `bw list folders` returns all folders
- [ ] `bw list collections` returns all collections
- [ ] `bw list organizations` returns all organizations
- [ ] Output format matches TypeScript CLI exactly (field names, structure)
- [ ] List operations complete in <1 second for typical vaults
- [ ] Decrypts only fields needed for display (name, folder, collection)

**List Commands**:
1. `bw list items [filters]` - List vault items
2. `bw list folders [--search]` - List folders
3. `bw list collections [--organizationid] [--search]` - List collections
4. `bw list organizations` - List organizations
5. `bw list org-members --organizationid <id>` - List org members (Should Have)
6. `bw list org-collections --organizationid <id>` - List org collections (Should Have)

**Filter Combinations Example**:
```bash
# Find GitHub work credentials
bw list items --search "github" --folderid "work-folder-id"

# Find all items in trash
bw list items --trash

# Find items by URL
bw list items --url "https://github.com"
```

### FR-3: Retrieve Specific Items
**Priority**: Must Have (MVP)

**Description**: Retrieve complete item details by ID or search query.

**User Stories**:
- **US-3.1**: As a CLI user, I want to run `bw get item <id>` to see full item details, so that I can view all fields
- **US-3.2**: As a CLI user, I want to search for an item by name when retrieving, so that I don't need to remember IDs
- **US-3.3**: As a CLI user, I want clear error messages when item not found, so that I can correct my query

**Acceptance Criteria**:
- [ ] `bw get item <id>` returns full item JSON including all encrypted fields (decrypted)
- [ ] `bw get item <search>` searches by name if <search> is not a valid UUID
- [ ] Search returns first match if multiple items found
- [ ] Returns error if item not found (exit code 1)
- [ ] Returns error if not synced yet
- [ ] Decrypts all item fields using SDK (enhancement 1)
- [ ] Output format matches TypeScript CLI exactly
- [ ] Get operations complete in <500ms

### FR-4: Extract Specific Fields
**Priority**: Must Have (MVP)

**Description**: Extract individual fields from items for scripting and automation.

**User Stories**:
- **US-4.1**: As a CLI user, I want to run `bw get password <id>` to extract just the password, so that I can use it in scripts
- **US-4.2**: As a CLI user, I want to run `bw get totp <id>` to get a current TOTP code, so that I can complete 2FA login
- **US-4.3**: As a CLI user, I want to run `bw get username <id>` to extract just the username, so that I can pipe it to other commands
- **US-4.4**: As a power user, I want to use `--raw` flag to get unformatted output, so that I can use it in shell scripts

**Acceptance Criteria**:
- [ ] `bw get username <id>` returns only username field as plain text
- [ ] `bw get password <id>` returns only password field as plain text
- [ ] `bw get uri <id>` returns first URI from item
- [ ] `bw get totp <id>` generates and returns current 6-digit TOTP code
- [ ] `bw get notes <id>` returns notes field
- [ ] TOTP generation uses SDK TOTP implementation
- [ ] Field extraction works with both ID and search term
- [ ] Returns empty string if field doesn't exist (not error)
- [ ] `--raw` flag outputs without formatting or newlines
- [ ] TOTP codes are valid and match mobile authenticator apps

**Field Extraction Commands** (MVP):
1. `bw get item <id>` - Full item details
2. `bw get username <id>` - Username field
3. `bw get password <id>` - Password field
4. `bw get uri <id>` - First URI
5. `bw get totp <id>` - Current TOTP code
6. `bw get notes <id>` - Notes field

**Field Extraction Commands** (Should Have):
7. `bw get attachment <id> --itemid <id>` - Download attachment
8. `bw get folder <id>` - Folder details
9. `bw get collection <id>` - Collection details
10. `bw get template <type>` - Item template JSON
11. `bw get fingerprint <email>` - Account fingerprint

**Scripting Example**:
```bash
# Login to service using credentials
USERNAME=$(bw get username "github")
PASSWORD=$(bw get password "github")
TOTP=$(bw get totp "github")

curl -u "$USERNAME:$PASSWORD" -H "X-2FA: $TOTP" https://api.github.com
```

### FR-5: Search Functionality
**Priority**: Must Have (MVP)

**Description**: Search across item names, notes, and URIs.

**User Stories**:
- **US-5.1**: As a CLI user, I want search to be case-insensitive, so that I don't need to remember exact capitalization
- **US-5.2**: As a CLI user, I want search to match partial terms, so that I can find items with incomplete information

**Acceptance Criteria**:
- [ ] Search matches against item name field
- [ ] Search matches against notes field content
- [ ] Search is case-insensitive
- [ ] Search supports partial matching (substring)
- [ ] Search does not require full vault scan (uses efficient filtering)
- [ ] Search combined with other filters (AND logic)

**Search Implementation Notes**:
- Match against decrypted name and notes fields
- No need for pre-built search index (acceptable to decrypt on demand for MVP)
- Consider search index optimization in future enhancement if performance issues arise

### FR-6: Output Formatting
**Priority**: Must Have (MVP)

**Description**: Format output according to global flags (--pretty, --raw, --response).

**User Stories**:
- **US-6.1**: As a CLI user, I want JSON output by default, so that I can parse results programmatically
- **US-6.2**: As a CLI user, I want `--pretty` to format JSON with indentation, so that I can read output easily
- **US-6.3**: As a CLI user, I want `--raw` to output plain text, so that I can use values directly in scripts

**Acceptance Criteria**:
- [ ] Default output is compact JSON (no pretty printing)
- [ ] `--pretty` flag formats JSON with 2-space indentation
- [ ] `--raw` flag outputs plain text without JSON wrapping
- [ ] `--response` flag wraps output in response envelope with success status
- [ ] Output format matches TypeScript CLI exactly (field names, structure, ordering)
- [ ] Sensitive fields are included in output (password, totp) - no redaction

**Output Format Examples**:
```bash
# Default (compact JSON)
bw get password "github"
> "ghp_abc123..."

# Pretty JSON
bw list items --pretty
> [
>   {
>     "id": "...",
>     "name": "GitHub",
>     ...
>   }
> ]

# Raw output (for scripting)
bw get password "github" --raw
> ghp_abc123...

# Response envelope
bw get password "github" --response
> {"success":true,"data":"ghp_abc123..."}
```

## Non-Functional Requirements

### NFR-1: Performance
**Priority**: Must Have

- **Sync Performance**: Complete sync in <10 seconds for typical vault (100 items)
- **List Performance**: Return results in <1 second for typical vault
- **Get Performance**: Retrieve item in <500ms
- **Large Vault Support**: Handle vaults with 10,000+ items without excessive memory usage
- **Decryption Efficiency**: Decrypt only fields needed for display/extraction

**Acceptance Criteria**:
- [ ] Sync operation shows progress for vaults >100 items
- [ ] List operations use lazy evaluation (don't decrypt all items upfront)
- [ ] Memory usage scales linearly with vault size
- [ ] No performance regression vs TypeScript CLI

### NFR-2: Security
**Priority**: Must Have

- **Encryption at Rest**: Store all synced data encrypted using storage layer
- **Memory Safety**: Clear decrypted data from memory after use
- **No Logging**: Never log decrypted passwords, TOTP codes, or sensitive fields
- **SDK Integration**: Use Bitwarden SDK for all cryptographic operations
- **Input Validation**: Validate all IDs and search terms to prevent injection

**Acceptance Criteria**:
- [ ] All cipher data decrypted using SDK (no manual crypto)
- [ ] Sensitive strings use `secrecy` crate for memory protection
- [ ] No decrypted data in logs or error messages
- [ ] Use `zeroize` for clearing sensitive memory
- [ ] Validate UUIDs before queries

**Security Implementation Notes**:
- Leverage `secrecy::Secret<String>` for passwords and keys
- Use `zeroize::Zeroize` trait for secure memory clearing
- SDK handles all encryption/decryption operations

### NFR-3: Compatibility
**Priority**: Must Have

- **TypeScript CLI Parity**: Output format exactly matches TypeScript CLI
- **Storage Format**: Compatible with TypeScript CLI data format (LowDB JSON)
- **API Compatibility**: Works with current Bitwarden API (no custom endpoints)

**Acceptance Criteria**:
- [ ] JSON output field names match TypeScript CLI (camelCase)
- [ ] JSON structure matches TypeScript CLI (object shapes, arrays)
- [ ] Can read vault data synced by TypeScript CLI
- [ ] TypeScript CLI can read vault data synced by Rust CLI

**Compatibility Testing**:
- Compare JSON output with TypeScript CLI using `diff`
- Test cross-compatibility with shared storage directory
- Validate against TypeScript CLI test suite output

### NFR-4: Reliability
**Priority**: Must Have

- **Graceful Degradation**: Handle sync failures without corrupting cache
- **Error Recovery**: Preserve previous cache on failed sync
- **Input Validation**: Validate IDs and search terms
- **Error Messages**: Clear, actionable error messages

**Acceptance Criteria**:
- [ ] Failed sync does not corrupt existing cached data
- [ ] Network failures during sync are handled gracefully
- [ ] Invalid IDs return clear error messages
- [ ] Missing items return appropriate error (not crash)
- [ ] Malformed API responses don't crash CLI

**Error Scenarios**:
1. Sync fails mid-download → preserve old cache
2. Item not found → return "Item not found" error
3. Not synced yet → return "Vault not synced" error
4. Invalid session → return authentication error
5. Network timeout → return retry message

### NFR-5: Usability
**Priority**: Should Have

- **Progress Indicators**: Show sync progress for large vaults
- **Helpful Errors**: Error messages include resolution hints
- **Consistent Output**: Predictable output format across all commands

**Acceptance Criteria**:
- [ ] Sync shows progress bar for vaults >100 items
- [ ] Error messages include next steps (e.g., "Run 'bw sync' first")
- [ ] Empty results return empty array, not error
- [ ] Help text matches TypeScript CLI conventions

## Integration Requirements

### INT-1: Storage Layer Integration
**Status**: Dependency on Enhancement 2 (Complete)

**Requirements**:
- Store synced vault data using `JsonFileStorage`
- Store encrypted ciphers in storage
- Store folders, collections, organizations metadata
- Store last sync timestamp
- Use atomic writes for data integrity

**Storage Keys**:
```rust
// Vault data
storage.set("lastSync", timestamp)?;
storage.set("ciphers", ciphers)?;
storage.set("folders", folders)?;
storage.set("collections", collections)?;
storage.set("organizations", organizations)?;
```

**Integration Points**:
- `bw-core::services::storage::Storage` trait
- `JsonFileStorage` implementation
- Data stored at `~/.config/Bitwarden CLI/data.json`

### INT-2: API Client Integration
**Status**: Dependency on Enhancement 3 (Complete)

**Requirements**:
- Use `ApiClient` for sync endpoint
- Handle authentication with Bearer tokens
- Handle API pagination for large responses
- Retry transient failures (network errors, 5xx responses)

**API Endpoints**:
```rust
// Sync all vault data
api_client.get_with_auth("/api/sync").await?
```

**Response Structure**:
```json
{
  "ciphers": [
    {
      "id": "uuid",
      "type": 1,
      "name": "2.encrypted...",
      "login": { ... },
      "folderId": "uuid",
      "collectionIds": ["uuid"],
      ...
    }
  ],
  "folders": [ ... ],
  "collections": [ ... ],
  "organizations": [ ... ]
}
```

### INT-3: Authentication Integration
**Status**: Dependency on Enhancement 4 (Complete)

**Requirements**:
- Check for valid session before sync/list/get operations
- Use session key for decryption
- Return authentication error if session expired
- Support `--session` flag for session key

**Session Validation**:
```rust
// Check if authenticated
let session = session_manager.get_session()?;
if session.is_none() {
    return Err("Not authenticated. Run 'bw login' first");
}
```

### INT-4: Bitwarden SDK Integration
**Status**: Dependency on Enhancement 1 (Complete)

**Requirements**:
- Decrypt ciphers using SDK
- Generate TOTP codes using SDK
- Parse encrypted strings (EncString format)
- Use SDK crypto for all decryption operations

**SDK Operations**:
```rust
// Decrypt cipher using SDK
let decrypted_cipher = sdk_client
    .decrypt_cipher(&encrypted_cipher, &session_key)
    .await?;

// Generate TOTP code
let totp_code = sdk_client
    .generate_totp(&totp_secret)
    .await?;
```

**SDK Structures** (from SDK documentation):
- `Cipher` - Encrypted vault item
- `CipherView` - Decrypted vault item
- `Totp` - TOTP configuration and generator
- `SymmetricCryptoKey` - Session encryption key

## Data Models

### Cipher Types
Based on Bitwarden API and SDK:

**Cipher Type Enum**:
1. `Login` (type=1) - Website credentials with username/password
2. `SecureNote` (type=2) - Encrypted notes
3. `Card` (type=3) - Credit card information
4. `Identity` (type=4) - Personal identity information

**Cipher Structure** (simplified):
```rust
pub struct Cipher {
    pub id: String,              // UUID
    pub type: CipherType,        // 1=Login, 2=Note, 3=Card, 4=Identity
    pub name: EncString,         // Encrypted name
    pub notes: Option<EncString>, // Encrypted notes
    pub folder_id: Option<String>,
    pub collection_ids: Vec<String>,
    pub organization_id: Option<String>,
    pub deleted_date: Option<String>, // ISO 8601 if in trash
    pub login: Option<CipherLogin>,
    // ... other fields
}

pub struct CipherLogin {
    pub username: Option<EncString>,
    pub password: Option<EncString>,
    pub uris: Vec<CipherUri>,
    pub totp: Option<EncString>,
}

pub struct CipherUri {
    pub uri: Option<EncString>,
    pub match_type: Option<UriMatchType>,
}
```

**Important**: All sensitive fields are encrypted using `EncString` format (e.g., `"2.base64data|base64iv|base64mac"`)

### Storage Schema
Data stored via storage layer:

```json
{
  "lastSync": "2025-12-05T12:00:00Z",
  "ciphers": [
    {
      "id": "uuid",
      "type": 1,
      "name": "2.encrypted...",
      "login": {
        "username": "2.encrypted...",
        "password": "2.encrypted...",
        "uris": [{"uri": "2.encrypted..."}],
        "totp": "2.encrypted..."
      },
      "folderId": "uuid",
      "collectionIds": ["uuid"],
      "organizationId": null,
      "deletedDate": null
    }
  ],
  "folders": [
    {
      "id": "uuid",
      "name": "2.encrypted..."
    }
  ],
  "collections": [
    {
      "id": "uuid",
      "name": "2.encrypted...",
      "organizationId": "uuid"
    }
  ],
  "organizations": [
    {
      "id": "uuid",
      "name": "Organization Name"
    }
  ]
}
```

## Success Criteria & Validation

### Validation Approach
1. **Unit Tests**: Test filtering, search, field extraction logic
2. **Integration Tests**: Test sync flow, decryption, TOTP generation
3. **Compatibility Tests**: Compare output with TypeScript CLI using `diff`
4. **Performance Tests**: Benchmark with large test vaults
5. **Manual Testing**: Real-world usage scenarios

### Acceptance Test Scenarios

**AT-1: Basic Sync and List**
```bash
# Given: Authenticated session
bw login user@example.com

# When: Sync vault
bw sync

# Then: Data cached locally, last sync timestamp stored
bw sync --last
# Expected: "2025-12-05T12:00:00Z"

# When: List items
bw list items

# Then: Returns JSON array of items
# Expected: [...items with decrypted names...]
```

**AT-2: Filtered List**
```bash
# Given: Synced vault with items in folders

# When: List items in specific folder
bw list items --folderid "work-folder-id"

# Then: Returns only items in work folder
# Expected: [items filtered by folderId]

# When: Search items
bw list items --search "github"

# Then: Returns items matching "github" in name
# Expected: [filtered items]
```

**AT-3: Field Extraction**
```bash
# Given: Synced vault with login item

# When: Get password
bw get password "github-item-id"

# Then: Returns plain text password
# Expected: "ghp_abc123def456..."

# When: Get TOTP code
bw get totp "github-item-id"

# Then: Returns current 6-digit code
# Expected: "123456"
```

**AT-4: Error Handling**
```bash
# Given: Not synced yet

# When: List items
bw list items

# Then: Error about needing to sync
# Expected: Error "Vault not synced. Run 'bw sync' first."

# When: Get non-existent item
bw get item "invalid-id"

# Then: Error about item not found
# Expected: Error "Item not found."
```

**AT-5: Trash Filtering**
```bash
# Given: Synced vault with trashed items

# When: List trash items
bw list items --trash

# Then: Returns only items with deletedDate set
# Expected: [items where deletedDate != null]
```

**AT-6: Output Formatting**
```bash
# Given: Synced vault

# When: Get password with --raw
bw get password "item-id" --raw

# Then: Plain text without formatting
# Expected: password_value (no quotes, no newline)

# When: List with --pretty
bw list items --pretty

# Then: Formatted JSON with indentation
# Expected: Indented JSON array
```

## Project Phasing & Implementation Strategy

### Phase 1: MVP Core (Must Have)
**Estimated Effort**: 7-10 days
**Goal**: Basic sync, list, and get functionality

**Components**:
1. **Sync Command** (2-3 days)
   - API integration for `/api/sync` endpoint
   - Storage integration for caching vault data
   - Basic error handling
   - `--last` and `--force` flags

2. **List Commands** (2-3 days)
   - `bw list items` with basic filters (folder, collection, organization)
   - `bw list folders`
   - `bw list collections`
   - `bw list organizations`
   - `--search` filter
   - `--trash` filter

3. **Get Commands** (2-3 days)
   - `bw get item <id>`
   - `bw get username <id>`
   - `bw get password <id>`
   - `bw get uri <id>`
   - `bw get totp <id>` with SDK TOTP generation
   - `bw get notes <id>`
   - Search by name if not UUID

4. **Output Formatting** (1 day)
   - Global flags: `--pretty`, `--raw`, `--response`
   - JSON serialization
   - Compatibility validation with TypeScript CLI

**Phase 1 Exit Criteria**:
- [ ] All MVP commands implemented
- [ ] Basic integration tests pass
- [ ] Output matches TypeScript CLI format
- [ ] Can sync, list, and retrieve items from real Bitwarden account

### Phase 2: Extended Features (Should Have)
**Estimated Effort**: 3-5 days
**Goal**: Additional get commands, org features, advanced filtering

**Components**:
1. **Advanced Get Commands** (2 days)
   - `bw get attachment <id>`
   - `bw get folder <id>`
   - `bw get collection <id>`
   - `bw get template <type>`

2. **Organization Features** (1-2 days)
   - `bw list org-members`
   - `bw list org-collections`

3. **URL Matching** (1 day)
   - `bw list items --url` with URI matching logic

**Phase 2 Exit Criteria**:
- [ ] All "Should Have" features implemented
- [ ] Advanced filtering works correctly
- [ ] Organization features tested

### Phase 3: Optimization & Polish (Nice to Have)
**Estimated Effort**: 2-3 days
**Goal**: Performance optimization, UX improvements

**Components**:
1. **Performance Optimization**
   - Lazy decryption for list operations
   - Search indexing if needed
   - Memory profiling for large vaults

2. **UX Improvements**
   - Progress indicators for sync
   - Better error messages with hints
   - Item count display

**Phase 3 Exit Criteria**:
- [ ] Performance benchmarks meet targets
- [ ] Large vault testing (10,000+ items) passes
- [ ] User experience polished

### Implementation Sequence
```
Prerequisites: Enhancements 1-4 complete
    ↓
Phase 1 (MVP Core)
├── Sync command
├── List commands
├── Get commands
└── Output formatting
    ↓
Phase 2 (Extended)
├── Advanced get commands
├── Organization features
└── URL matching
    ↓
Phase 3 (Polish)
├── Performance optimization
└── UX improvements
```

**Critical Path**: Phase 1 blocks enhancement 6 (vault write operations)

## Open Questions & Clarification Needs

### OQ-1: Large Vault Handling
**Question**: How should we handle very large vaults (10,000+ items) efficiently?

**Options**:
1. Load all items into memory and filter (simple but memory-intensive)
2. Implement lazy loading with pagination (complex but efficient)
3. Use memory-mapped storage (requires storage layer changes)

**Recommendation**: Start with option 1 (load all) for MVP, optimize later if needed. Most users have <1000 items.

**Decision Needed**: Confirm acceptable memory usage limits for target users.

### OQ-2: List Result Pagination
**Question**: Should `bw list` results be paginated for large result sets?

**Context**: TypeScript CLI returns all results without pagination. Large result sets may be unwieldy.

**Options**:
1. No pagination (match TypeScript CLI) - simpler, familiar
2. Add optional pagination (e.g., `--limit`, `--skip`) - more flexible
3. Auto-paginate if >100 results - automatic but surprising

**Recommendation**: No pagination for MVP (option 1) to maintain compatibility. Add optional pagination in future if users request it.

**Decision Needed**: Confirm no pagination for initial release.

### OQ-3: Sync Conflict Handling
**Question**: How should we handle sync conflicts (local changes vs server changes)?

**Context**: This enhancement is read-only, but conflicts may occur if vault modified on server since last sync.

**Options**:
1. Always overwrite local with server (server wins) - simple, consistent
2. Detect conflicts and warn user - complex, requires change tracking
3. Merge changes (like git) - very complex, error-prone

**Recommendation**: Option 1 (server wins) since this is read-only. Conflicts are not possible without write operations.

**Decision Needed**: Confirm server-wins strategy acceptable.

### OQ-4: Caching Strategy
**Question**: What's the caching strategy for synced data?

**Context**: Should we cache decrypted data or only encrypted data?

**Options**:
1. Cache only encrypted data, decrypt on every read - slower but more secure
2. Cache decrypted data with timeout - faster but security concern
3. Hybrid: cache decrypted for session duration - balanced

**Recommendation**: Option 1 (cache encrypted only) for MVP. Decryption is fast enough with SDK. Security is priority.

**Decision Needed**: Confirm no decrypted data caching.

### OQ-5: Incremental Sync Support
**Question**: Should we support incremental sync (only changes since last sync)?

**Context**: API may support incremental sync using revision dates. Could reduce bandwidth and sync time.

**Options**:
1. Full sync only (current TypeScript CLI behavior) - simple, reliable
2. Incremental sync with fallback to full sync - complex but efficient
3. Incremental sync as separate command (e.g., `bw sync --incremental`) - flexible

**Recommendation**: Option 1 (full sync only) for MVP. Add incremental sync in future enhancement if API supports it well.

**Decision Needed**: Confirm full sync only for initial release. Defer incremental sync to future enhancement.

### OQ-6: Trash Item Handling
**Question**: How should trash items be handled by default?

**Context**: Items can be soft-deleted (moved to trash) before permanent deletion.

**Options**:
1. Exclude trash items by default, require `--trash` to see them - cleaner default
2. Include trash items by default, require `--no-trash` to exclude - more complete
3. Have separate command `bw list trash` - more explicit

**Recommendation**: Option 1 (exclude by default) to match TypeScript CLI behavior. Most users don't want to see deleted items.

**Decision Needed**: Confirm trash excluded by default.

### OQ-7: TOTP Algorithm Support
**Question**: Which TOTP algorithms must we support?

**Context**: TOTP can use SHA1, SHA256, or SHA512. Most services use SHA1.

**Known Requirements**:
- SHA1 (most common, 6 digits, 30 second period)
- SDK already supports multiple algorithms

**Recommendation**: Support all algorithms supported by SDK. Let SDK handle algorithm detection from TOTP secret format.

**Decision Needed**: Confirm SDK handles all required TOTP algorithms. No custom implementation needed.

### OQ-8: Search Performance with Large Vaults
**Question**: Do we need a search index for large vaults?

**Context**: Searching 10,000+ items by decrypting each one may be slow.

**Options**:
1. No index, decrypt on search (simple but potentially slow)
2. Build in-memory search index on sync (complex but fast)
3. Persistent search index in storage (very complex, storage layer changes)

**Recommendation**: Option 1 (no index) for MVP. Profile performance with large test vaults. Add indexing in Phase 3 if needed.

**Decision Needed**: Confirm acceptable to defer search optimization.

## Risk Assessment & Mitigation

### Risk 1: SDK Decryption Complexity
**Severity**: High
**Probability**: Medium

**Description**: Integrating SDK decryption for all cipher types may reveal unexpected complexity or bugs.

**Mitigation**:
- Prioritize SDK integration in early implementation
- Create comprehensive test suite with all cipher types
- Reference TypeScript CLI SDK usage patterns
- Engage with SDK team if issues found

**Contingency**: If SDK issues block progress, implement workaround and file SDK bugs.

### Risk 2: TypeScript CLI Compatibility
**Severity**: Medium
**Probability**: Low

**Description**: Output format may not exactly match TypeScript CLI, breaking user scripts.

**Mitigation**:
- Create automated compatibility tests comparing JSON output
- Test with real TypeScript CLI installations
- Document any intentional differences
- Provide migration guide if format changes needed

**Contingency**: Add compatibility mode flag (e.g., `--typescript-compat`) if differences required.

### Risk 3: Performance with Large Vaults
**Severity**: Medium
**Probability**: Medium

**Description**: Decrypting thousands of items may be too slow for acceptable UX.

**Mitigation**:
- Benchmark early with large test vaults (10,000+ items)
- Implement lazy decryption (decrypt only displayed fields)
- Profile memory usage and optimize hot paths
- Add progress indicators for slow operations

**Contingency**: Implement search indexing or pagination if performance issues confirmed.

### Risk 4: TOTP Generation Accuracy
**Severity**: High
**Probability**: Low

**Description**: TOTP codes must exactly match authenticator apps or users can't login.

**Mitigation**:
- Use SDK TOTP implementation exclusively (no manual implementation)
- Test TOTP with real accounts and services
- Validate against multiple authenticator apps (Google Authenticator, Authy)
- Unit test TOTP with known test vectors

**Contingency**: If SDK TOTP has bugs, work with SDK team to fix. TOTP is critical for user access.

### Risk 5: Sync API Changes
**Severity**: Low
**Probability**: Low

**Description**: Bitwarden API may change sync response format during development.

**Mitigation**:
- Use current API documentation as source of truth
- Version API client to support multiple API versions
- Monitor Bitwarden release notes for API changes
- Flexible JSON parsing with optional fields

**Contingency**: Update data models and parsers to match new API format.

## Dependencies & Blockers

### Blockers (Must Complete Before Starting)
1. ✅ **Enhancement 1**: Project Bootstrap - Required for SDK setup
2. ✅ **Enhancement 2**: Storage Layer - Required for caching vault data
3. ✅ **Enhancement 3**: API Client - Required for sync endpoint
4. ✅ **Enhancement 4**: Authentication - Required for session management

**Status**: All blockers complete, ready to proceed with architecture phase.

### External Dependencies
1. **Bitwarden SDK** (`bitwarden-core` crate)
   - Used for: Cipher decryption, TOTP generation
   - Status: Available at `../sdk/` (per project setup)
   - Version: Latest stable

2. **Bitwarden API**
   - Used for: Sync endpoint (`/api/sync`)
   - Status: Production API documented
   - Authentication: Bearer token

3. **Storage Layer** (Enhancement 2)
   - Used for: Caching synced vault data
   - Interface: `Storage` trait with `JsonFileStorage` impl
   - Status: Complete

4. **API Client** (Enhancement 3)
   - Used for: HTTP requests to Bitwarden API
   - Interface: `ApiClient` trait with `BitwardenApiClient` impl
   - Status: Complete

## Technical Flags for Architect

### Architecture Decision Points

**AD-1: SDK Integration Strategy**
- **Decision Needed**: How to structure SDK integration for decryption
- **Options**: Direct SDK calls vs wrapper service layer
- **Recommendation**: Create `VaultService` wrapper around SDK for testability
- **Rationale**: Isolates SDK dependency, enables mocking for tests

**AD-2: Cipher Decryption Caching**
- **Decision Needed**: Whether to cache decrypted ciphers in memory during command execution
- **Options**: Cache for command duration vs decrypt on every access
- **Recommendation**: Cache for command duration using HashMap
- **Rationale**: List operations may access same cipher multiple times (filtering), avoid redundant decryption

**AD-3: Search Implementation**
- **Decision Needed**: How to implement efficient search across encrypted fields
- **Options**: Decrypt all vs decrypt on-demand vs pre-built index
- **Recommendation**: Start with decrypt on-demand, add index if performance issues
- **Rationale**: Premature optimization. Profile first, optimize later.

**AD-4: Filter Combination Logic**
- **Decision Needed**: How to combine multiple filters (--search, --folderid, etc.)
- **Options**: Sequential filtering vs parallel filtering vs query builder
- **Recommendation**: Sequential filtering with short-circuit evaluation
- **Rationale**: Simple, correct, performant enough for MVP

**AD-5: Error Propagation**
- **Decision Needed**: How to handle partial sync failures (some items fail decryption)
- **Options**: Fail entire sync vs skip failed items vs retry logic
- **Recommendation**: Log warnings for failed items, continue with successful ones
- **Rationale**: Graceful degradation, don't block user access to working items

### High-Level Technical Challenges

**TC-1: TOTP Code Generation**
- **Challenge**: TOTP codes must be accurate and time-synchronized
- **Approach**: Use SDK TOTP implementation, validate with test vectors
- **Complexity**: Medium (SDK handles heavy lifting)

**TC-2: URI Matching for --url Filter**
- **Challenge**: URL matching logic can be complex (domain matching, subdomain handling)
- **Approach**: Implement basic matching for MVP, enhance later
- **Complexity**: Medium (define matching rules)

**TC-3: Large Vault Performance**
- **Challenge**: Decrypting 10,000+ items may be slow
- **Approach**: Lazy evaluation, progress indicators, benchmark early
- **Complexity**: Medium to High (depends on SDK performance)

**TC-4: TypeScript CLI Compatibility**
- **Challenge**: Ensuring exact JSON output format match
- **Approach**: Automated diff testing, field-by-field validation
- **Complexity**: Low to Medium (mostly data modeling)

**TC-5: Secure Memory Handling**
- **Challenge**: Ensuring decrypted passwords cleared from memory
- **Approach**: Use `secrecy` and `zeroize` crates consistently
- **Complexity**: Medium (requires discipline across codebase)

## Documentation Requirements

### User Documentation
1. **Command Help Text** - Document all commands, flags, examples
2. **Migration Guide** - How to switch from TypeScript CLI
3. **Troubleshooting Guide** - Common errors and solutions
4. **Output Format Reference** - JSON structure documentation

### Developer Documentation
1. **Architecture Overview** - High-level design and data flow
2. **SDK Integration Guide** - How to use Bitwarden SDK
3. **Testing Guide** - How to run and write tests
4. **Performance Benchmarks** - Performance characteristics and targets

## Appendix: Reference Materials

### TypeScript CLI References
Referenced in specification:
- `apps/cli/src/vault/commands/sync.command.ts` - Sync command implementation
- `apps/cli/src/vault/commands/list.command.ts` - List command implementation
- `apps/cli/src/vault/commands/get.command.ts` - Get command implementation
- `libs/common/src/services/sync.service.ts` - Sync service logic

### Bitwarden SDK References
From web research:
- [SDK Cipher Documentation](https://sdk-api-docs.bitwarden.com/src/bitwarden_vault/cipher/cipher.rs.html) - Cipher decryption
- [SDK TOTP Documentation](https://sdk-api-docs.bitwarden.com/bitwarden_vault/struct.Totp.html) - TOTP generation
- [Bitwarden Crypto Crate](https://sdk-api-docs.bitwarden.com/bitwarden_crypto/index.html) - Cryptographic operations

### API Documentation
- [Bitwarden CLI Help](https://bitwarden.com/help/cli/) - Official CLI documentation
- [Vaultwarden API Reference](https://deepwiki.com/dani-garcia/vaultwarden/3-api-reference) - API endpoint documentation
- [Cozy Bitwarden API](https://docs.cozy.io/en/cozy-stack/bitwarden/) - API implementation reference

### Related Enhancements
- **Enhancement 1**: Project Bootstrap - SDK setup and project structure
- **Enhancement 2**: Storage Layer - JSON file storage for vault cache
- **Enhancement 3**: API Client - HTTP client for Bitwarden API
- **Enhancement 4**: Authentication - Session management and login

---

## Summary for Next Phase (Architecture)

**Readiness**: ✅ Ready for architecture phase

**Key Deliverables for Architect**:
1. Design `VaultService` for SDK integration and cipher decryption
2. Design data models for ciphers, folders, collections, organizations
3. Design filtering and search algorithm with performance considerations
4. Design TOTP generation flow using SDK
5. Design error handling strategy for sync failures and decryption errors
6. Design testing strategy including compatibility tests

**Critical Decisions for Architect**:
- Cipher decryption caching strategy (memory vs performance trade-off)
- Search implementation approach (decrypt on-demand vs indexing)
- Filter combination logic and short-circuit evaluation
- Error handling for partial sync failures
- Module organization and separation of concerns

**Success Criteria for Architecture Phase**:
- [ ] Clear module organization defined
- [ ] Data models specified with serialization format
- [ ] SDK integration patterns documented
- [ ] Filtering and search algorithms designed
- [ ] Error handling strategy documented
- [ ] Performance targets identified with profiling points
- [ ] Testing strategy defined (unit, integration, compatibility)

---

**Requirements Analysis Status**: ✅ COMPLETE
**Next Agent**: Architect
**Workflow Status**: READY_FOR_ARCHITECTURE
