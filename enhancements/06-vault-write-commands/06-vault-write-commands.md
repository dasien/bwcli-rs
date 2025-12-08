---
slug: vault-write-commands
status: NEW
created: 2024-12-02
author: Migration Team
priority: high
---

# Enhancement: CLI Rust Migration - Vault Write Operations

## Overview
**Goal:** Implement create, edit, delete, restore, share, and move commands for managing vault items.

**User Story:**
As a CLI user, I want to create, modify, and delete vault items from the command line so that I can manage my passwords without using the web interface.

## Context & Background
**Current State:**
- TypeScript CLI implements create, edit, delete, restore, share, move, confirm commands
- Create supports items, folders, attachments, org-collections
- Edit supports modifying existing items and their properties
- Delete supports soft delete (to trash) and permanent delete
- Restore recovers items from trash
- Share moves items to organizations
- This is enhancement 6 of 8, depends on enhancements 1-5

**Technical Context:**
- Rust project at `bwcli-rs/`
- Must encrypt items using SDK before sending to API
- Uses API client for CRUD operations
- Updates local cache after operations
- Requires authentication and vault sync

**Dependencies:**
- Enhancement: project-bootstrap (must be complete)
- Enhancement: storage-layer (for updating cache)
- Enhancement: api-client (for CRUD APIs)
- Enhancement: authentication-commands (for session)
- Enhancement: vault-read-operations (for retrieving items)
- Bitwarden SDK for encryption

## Requirements

### Functional Requirements
1. Create command for items (login, note, card, identity)
2. Create command for folders
3. Create command for attachments
4. Create command for org-collections
5. Edit command for items with field updates
6. Edit command for item-collections associations
7. Edit command for folders
8. Edit command for org-collections
9. Delete command for items (soft delete to trash)
10. Delete command with --permanent flag for hard delete
11. Delete command for attachments
12. Delete command for folders
13. Delete command for org-collections
14. Restore command to recover from trash
15. Share command to move items to organization
16. Move command to change folder
17. Confirm command for organization members
18. Support reading item data from stdin (encoded JSON)
19. Template generation for item types

### Non-Functional Requirements
- **Performance:** CRUD operations <2s typical
- **Memory:** Efficient handling of large attachments
- **Reliability:** Validate data before sending, handle errors gracefully
- **Compatibility:** Data format matches TypeScript CLI exactly

### Must Have (MVP)
- [ ] `bw create item` with JSON input
- [ ] `bw create folder`
- [ ] `bw edit item <id>`
- [ ] `bw edit folder <id>`
- [ ] `bw delete item <id>`
- [ ] `bw delete item <id> --permanent`
- [ ] `bw delete folder <id>`
- [ ] `bw restore item <id>`
- [ ] `bw move <id> <folderId>`
- [ ] Item encryption using SDK
- [ ] Template generation for item types
- [ ] Update local cache after operations
- [ ] Input validation
- [ ] Error handling

### Should Have (if time permits)
- [ ] `bw create attachment`
- [ ] `bw delete attachment`
- [ ] `bw create org-collection`
- [ ] `bw edit org-collection`
- [ ] `bw delete org-collection`
- [ ] `bw share <id> <organizationId>`
- [ ] `bw confirm org-member <id>`
- [ ] `bw edit item-collections <id>`
- [ ] Batch operations
- [ ] Undo functionality

### Won't Have (out of scope)
- GUI item editor (reason: CLI only)
- Conflict resolution (reason: last-write-wins)
- Version history (reason: not in API)

## Open Questions

1. How should we handle item conflicts during edit?
2. Should we validate item data structure before sending?
3. How to handle attachment uploads efficiently?
4. Should we support batch create/edit/delete?
5. How to handle partial update failures?
6. Should we prompt for confirmation on delete?

## Constraints & Limitations
**Technical Constraints:**
- Must encrypt items using SDK
- Must update local cache after operations
- Must validate item structure
- Must handle API errors gracefully
- Attachments size limited by API

**Business/Timeline Constraints:**
- Blocking enhancement 7 (tools)
- Critical path item
- Must maintain data integrity

## Success Criteria
**Definition of Done:**
- [ ] `bw create item` creates and encrypts item
- [ ] `bw edit item` modifies existing item
- [ ] `bw delete item` moves to trash
- [ ] `bw delete --permanent` removes permanently
- [ ] `bw restore` recovers from trash
- [ ] `bw move` changes item folder
- [ ] Local cache updated after operations
- [ ] All tests pass
- [ ] Documentation complete

**Acceptance Tests:**
1. Given item JSON, when running `bw create item`, then item created and ID returned
2. Given folder name, when running `bw create folder <name>`, then folder created
3. Given item ID, when running `bw edit item <id>`, then item updated
4. Given item ID, when running `bw delete item <id>`, then item moved to trash
5. Given trashed item, when running `bw restore item <id>`, then item restored
6. Given item and folder, when running `bw move <id> <folderId>`, then item moved
7. Given invalid item data, when creating, then validation error returned
8. Given deleted item with --permanent, when deleting, then item removed permanently
9. Given edited item, when syncing, then changes reflected
10. Given create operation, when offline, then clear error message

## Security & Safety Considerations
- Encrypt all sensitive fields before sending
- Validate item structure before encryption
- Don't log decrypted item data
- Clear sensitive data from memory
- Handle encryption failures gracefully
- Confirm before permanent deletion
- Validate IDs before operations

## UI/UX Considerations (if applicable)
- Show confirmation for destructive operations
- Clear success/failure messages
- Return item ID after creation
- Progress indication for attachments
- Helpful error messages
- Support --quiet for scripting

## Testing Strategy
**Unit Tests:**
- Test item encryption
- Test input validation
- Test JSON parsing
- Test cache updates
- Test error handling
- Test field updates

**Integration Tests:**
- Test full create flow
- Test edit with various fields
- Test delete and restore
- Test move operations
- Test with test vault
- Test error scenarios

**Manual Test Scenarios:**
1. Create various item types
2. Edit items with different fields
3. Delete and restore items
4. Move items between folders
5. Test permanent delete
6. Test with invalid data
7. Compare output with TypeScript CLI
8. Test offline behavior

## References & Research
- apps/cli/src/vault/commands/create.command.ts
- apps/cli/src/vault/commands/edit.command.ts
- apps/cli/src/vault/commands/delete.command.ts
- apps/cli/src/vault/commands/restore.command.ts
- apps/cli/src/vault/commands/share.command.ts
- apps/cli/src/vault/commands/move.command.ts
- Bitwarden SDK encryption APIs
- API CRUD endpoints documentation

## Notes for PM Subagent
- Verify all write operations required for MVP
- Confirm validation requirements
- Flag if attachment handling is complex
- Ensure destructive operations have safeguards

## Notes for Architect Subagent
- Design validation layer for item data
- Separate encryption from API communication
- Plan cache invalidation strategy
- Design for transactional operations if needed
- Use SDK for all encryption
- Consider optimistic updates for better UX
- Plan error recovery strategies

## Notes for Implementer Subagent
- Use Bitwarden SDK for encryption
- Validate item structure before encryption
- Update cache after successful operations
- Handle partial failures gracefully
- Support reading JSON from stdin
- Implement template generation
- Follow TypeScript input/output format
- Add confirmation prompts for destructive ops
- Use atomic operations where possible

## Notes for Testing Subagent
- Test all item types (login, note, card, identity)
- Test field updates thoroughly
- Test delete and restore flow
- Test permanent delete carefully
- Verify encryption before sending
- Test cache updates
- Test error scenarios
- Test with concurrent operations
- Verify data integrity after operations