---
enhancement: 12-vault-create-edit
agent: requirements-analyst
task_id: task_1765414288_20002
timestamp: 2025-12-10T12:00:00Z
status: READY_FOR_ARCHITECTURE
---

# Requirements Analysis Summary: Vault Create/Edit CLI Commands

## Executive Summary

This enhancement connects the existing bw-core WriteService to CLI command handlers, enabling users to create, edit, delete, restore, and move vault items from the command line using JSON input. The core business logic is already implemented; this work focuses on CLI integration, input parsing, and output formatting.

## Functional Requirements

### FR-1: Create Vault Items
**As a** CLI user
**I want to** create vault items using JSON input
**So that** I can programmatically add passwords and secure data to my vault

**Acceptance Criteria:**
- [ ] `bw create item <json>` creates a new encrypted vault item
- [ ] Supports all cipher types: Login (1), SecureNote (2), Card (3), Identity (4)
- [ ] Accepts base64-encoded JSON input (TypeScript CLI compatible)
- [ ] Accepts raw JSON input starting with `{` (developer convenience)
- [ ] Returns decrypted item JSON on success
- [ ] Validates all input fields before API submission
- [ ] Requires valid session key (--session or BW_SESSION)

### FR-2: Create Folders
**As a** CLI user
**I want to** create folders using JSON input
**So that** I can organize my vault items programmatically

**Acceptance Criteria:**
- [ ] `bw create folder <json>` creates a new folder
- [ ] Accepts JSON with `name` field
- [ ] Returns created folder JSON on success
- [ ] Validates folder name (max 1000 characters, non-empty)

### FR-3: Edit Vault Items
**As a** CLI user
**I want to** edit existing vault items using JSON input
**So that** I can update passwords and secure data programmatically

**Acceptance Criteria:**
- [ ] `bw edit item <id> <json>` updates an existing item
- [ ] Merges new data into existing item (preserves unchanged fields)
- [ ] Cannot edit items in trash (deleted_date present)
- [ ] Returns updated decrypted item JSON on success
- [ ] Validates item exists before update

### FR-4: Edit Folders
**As a** CLI user
**I want to** edit folder names
**So that** I can reorganize my vault structure

**Acceptance Criteria:**
- [ ] `bw edit folder <id> <json>` updates folder name
- [ ] Validates folder exists before update
- [ ] Returns updated folder JSON on success

### FR-5: Delete Items (Soft Delete)
**As a** CLI user
**I want to** move items to trash
**So that** I can remove items while retaining ability to restore

**Acceptance Criteria:**
- [ ] `bw delete item <id>` moves item to trash
- [ ] Sets `deleted_date` on the item
- [ ] Returns success message on completion
- [ ] Validates item exists before deletion

### FR-6: Delete Items (Permanent)
**As a** CLI user
**I want to** permanently delete items
**So that** I can completely remove sensitive data

**Acceptance Criteria:**
- [ ] `bw delete item <id> --permanent` permanently deletes item
- [ ] Item cannot be restored after permanent deletion
- [ ] `--permanent` flag is explicit opt-in
- [ ] Returns success message on completion

### FR-7: Delete Folders
**As a** CLI user
**I want to** delete folders
**So that** I can clean up my vault organization

**Acceptance Criteria:**
- [ ] `bw delete folder <id>` deletes the folder
- [ ] Items in the folder are moved to "no folder"
- [ ] Returns success message on completion

### FR-8: Restore Items from Trash
**As a** CLI user
**I want to** restore items from trash
**So that** I can recover accidentally deleted items

**Acceptance Criteria:**
- [ ] `bw restore item <id>` restores item from trash
- [ ] Only works for items with `deleted_date` set
- [ ] Returns error for items not in trash
- [ ] Returns restored item JSON on success

### FR-9: Move Items to Folders
**As a** CLI user
**I want to** move items between folders
**So that** I can organize my vault

**Acceptance Criteria:**
- [ ] `bw move <itemId> <folderId>` moves item to specified folder
- [ ] `bw move <itemId> null` removes item from folder
- [ ] Validates both item and folder exist
- [ ] Returns updated item JSON on success

### FR-10: Get Item Templates
**As a** CLI user
**I want to** get JSON templates for item types
**So that** I have a starting point for creating items

**Acceptance Criteria:**
- [ ] `bw get template item.login` returns login template
- [ ] `bw get template item.secureNote` returns secure note template
- [ ] `bw get template item.card` returns card template
- [ ] `bw get template item.identity` returns identity template
- [ ] `bw get template folder` returns folder template
- [ ] Templates match TypeScript CLI format exactly

### FR-11: Input Format Support
**As a** CLI user
**I want to** provide input in multiple formats
**So that** I can use the CLI conveniently in scripts and manually

**Acceptance Criteria:**
- [ ] Accepts base64-encoded JSON (TypeScript CLI compatible)
- [ ] Accepts raw JSON (detected by leading `{`)
- [ ] Supports reading JSON from stdin
- [ ] Clear error messages for invalid input

## Non-Functional Requirements

### NFR-1: Performance
- Operations complete in < 2 seconds under normal network conditions
- No unnecessary memory allocations for large JSON payloads
- Efficient stdin reading for piped input

### NFR-2: Reliability
- All user input validated before encryption and API calls
- Graceful error handling with actionable error messages
- Local cache updated atomically after API success

### NFR-3: Compatibility
- Input/output JSON format matches TypeScript CLI exactly
- Items created by Rust CLI readable by TypeScript CLI
- Items created by TypeScript CLI editable by Rust CLI
- Supports all existing CLI flags (--session, --raw, etc.)

