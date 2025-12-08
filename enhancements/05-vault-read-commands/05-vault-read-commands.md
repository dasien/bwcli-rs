---
slug: vault-read-commands
status: NEW
created: 2024-12-02
author: Migration Team
priority: high
---

# Enhancement: CLI Rust Migration - Vault Read Operations

## Overview
**Goal:** Implement sync, list, and get commands to read and search vault data.

**User Story:**
As a CLI user, I want to sync my vault, list items, and retrieve specific data so that I can access my passwords and sensitive information from the command line.

## Context & Background
**Current State:**
- TypeScript CLI implements sync, list, and get commands
- Sync downloads full vault data from server
- List supports filtering by item type, folder, collection, organization
- Get retrieves specific items by ID or search
- Get supports extracting specific fields (username, password, TOTP, etc.)
- This is enhancement 5 of 8, depends on enhancements 1-4

**Technical Context:**
- Rust project at `bwcli-rs/`
- Must decrypt vault items using SDK
- Uses storage layer to cache synced data
- Uses API client for sync operations
- Requires authentication from enhancement 4

**Dependencies:**
- Enhancement: project-bootstrap (must be complete)
- Enhancement: storage-layer (for caching synced data)
- Enhancement: api-client (for sync API)
- Enhancement: authentication-commands (for session management)
- Bitwarden SDK for decryption

## Requirements

### Functional Requirements
1. Sync command to download full vault
2. Sync with --last flag to show last sync time
3. Sync with --force flag to force full sync
4. List command for: items, folders, collections, organizations, org-members, org-collections
5. List with filters: --search, --url, --folderid, --collectionid, --organizationid, --trash
6. Get command for items by ID or search query
7. Get specific fields: username, password, uri, totp, notes, exposed, attachment, folder, collection, template, fingerprint
8. Search functionality across items
9. Output in JSON, raw, or pretty formats
10. Decrypt ciphers using session key

### Non-Functional Requirements
- **Performance:** Sync <10s for typical vault, list <1s, get <500ms
- **Memory:** Efficient handling of large vaults (1000+ items)
- **Reliability:** Handle sync failures gracefully, cache partial data
- **Compatibility:** Output format matches TypeScript CLI exactly

### Must Have (MVP)
- [ ] `bw sync` command (full vault sync)
- [ ] `bw sync --last` to show last sync time
- [ ] `bw sync --force` for forced sync
- [ ] `bw list items` with filters
- [ ] `bw list folders`
- [ ] `bw list collections`
- [ ] `bw list organizations`
- [ ] `bw get item <id>`
- [ ] `bw get username <id>`
- [ ] `bw get password <id>`
- [ ] `bw get totp <id>`
- [ ] `bw get notes <id>`
- [ ] `bw get uri <id>`
- [ ] Search functionality across items
- [ ] Cipher decryption using SDK
- [ ] Output formatting

### Should Have (if time permits)
- [ ] `bw get attachment <id>`
- [ ] `bw get folder <id>`
- [ ] `bw get collection <id>`
- [ ] `bw get template <type>`
- [ ] `bw list org-members`
- [ ] `bw list org-collections`
- [ ] URL matching for list --url
- [ ] Trash filtering

### Won't Have (out of scope)
- Partial sync (reason: not in API currently)
- Offline mode (reason: separate enhancement)
- Real-time sync (reason: CLI is on-demand)

## Open Questions

1. How should we handle large vaults (10,000+ items)?
2. Should list results be paginated?
3. How to handle sync conflicts?
4. What's the caching strategy for synced data?
5. Should we support incremental sync in the future?
6. How to handle items in trash?

## Constraints & Limitations
**Technical Constraints:**
- Must decrypt all items using SDK
- Must cache synced data efficiently
- Must handle API pagination
- Must support all cipher types
- Must preserve encryption structure

**Business/Timeline Constraints:**
- Blocking enhancement 6 (write operations)
- Critical path item
- Must maintain data integrity

## Success Criteria
**Definition of Done:**
- [ ] `bw sync` downloads and caches full vault
- [ ] `bw sync --last` shows correct timestamp
- [ ] `bw list items` returns all decrypted items
- [ ] `bw list` filters work correctly
- [ ] `bw get` retrieves specific items
- [ ] Field extraction (username, password, etc.) works
- [ ] Search functionality works
- [ ] Output matches TypeScript CLI format
- [ ] All tests pass
- [ ] Documentation complete

**Acceptance Tests:**
1. Given authenticated session, when running `bw sync`, then vault data downloaded and cached
2. Given synced vault, when running `bw list items`, then all items returned in JSON
3. Given items in folder, when running `bw list items --folderid <id>`, then only folder items returned
4. Given item ID, when running `bw get item <id>`, then item details returned
5. Given item ID, when running `bw get password <id>`, then only password field returned
6. Given search query, when running `bw list items --search <query>`, then matching items returned
7. Given TOTP secret, when running `bw get totp <id>`, then current TOTP code generated
8. Given trashed items, when running `bw list items --trash`, then only trash items returned
9. Given no sync, when running `bw list`, then error about needing to sync first
10. Given stale sync, when running `bw sync`, then updates cached data

## Security & Safety Considerations
- Decrypt items only when needed
- Don't log decrypted data
- Clear decrypted data from memory after use
- Validate item structure before decryption
- Handle malformed vault data gracefully
- Zeroize sensitive fields

## UI/UX Considerations (if applicable)
- Show sync progress for large vaults
- Clear error if not synced yet
- Helpful messages for empty results
- Format output based on --pretty/--raw flags
- Show item count in list operations
- Clear indication of last sync time

## Testing Strategy
**Unit Tests:**
- Test cipher decryption
- Test filtering logic
- Test search functionality
- Test field extraction
- Test output formatting
- Test cache operations

**Integration Tests:**
- Test full sync flow
- Test list with various filters
- Test get operations
- Test with test vault data
- Test TOTP generation
- Test trash operations

**Manual Test Scenarios:**
1. Sync fresh vault
2. List all items
3. List with various filters
4. Get specific items
5. Extract passwords and TOTP
6. Search for items
7. Compare output with TypeScript CLI
8. Test with large vault (1000+ items)

## References & Research
- apps/cli/src/vault/commands/sync.command.ts
- apps/cli/src/vault/commands/list.command.ts
- apps/cli/src/vault/commands/get.command.ts
- libs/common/src/services/sync.service.ts
- Bitwarden SDK decryption APIs
- API sync endpoint documentation

## Notes for PM Subagent
- Verify all filter options required
- Confirm field extraction list is complete
- Flag if search functionality unclear
- Ensure performance requirements are realistic

## Notes for Architect Subagent
- Design efficient caching strategy
- Separate sync from list/get concerns
- Plan for large vault handling
- Design search indexing if needed
- Use SDK for all decryption
- Consider memory-mapped storage for large vaults
- Plan error handling for sync failures

## Notes for Implementer Subagent
- Use Bitwarden SDK for decryption
- Implement efficient filtering without loading all items
- Cache decrypted items temporarily
- Use iterators for large result sets
- Implement TOTP generation using SDK
- Follow TypeScript output format exactly
- Add progress indication for long operations
- Handle missing fields gracefully

## Notes for Testing Subagent
- Test with vaults of various sizes
- Test all filter combinations
- Test field extraction thoroughly
- Verify TOTP codes are valid
- Test search with various queries
- Verify output format matches TypeScript
- Test error handling for missing data
- Test cache invalidation
- Test concurrent list operations