### NFR-4: Security
- Session key required for all write operations
- No sensitive data logged (passwords, TOTP secrets, etc.)
- Input validation prevents injection attacks
- Zeroization patterns for sensitive intermediate data

## Integration Points

### Existing Infrastructure (No Changes Required)

| Component | Location | Status |
|-----------|----------|--------|
| WriteService | `bw-core/src/services/vault/write_service.rs` | Complete |
| CipherView model | `bw-core/src/models/vault/cipher.rs` | Complete |
| CipherService | `bw-core/src/services/vault/cipher_service.rs` | Complete |
| ValidationService | `bw-core/src/services/vault/validation_service.rs` | Complete |
| API endpoints | `bw-core/src/services/api/endpoints.rs` | Complete |

### CLI Components (To Be Implemented)

| Component | Location | Purpose |
|-----------|----------|---------|
| Input parsing helpers | `bw-cli/src/commands/vault.rs` | Base64/JSON parsing |
| Command handlers | `bw-cli/src/commands/vault.rs` | Connect to WriteService |
| Template generation | `bw-cli/src/commands/vault.rs` | Generate type templates |
| Response formatting | `bw-cli/src/commands/vault.rs` | Format output JSON |

## Technical Constraints

1. **Must use existing WriteService** - No duplicate business logic
2. **Must use existing CipherService** - For all encryption operations
3. **Must use existing ValidationService** - For input validation
4. **Session key required** - For user key decryption
5. **TypeScript CLI compatibility** - Input/output format parity

## Open Questions (Requiring Resolution)

### OQ-1: `bw move <id> null` Handling
**Question:** How should we represent "no folder" in the move command?
**Options:**
- A) Literal string "null"
- B) Empty string ""
- C) Special keyword "none"

**Recommendation:** Option A (literal "null") for TypeScript CLI compatibility.

### OQ-2: `--quiet` Flag
**Question:** Should we support `--quiet` flag to suppress output for scripting?

**Recommendation:** Defer to Phase 2. Not in current MVP scope.

## Scope Definition

### In Scope (MVP)

| Command | Description |
|---------|-------------|
| `bw create item <json>` | Create vault item (all types) |
| `bw create folder <json>` | Create folder |
| `bw edit item <id> <json>` | Update vault item |
| `bw edit folder <id> <json>` | Update folder |
| `bw delete item <id>` | Soft delete to trash |
| `bw delete item <id> --permanent` | Permanent delete |
| `bw delete folder <id>` | Delete folder |
| `bw restore item <id>` | Restore from trash |
| `bw move <id> <folderId>` | Move item to folder |
| `bw get template <type>` | Get JSON templates |

### Out of Scope (Deferred)

| Command | Reason |
|---------|--------|
| `bw create attachment` | Complexity, separate enhancement |
| `bw delete attachment` | Complexity, separate enhancement |
| `bw edit item-collections` | Organization feature |
| `bw create/edit/delete org-collection` | Organization feature |
| `bw share` | Organization feature |
| `bw confirm` | Organization feature |
| Interactive prompts | CLI-only design |
| Batch operations | Complexity |
| Undo functionality | API doesn't support |

## Validation Rules Summary

### Cipher Validation
| Field | Rule |
|-------|------|
| `name` | Required, max 1000 characters |
| `notes` | Optional, max 10000 characters |
| `uri` (in login) | Max 10000 characters each |
| `folderId` | Valid UUID format if present |
| `organizationId` | Valid UUID format if present |
| `totp` | Must be `otpauth://` URI if present |
| Type-specific data | Required for cipher type (login, card, etc.) |

### Folder Validation
| Field | Rule |
|-------|------|
| `name` | Required, max 1000 characters |

## Success Metrics

1. **Functional Completeness:** All MVP commands implemented and working
2. **TypeScript CLI Parity:** Input/output format matches 100%
3. **Interoperability:** Round-trip create/edit between Rust and TypeScript CLIs
4. **Test Coverage:** Unit tests for parsing, integration tests for flows
5. **Error Quality:** Clear, actionable error messages for all failure modes

## Risk Assessment

### Low Risk
- WriteService implementation is complete and tested
- CipherView models match TypeScript format
- API endpoints are established

### Medium Risk
- JSON merge logic for edit operations needs careful implementation
- stdin input handling complexity

### Mitigations
- Reference TypeScript CLI CipherExport.toView() for merge behavior
- Use existing stdin patterns from `bw encode` command

## Project Phases

### Phase 1: Input Parsing & Templates
- Base64/JSON parsing helpers
- Template generation for all types
- Stdin input support

### Phase 2: Create/Edit Commands
- `bw create item/folder`
- `bw edit item/folder`
- Response formatting

### Phase 3: Delete/Restore/Move Commands
- `bw delete item/folder`
- `bw restore item`
- `bw move`

### Phase 4: Testing & Documentation
- Unit tests for parsing
- Integration tests for flows
- User documentation

## Handoff to Architecture

This analysis is complete and ready for architectural design. Key areas requiring architectural decisions:

1. **Input parsing module structure** - Where to place base64/JSON helpers
2. **Template generation approach** - Static JSON vs programmatic CipherView creation
3. **Error handling strategy** - CliError variants for input/validation failures
4. **stdin detection approach** - How to detect piped input

The WriteService API is stable and well-defined. No changes to bw-core are anticipated.
