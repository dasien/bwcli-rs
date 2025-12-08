---
enhancement: 06-vault-write-commands
agent: requirements-analyst
task_id: task_1764951074_96448
timestamp: 2025-12-05T08:45:00Z
status: READY_FOR_ARCHITECTURE
---

# Vault Write Commands - Requirements Analysis

## Executive Summary

This enhancement implements vault write operations (`create`, `edit`, `delete`, `restore`, `move`, `share`, `confirm`) for the Bitwarden CLI Rust migration. These commands enable users to create, modify, and delete vault items from the command line, completing the core vault management functionality.

**Project Scope**: Core vault write operations - critical path for feature parity with TypeScript CLI
**Dependencies**: Enhancements 1-5 (project bootstrap, storage layer, API client, authentication, vault read operations)
**Estimated Complexity**: High (CRUD operations, encryption, validation, cache management, destructive operations)
**Risk Level**: High (data integrity, destructive operations, encryption complexity, API error handling)

## User-Centric Problem Statement

As a Bitwarden CLI user, I need to:
1. **Create new vault items** (logins, notes, cards, identities) without using the web interface
2. **Edit existing items** to update passwords, usernames, or other fields
3. **Delete items** safely with ability to restore from trash
4. **Organize items** by moving them between folders or sharing with organizations
5. **Manage vault structure** by creating and editing folders and collections
6. **Ensure data integrity** with validation and safe destructive operations

**Current Pain Points**:
- Cannot manage vault items from command line (must use web/desktop app)
- No scriptable way to create or update credentials programmatically
- Risk of data loss with destructive operations
- Need for confirmation on permanent deletions
- Complex item structure requires validation before submission
- Must maintain exact compatibility with TypeScript CLI for migration

## Functional Requirements

### FR-1: Create Vault Items
**Priority**: Must Have (MVP)

**Description**: Create new vault items with encrypted data using SDK.

**User Stories**:
- **US-1.1**: As a CLI user, I want to run `bw create item` with JSON input, so that I can create login credentials programmatically
- **US-1.2**: As a CLI user, I want to pipe JSON to `bw create item`, so that I can integrate with other tools
- **US-1.3**: As a CLI user, I want to use item templates, so that I know the correct JSON structure for different item types
- **US-1.4**: As a CLI user, I want to create folders for organization, so that I can group related items
- **US-1.5**: As a CLI user, I want validation errors before submission, so that I catch mistakes early

**Acceptance Criteria**:
- [ ] `bw create item` accepts JSON from stdin or as base64-encoded parameter
- [ ] Supports all item types: login (type=1), note (type=2), card (type=3), identity (type=4)
- [ ] Validates item structure before encryption (required fields, valid types)
- [ ] Encrypts all sensitive fields using SDK before API submission
- [ ] Returns created item ID in JSON response
- [ ] `bw create folder <name>` creates folder and returns ID
- [ ] Updates local cache after successful creation
- [ ] Returns clear validation errors for malformed JSON
- [ ] Requires authenticated session
- [ ] Create operations complete in <2 seconds typical

**Item Structure**:
```json
{
  "type": 1,
  "name": "GitHub Login",
  "notes": "Personal account",
  "login": {
    "username": "user@example.com",
    "password": "strong_password_123",
    "uris": [{"uri": "https://github.com"}],
    "totp": "otpauth://totp/..."
  },
  "folderId": "uuid-optional",
  "favorite": false
}
```

**Template Generation**:
- [ ] `bw get template item.login` returns login item template
- [ ] `bw get template item.note` returns note template
- [ ] `bw get template item.card` returns card template
- [ ] `bw get template item.identity` returns identity template
- [ ] `bw get template folder` returns folder template

### FR-2: Edit Vault Items
**Priority**: Must Have (MVP)

**Description**: Modify existing vault items with field updates and re-encryption.

**User Stories**:
- **US-2.1**: As a CLI user, I want to run `bw edit item <id>` with updated JSON, so that I can change passwords or other fields
- **US-2.2**: As a CLI user, I want to edit folders, so that I can rename organizational structure
- **US-2.3**: As a CLI user, I want validation before update, so that I don't corrupt existing items
- **US-2.4**: As a CLI user, I want to see clear errors if item not found, so that I can correct my command

**Acceptance Criteria**:
- [ ] `bw edit item <id>` accepts complete item JSON (not partial updates)
- [ ] Retrieves existing item first to ensure it exists
- [ ] Validates updated JSON structure before encryption
- [ ] Encrypts updated fields using SDK
- [ ] Preserves immutable fields (id, organizationId, revision date)
- [ ] Returns updated item in JSON response
- [ ] `bw edit folder <id> <name>` renames folder
- [ ] Updates local cache after successful edit
- [ ] Returns error if item not found (exit code 1)
- [ ] Edit operations complete in <2 seconds typical

**Edit Workflow**:
```bash
# Get item, modify locally, then update
bw get item <id> | jq '.name = "New Name"' | bw encode | bw edit item <id>
```

**Field Update Behavior**:
- Full replacement (not merge) - user must provide complete item JSON
- ID and core metadata preserved (not overwritten)
- Revision tracking updated by API
- Cache invalidated and refreshed after update

### FR-3: Delete Vault Items
**Priority**: Must Have (MVP)

**Description**: Delete items with two-stage process (trash then permanent).

**User Stories**:
- **US-3.1**: As a CLI user, I want to run `bw delete item <id>` to move to trash, so that I can recover if needed
- **US-3.2**: As a CLI user, I want to run `bw delete item <id> --permanent` for hard delete, so that I can permanently remove sensitive data
- **US-3.3**: As a CLI user, I want confirmation before permanent delete, so that I don't accidentally lose data
- **US-3.4**: As a CLI user, I want to delete folders, so that I can clean up organizational structure
- **US-3.5**: As a CLI user, I want clear feedback on deletion success, so that I know the operation completed

**Acceptance Criteria**:
- [ ] `bw delete item <id>` performs soft delete (moves to trash, sets deletedDate)
- [ ] `bw delete item <id> --permanent` performs hard delete (permanent removal)
- [ ] Permanent delete shows confirmation prompt (unless `--nointeraction` flag)
- [ ] `bw delete folder <id>` deletes folder (items become unfoldered)
- [ ] Returns success message after deletion
- [ ] Updates local cache after successful deletion
- [ ] Returns error if item not found
- [ ] Soft delete preserves item data (can be restored)
- [ ] Hard delete removes item completely from vault
- [ ] Delete operations complete in <2 seconds typical

**Deletion Behavior**:
- Soft delete: Sets `deletedDate` field, item remains in vault (visible with `--trash`)
- Hard delete: API removes item completely, cache purged
- Folder deletion: Items in folder set to `folderId: null`, folder removed
- Confirmation prompt: "Are you sure you want to permanently delete this item? [y/N]"

### FR-4: Restore Deleted Items
**Priority**: Must Have (MVP)

**Description**: Recover items from trash back to active vault.

**User Stories**:
- **US-4.1**: As a CLI user, I want to run `bw restore item <id>` to recover from trash, so that I can undo accidental deletions
- **US-4.2**: As a CLI user, I want clear error if item not in trash, so that I understand restore requirements

**Acceptance Criteria**:
- [ ] `bw restore item <id>` clears deletedDate field, making item active
- [ ] Returns restored item in JSON response
- [ ] Updates local cache after successful restore
- [ ] Returns error if item not found or not in trash
- [ ] Restore operations complete in <2 seconds typical
- [ ] Item restored to original folder and organization

**Restore Workflow**:
```bash
# List trashed items
bw list items --trash

# Restore specific item
bw restore item <id>

# Verify restoration
bw get item <id>
```

### FR-5: Move Items Between Folders
**Priority**: Must Have (MVP)

**Description**: Change item folder assignment for organization.

**User Stories**:
- **US-5.1**: As a CLI user, I want to run `bw move <id> <folderId>` to move items, so that I can reorganize my vault
- **US-5.2**: As a CLI user, I want to move items to root folder, so that I can remove folder assignment

**Acceptance Criteria**:
- [ ] `bw move <itemId> <folderId>` updates item's folderId field
- [ ] `bw move <itemId> null` removes folder assignment (move to root)
- [ ] Returns updated item in JSON response
- [ ] Updates local cache after successful move
- [ ] Returns error if item or folder not found
- [ ] Move operations complete in <2 seconds typical
- [ ] Preserves all other item fields

**Move Behavior**:
- Updates only folderId field (partial update)
- Validates folder exists before move
- Null folder ID moves to vault root
- Cache updated atomically

### FR-6: Input Validation
**Priority**: Must Have (MVP)

**Description**: Validate item structure and data before submission to API.

**User Stories**:
- **US-6.1**: As a CLI user, I want validation errors before API submission, so that I catch mistakes early
- **US-6.2**: As a CLI user, I want specific error messages, so that I can fix validation issues quickly
- **US-6.3**: As a CLI user, I want validation of required fields, so that I don't create incomplete items

**Acceptance Criteria**:
- [ ] Validates JSON structure before parsing
- [ ] Validates item type is valid (1-4)
- [ ] Validates required fields present (type, name)
- [ ] Validates field types match expected schema
- [ ] Validates UUIDs for IDs and references
- [ ] Returns specific error messages indicating validation failure
- [ ] Validation occurs before encryption (no wasted crypto operations)
- [ ] Validation follows TypeScript CLI rules exactly

**Validation Rules**:
1. **Required Fields**: `type`, `name`
2. **Type Values**: 1 (login), 2 (note), 3 (card), 4 (identity)
3. **UUID Format**: For `id`, `folderId`, `organizationId`, `collectionIds`
4. **Login Type**: Must have `login` object if type=1
5. **URI Structure**: `uris` array with valid URI objects
6. **TOTP Format**: Valid otpauth:// URI if provided
7. **Max Lengths**: Name (1000), Notes (10000), Password (1000)

### FR-7: Create Attachments
**Priority**: Should Have

**Description**: Upload file attachments to existing items.

**User Stories**:
- **US-7.1**: As a CLI user, I want to run `bw create attachment --file <path> --itemid <id>`, so that I can attach files to items
- **US-7.2**: As a CLI user, I want progress indication for large files, so that I know upload is progressing

**Acceptance Criteria**:
- [ ] `bw create attachment --file <path> --itemid <id>` uploads file
- [ ] Validates file exists before upload
- [ ] Shows progress bar for files >1MB
- [ ] Encrypts attachment using SDK before upload
- [ ] Returns attachment ID after successful upload
- [ ] Updates local cache with attachment metadata
- [ ] Handles API size limits (max 100MB typically)
- [ ] Returns clear error if item not found or size exceeded

### FR-8: Delete Attachments
**Priority**: Should Have

**Description**: Remove attachments from vault items.

**User Stories**:
- **US-8.1**: As a CLI user, I want to run `bw delete attachment <attachmentId> --itemid <itemId>`, so that I can remove outdated files

**Acceptance Criteria**:
- [ ] `bw delete attachment <id> --itemid <id>` removes attachment
- [ ] Returns success message after deletion
- [ ] Updates local cache to remove attachment metadata
- [ ] Returns error if attachment or item not found

### FR-9: Organization Collection Management
**Priority**: Should Have

**Description**: Create and manage organization collections.

**User Stories**:
- **US-9.1**: As an organization admin, I want to create collections, so that I can organize shared items
- **US-9.2**: As an organization admin, I want to edit collections, so that I can update sharing permissions
- **US-9.3**: As an organization admin, I want to delete collections, so that I can clean up unused sharing groups

**Acceptance Criteria**:
- [ ] `bw create org-collection --organizationid <id>` creates collection from JSON
- [ ] `bw edit org-collection <id>` updates collection from JSON
- [ ] `bw delete org-collection <id>` removes collection
- [ ] Validates organization membership before operations
- [ ] Returns clear error if insufficient permissions
- [ ] Updates local cache after collection operations

### FR-10: Share Items with Organizations
**Priority**: Should Have

**Description**: Move personal items to organization shared vaults.

**User Stories**:
- **US-10.1**: As a CLI user, I want to run `bw share <itemId> <organizationId>`, so that I can share credentials with my team
- **US-10.2**: As a CLI user, I want to specify collections during share, so that I control access scope

**Acceptance Criteria**:
- [ ] `bw share <itemId> <organizationId>` transfers item ownership to org
- [ ] Accepts collection IDs to assign during share
- [ ] Re-encrypts item with organization key (SDK operation)
- [ ] Returns updated item in JSON response
- [ ] Updates local cache with organization assignment
- [ ] Validates organization membership before share
- [ ] Item removed from personal vault, added to org vault

### FR-11: Confirm Organization Members
**Priority**: Should Have

**Description**: Confirm pending organization member invitations.

**User Stories**:
- **US-11.1**: As an organization admin, I want to run `bw confirm org-member <id>`, so that I can activate pending users

**Acceptance Criteria**:
- [ ] `bw confirm org-member <id> --organizationid <id>` confirms member
- [ ] Validates admin permissions before operation
- [ ] Returns success message after confirmation
- [ ] Updates local cache with member status

### FR-12: Edit Item-Collection Associations
**Priority**: Should Have

**Description**: Update which collections an organization item belongs to.

**User Stories**:
- **US-12.1**: As a CLI user, I want to edit item-collection assignments, so that I can control access to shared items

**Acceptance Criteria**:
- [ ] `bw edit item-collections <itemId> --organizationid <id>` updates collection IDs
- [ ] Accepts collection IDs as JSON array
- [ ] Validates collection membership
- [ ] Updates local cache with new assignments

## Non-Functional Requirements

### NFR-1: Performance
**Priority**: Must Have

- **Create Performance**: Complete in <2 seconds for typical item
- **Edit Performance**: Complete in <2 seconds for typical item
- **Delete Performance**: Complete in <2 seconds for typical item
- **Attachment Upload**: Show progress for files >1MB, complete in reasonable time
- **Encryption Efficiency**: SDK encryption should not be bottleneck

**Acceptance Criteria**:
- [ ] CRUD operations meet performance targets
- [ ] No unnecessary API round-trips
- [ ] Efficient cache updates (atomic writes)
- [ ] Encryption operations parallelized where possible
- [ ] No performance regression vs TypeScript CLI

### NFR-2: Data Integrity & Reliability
**Priority**: Must Have (Critical)

- **Atomic Operations**: Updates complete fully or rollback
- **Cache Consistency**: Local cache always reflects server state after operations
- **Validation Before Submission**: Prevent API errors through client-side validation
- **Error Recovery**: Handle partial failures gracefully
- **Idempotency**: Retry-safe operations where possible

**Acceptance Criteria**:
- [ ] Failed create/edit/delete does not corrupt cache
- [ ] Cache updated atomically after successful operations
- [ ] Validation prevents invalid data submission
- [ ] Network failures handled with clear error messages
- [ ] Partial update failures reported clearly
- [ ] Concurrent operation safety (no race conditions)

**Critical Scenarios**:
1. Network failure during create → cache unchanged, clear error
2. API validation failure → cache unchanged, show API error
3. Encryption failure → operation aborted, clear error
4. Cache write failure → operation aborted, clear error
5. Concurrent edits → last-write-wins (API behavior)

### NFR-3: Security
**Priority**: Must Have (Critical)

- **Encryption at Rest**: All sensitive fields encrypted using SDK
- **Memory Safety**: Clear sensitive data from memory after use
- **No Logging**: Never log decrypted passwords or sensitive fields
- **SDK Integration**: Use Bitwarden SDK for all cryptographic operations
- **Validation**: Prevent injection attacks through input validation
- **Confirmation**: Require confirmation for destructive operations

**Acceptance Criteria**:
- [ ] All item data encrypted using SDK before API submission
- [ ] Sensitive strings use `secrecy` crate for memory protection
- [ ] No sensitive data in logs or error messages
- [ ] Use `zeroize` for clearing sensitive memory
- [ ] Validate UUIDs and input to prevent injection
- [ ] Permanent delete requires confirmation (unless `--nointeraction`)
- [ ] Encryption failures abort operation (fail-safe)

**Security Implementation**:
- SDK handles all encryption/decryption
- `secrecy::Secret<String>` for passwords and keys
- `zeroize::Zeroize` trait for secure memory clearing
- Input validation on all user-provided data
- No echo of sensitive data in terminal output

### NFR-4: Compatibility
**Priority**: Must Have

- **TypeScript CLI Parity**: Input/output format exactly matches TypeScript CLI
- **API Compatibility**: Works with current Bitwarden API (no custom endpoints)
- **Data Format**: Compatible with TypeScript CLI data structures
- **Behavior Parity**: Same validation rules and error messages

**Acceptance Criteria**:
- [ ] JSON input format matches TypeScript CLI
- [ ] JSON output format matches TypeScript CLI (field names, structure)
- [ ] Can modify items created by TypeScript CLI
- [ ] TypeScript CLI can modify items created by Rust CLI
- [ ] Error messages similar to TypeScript CLI
- [ ] Confirmation prompts match TypeScript CLI behavior

### NFR-5: Usability
**Priority**: Should Have

- **Clear Errors**: Error messages include resolution hints
- **Progress Indication**: Show progress for long operations (attachments)
- **Confirmation Prompts**: Warn before destructive operations
- **Template Support**: Provide item templates for easy creation
- **Helpful Messages**: Success messages confirm operation

**Acceptance Criteria**:
- [ ] Validation errors specify field and problem
- [ ] API errors shown with context
- [ ] Confirmation prompt for `--permanent` delete
- [ ] Progress bar for attachment uploads >1MB
- [ ] Success messages include created/updated item ID
- [ ] Help text matches TypeScript CLI conventions

## Integration Requirements

### INT-1: Storage Layer Integration
**Status**: Dependency on Enhancement 2 (Complete)

**Requirements**:
- Update cached vault data after create/edit/delete operations
- Atomic cache updates for consistency
- Cache invalidation on errors
- Store encrypted ciphers (as received from API)

**Cache Update Operations**:
```rust
// After create
storage.add_cipher(created_cipher)?;
storage.set("lastSync", current_timestamp)?;

// After edit
storage.update_cipher(item_id, updated_cipher)?;

// After delete (soft)
storage.update_cipher(item_id, cipher_with_deleted_date)?;

// After delete (hard)
storage.remove_cipher(item_id)?;

// After restore
storage.update_cipher(item_id, restored_cipher)?;
```

### INT-2: API Client Integration
**Status**: Dependency on Enhancement 3 (Complete)

**Requirements**:
- Use `ApiClient` for CRUD endpoints
- Handle authentication with Bearer tokens
- Retry transient failures (network errors, 5xx responses)
- Parse API error responses

**API Endpoints**:
```rust
// Create item
api_client.post_with_auth("/api/ciphers", encrypted_item).await?

// Edit item
api_client.put_with_auth(&format!("/api/ciphers/{}", id), encrypted_item).await?

// Delete item (soft)
api_client.delete_with_auth(&format!("/api/ciphers/{}", id)).await?

// Delete item (hard)
api_client.delete_with_auth(&format!("/api/ciphers/{}/permanent", id)).await?

// Restore item
api_client.put_with_auth(&format!("/api/ciphers/{}/restore", id), json!({})).await?

// Create folder
api_client.post_with_auth("/api/folders", folder_data).await?

// Edit folder
api_client.put_with_auth(&format!("/api/folders/{}", id), folder_data).await?

// Create attachment
api_client.post_with_auth(&format!("/api/ciphers/{}/attachment", id), file_data).await?
```

### INT-3: Authentication Integration
**Status**: Dependency on Enhancement 4 (Complete)

**Requirements**:
- Check for valid session before CRUD operations
- Use session key for encryption
- Return authentication error if session expired
- Support `--session` flag for session key

**Session Validation**:
```rust
let session = session_manager.get_session()?;
if session.is_none() {
    return Err("Not authenticated. Run 'bw login' first");
}
```

### INT-4: Bitwarden SDK Integration
**Status**: Dependency on Enhancement 1 (Complete)

**Requirements**:
- Encrypt ciphers using SDK before API submission
- Parse and validate item structures
- Handle encryption errors gracefully
- Use SDK crypto for all encryption operations

**SDK Operations**:
```rust
// Encrypt cipher before create/edit
let encrypted_cipher = sdk_client
    .encrypt_cipher(&cipher_view, &session_key)
    .await?;

// Create cipher object for API
let cipher_request = CipherRequest::from(encrypted_cipher);

// API submission
let created_cipher = api_client
    .post_with_auth("/api/ciphers", &cipher_request)
    .await?;
```

### INT-5: Vault Read Commands Integration
**Status**: Dependency on Enhancement 5 (Complete)

**Requirements**:
- Use existing vault read functionality to retrieve items before edit
- Validate items exist before delete/restore/move operations
- Reuse decryption logic for displaying results
- Leverage search functionality for item lookup

**Read Integration**:
```rust
// Get item before edit
let existing_item = vault_service.get_item(id).await?;

// Validate item exists before delete
let item = vault_service.get_item(id).await
    .map_err(|_| "Item not found")?;

// Get folder for validation
let folder = vault_service.get_folder(folder_id).await?;
```

## Data Models

### Create/Edit Request Structures

**Item Creation Request** (before encryption):
```rust
pub struct CipherView {
    pub id: Option<String>,          // None for create, Some for edit
    pub type: CipherType,            // 1=Login, 2=Note, 3=Card, 4=Identity
    pub name: String,                // Plain text (to be encrypted)
    pub notes: Option<String>,       // Plain text (to be encrypted)
    pub folder_id: Option<String>,
    pub organization_id: Option<String>,
    pub favorite: bool,
    pub login: Option<LoginView>,    // For type=1
    pub card: Option<CardView>,      // For type=3
    pub identity: Option<IdentityView>, // For type=4
    pub secure_note: Option<SecureNoteView>, // For type=2
    pub fields: Vec<FieldView>,      // Custom fields
    pub password_history: Vec<PasswordHistoryView>,
    pub attachments: Vec<AttachmentView>,
}

pub struct LoginView {
    pub username: Option<String>,
    pub password: Option<String>,
    pub uris: Vec<LoginUriView>,
    pub totp: Option<String>,
}
```

**API Request Structure** (after encryption):
```json
{
  "type": 1,
  "name": "2.encrypted_name_data",
  "notes": "2.encrypted_notes_data",
  "login": {
    "username": "2.encrypted_username",
    "password": "2.encrypted_password",
    "uris": [
      {
        "uri": "2.encrypted_uri",
        "match": null
      }
    ],
    "totp": "2.encrypted_totp"
  },
  "folderId": "uuid",
  "favorite": false,
  "fields": [],
  "passwordHistory": []
}
```

### Response Structures

**API Response** (after create/edit):
```json
{
  "id": "uuid",
  "type": 1,
  "name": "2.encrypted_name",
  "revisionDate": "2025-12-05T12:00:00Z",
  "folderId": "uuid",
  ...
}
```

**CLI Output** (decrypted for display):
```json
{
  "id": "uuid",
  "type": 1,
  "name": "GitHub Login",
  "revisionDate": "2025-12-05T12:00:00Z",
  "folderId": "uuid",
  "login": {
    "username": "user@example.com",
    "password": "password123"
  }
}
```

## Success Criteria & Validation

### Validation Approach
1. **Unit Tests**: Test validation, encryption, cache updates
2. **Integration Tests**: Test full CRUD flows with API
3. **Compatibility Tests**: Compare behavior with TypeScript CLI
4. **Security Tests**: Verify encryption and memory handling
5. **Manual Testing**: Real-world usage scenarios

### Acceptance Test Scenarios

**AT-1: Create Item**
```bash
# Given: Authenticated session, item template
bw login user@example.com

# Create item JSON
cat > item.json <<EOF
{
  "type": 1,
  "name": "Test Login",
  "login": {
    "username": "test@example.com",
    "password": "password123",
    "uris": [{"uri": "https://example.com"}]
  }
}
EOF

# When: Create item
ITEM_ID=$(bw create item "$(cat item.json | bw encode)" | jq -r '.id')

# Then: Item created with ID returned
bw get item "$ITEM_ID"
# Expected: Item details match input
```

**AT-2: Edit Item**
```bash
# Given: Existing item
ITEM_ID="existing-item-id"

# When: Edit item
bw get item "$ITEM_ID" | \
  jq '.name = "Updated Name"' | \
  bw encode | \
  bw edit item "$ITEM_ID"

# Then: Item updated
bw get item "$ITEM_ID" | jq -r '.name'
# Expected: "Updated Name"
```

**AT-3: Delete and Restore**
```bash
# Given: Existing item
ITEM_ID="test-item-id"

# When: Soft delete
bw delete item "$ITEM_ID"

# Then: Item in trash
bw list items --trash | jq -r '.[].id'
# Expected: Includes ITEM_ID

# When: Restore
bw restore item "$ITEM_ID"

# Then: Item active again
bw get item "$ITEM_ID"
# Expected: Item accessible, deletedDate null
```

**AT-4: Permanent Delete**
```bash
# Given: Item in trash
ITEM_ID="trashed-item-id"

# When: Permanent delete (with confirmation)
echo "y" | bw delete item "$ITEM_ID" --permanent

# Then: Item completely removed
bw get item "$ITEM_ID"
# Expected: Error "Item not found"
```

**AT-5: Move Item**
```bash
# Given: Item and folder
ITEM_ID="test-item-id"
FOLDER_ID="work-folder-id"

# When: Move item
bw move "$ITEM_ID" "$FOLDER_ID"

# Then: Item in new folder
bw get item "$ITEM_ID" | jq -r '.folderId'
# Expected: FOLDER_ID
```

**AT-6: Validation Errors**
```bash
# Given: Invalid item JSON (missing required field)
cat > invalid.json <<EOF
{
  "type": 1
}
EOF

# When: Create with invalid data
bw create item "$(cat invalid.json | bw encode)"

# Then: Validation error
# Expected: Error "Required field 'name' missing"
```

**AT-7: Create Folder**
```bash
# Given: Authenticated session

# When: Create folder
FOLDER_ID=$(bw create folder "Test Folder" | jq -r '.id')

# Then: Folder created
bw list folders | jq -r '.[] | select(.id == "'$FOLDER_ID'").name'
# Expected: "Test Folder"
```

**AT-8: Cache Update**
```bash
# Given: Synced vault
bw sync

# When: Create item
bw create item "$(echo '{"type":2,"name":"Note"}' | bw encode)"

# Then: Item visible without re-sync
bw list items | grep "Note"
# Expected: Note appears in list
```

## Project Phasing & Implementation Strategy

### Phase 1: MVP Core (Must Have)
**Estimated Effort**: 10-14 days
**Goal**: Basic create, edit, delete, restore, move functionality

**Components**:
1. **Create Commands** (3-4 days)
   - `bw create item` with JSON input and encoding
   - `bw create folder`
   - Item validation before encryption
   - SDK encryption integration
   - Cache updates after create
   - Template generation (`bw get template`)

2. **Edit Commands** (2-3 days)
   - `bw edit item <id>` with JSON update
   - `bw edit folder <id>`
   - Retrieve existing item first
   - Validation and re-encryption
   - Cache updates after edit

3. **Delete Commands** (2-3 days)
   - `bw delete item <id>` (soft delete)
   - `bw delete item <id> --permanent` with confirmation
   - `bw delete folder <id>`
   - Cache updates after delete

4. **Restore & Move Commands** (2-3 days)
   - `bw restore item <id>`
   - `bw move <itemId> <folderId>`
   - Validation and cache updates

5. **Input Validation** (1-2 days)
   - JSON structure validation
   - Required field validation
   - Type validation
   - UUID validation
   - Error message formatting

**Phase 1 Exit Criteria**:
- [ ] All MVP commands implemented
- [ ] Encryption working correctly with SDK
- [ ] Cache updates atomic and consistent
- [ ] Validation prevents invalid submissions
- [ ] Basic integration tests pass
- [ ] Can create, edit, delete items in real vault

### Phase 2: Extended Features (Should Have)
**Estimated Effort**: 5-7 days
**Goal**: Attachments, organization features, advanced operations

**Components**:
1. **Attachment Commands** (2-3 days)
   - `bw create attachment --file <path> --itemid <id>`
   - `bw delete attachment <id> --itemid <id>`
   - File encryption and upload
   - Progress indication

2. **Organization Features** (2-3 days)
   - `bw create org-collection`
   - `bw edit org-collection`
   - `bw delete org-collection`
   - `bw share <itemId> <orgId>`
   - `bw confirm org-member`
   - `bw edit item-collections`

3. **Advanced Operations** (1 day)
   - Batch operation support (if time)
   - Better error recovery

**Phase 2 Exit Criteria**:
- [ ] All "Should Have" features implemented
- [ ] Attachment upload working with progress
- [ ] Organization features tested
- [ ] Permission validation working

### Phase 3: Polish & Optimization (Nice to Have)
**Estimated Effort**: 2-3 days
**Goal**: UX improvements, performance optimization

**Components**:
1. **UX Improvements**
   - Better error messages with hints
   - Confirmation prompts polish
   - Success message improvements
   - Help text refinement

2. **Performance Optimization**
   - Parallel encryption for multiple items
   - Cache update optimization
   - Error recovery improvements

3. **Additional Features** (if time)
   - Undo functionality exploration
   - Batch operations

**Phase 3 Exit Criteria**:
- [ ] User experience polished
- [ ] Error messages clear and helpful
- [ ] Performance benchmarks meet targets

### Implementation Sequence
```
Prerequisites: Enhancements 1-5 complete
    ↓
Phase 1 (MVP Core)
├── Create commands (item, folder, templates)
├── Edit commands (item, folder)
├── Delete commands (soft, permanent, folder)
├── Restore command
├── Move command
└── Input validation
    ↓
Phase 2 (Extended)
├── Attachment commands
├── Organization collection management
└── Share and confirm commands
    ↓
Phase 3 (Polish)
├── UX improvements
└── Performance optimization
```

**Critical Path**: Phase 1 blocks enhancement 7 (tool commands) and 8 (import/export)

## Open Questions & Clarification Needs

### OQ-1: Item Conflict Handling During Edit
**Question**: How should we handle item conflicts when editing (local changes vs server changes)?

**Context**: If item modified on server since last sync, edit may conflict.

**Options**:
1. Last-write-wins (overwrite server version) - simple, may lose data
2. Detect conflicts and error (require re-sync first) - safer but annoying
3. Merge changes (complex, error-prone)

**Recommendation**: Option 1 (last-write-wins) to match TypeScript CLI behavior. API handles revision tracking. Users responsible for sync before edit.

**Decision Needed**: Confirm last-write-wins acceptable.

### OQ-2: Item Data Validation Depth
**Question**: How deeply should we validate item data structure before sending to API?

**Context**: Balancing client-side validation vs letting API validate.

**Options**:
1. Minimal validation (type, name only) - fast but more API errors
2. Complete validation (all fields, types, formats) - slower but fewer errors
3. Schema-based validation (JSON schema) - comprehensive but complex

**Recommendation**: Option 2 (complete validation) to match TypeScript CLI. Prevent API round-trips with bad data. Validate required fields, types, UUIDs, max lengths.

**Decision Needed**: Confirm validation scope - which fields must be validated?

### OQ-3: Attachment Upload Efficiency
**Question**: How should we handle large attachment uploads efficiently?

**Context**: Attachments up to 100MB may take time to upload.

**Options**:
1. Single upload with progress bar - simple, no resume
2. Chunked upload with resume support - complex but robust
3. Streaming upload - efficient memory, moderate complexity

**Recommendation**: Option 1 (single upload with progress) for MVP. Chunked upload adds significant complexity. Most attachments are small.

**Decision Needed**: Confirm single upload acceptable, no resume needed.

### OQ-4: Batch Operations Support
**Question**: Should we support batch create/edit/delete operations?

**Context**: Creating/updating many items one at a time is inefficient.

**Options**:
1. No batch support (one at a time) - simple, familiar
2. Batch commands (e.g., `bw create items --file items.json`) - efficient
3. Parallel execution (internal optimization) - transparent

**Recommendation**: No batch support for MVP (option 1). Add in future if users request. Scripting can loop with parallel execution.

**Decision Needed**: Confirm no batch operations for initial release.

### OQ-5: Partial Update Failure Handling
**Question**: How should we handle partial failures in multi-step operations (e.g., encrypt succeeds but API fails)?

**Context**: Operations have multiple steps that can fail independently.

**Options**:
1. Rollback on any failure (transaction-like) - complex, not supported by API
2. Best-effort with clear error (cache not updated on failure) - simple
3. Partial success (cache some updates) - confusing

**Recommendation**: Option 2 (best-effort). If API fails, cache unchanged, clear error message. User can retry. No rollback needed since no partial state.

**Decision Needed**: Confirm best-effort error handling acceptable.

### OQ-6: Delete Confirmation Behavior
**Question**: Should we prompt for confirmation on soft delete (to trash) or only permanent delete?

**Context**: Balance safety vs usability.

**Options**:
1. Confirm only on `--permanent` delete - less annoying, trash is safety net
2. Confirm on all deletes - safer but cumbersome
3. Add `--force` flag to skip confirmation - flexible

**Recommendation**: Option 1 (confirm only permanent delete) to match TypeScript CLI. Soft delete is recoverable, so no confirmation needed. Permanent delete shows prompt unless `--nointeraction`.

**Decision Needed**: Confirm confirmation only for permanent delete.

### OQ-7: Template Format
**Question**: What format should `bw get template` return?

**Context**: Users need templates to create items correctly.

**Options**:
1. Minimal template (only required fields) - simple but incomplete
2. Complete template (all fields with null/defaults) - comprehensive
3. Documented template with comments - helpful but invalid JSON

**Recommendation**: Option 2 (complete template) to match TypeScript CLI. Include all possible fields with appropriate defaults. Users can remove unneeded fields.

**Decision Needed**: Confirm complete template format matches TypeScript CLI.

### OQ-8: Cache Update Atomicity
**Question**: How should we ensure cache updates are atomic?

**Context**: Cache corruption risk if process interrupted during write.

**Options**:
1. Write entire cache file atomically (temp file + rename) - safe but overwrites all
2. Use transactional storage (SQLite, etc.) - robust but complex
3. Write-ahead log for incremental updates - complex

**Recommendation**: Option 1 (atomic file write) using storage layer's existing atomic write support. Write complete cache to temp file, then rename. Loss of incremental update efficiency acceptable for reliability.

**Decision Needed**: Confirm atomic file write acceptable (handled by storage layer).

## Risk Assessment & Mitigation

### Risk 1: Data Loss from Destructive Operations
**Severity**: Critical
**Probability**: Medium

**Description**: Permanent delete or edit operations could cause data loss if user makes mistake.

**Mitigation**:
- Confirmation prompts for `--permanent` delete
- Soft delete as default (recoverable)
- Validation before submission
- Clear error messages
- Document backup recommendations

**Contingency**: If data loss occurs, user must rely on server revision history (outside CLI scope). Ensure user documentation emphasizes backup importance.

### Risk 2: SDK Encryption Failures
**Severity**: High
**Probability**: Low

**Description**: SDK encryption operations may fail or produce incorrect results, preventing item creation/editing.

**Mitigation**:
- Use SDK exclusively (no manual crypto)
- Comprehensive error handling on SDK calls
- Test with all cipher types
- Validate encrypted output format
- Log encryption errors (without sensitive data)

**Contingency**: If SDK encryption bugs found, work with SDK team to resolve. Create test cases demonstrating issue.

### Risk 3: Cache Consistency Issues
**Severity**: High
**Probability**: Medium

**Description**: Cache may become inconsistent with server state after failed operations or network issues.

**Mitigation**:
- Atomic cache updates (write temp file, rename)
- Never update cache on operation failure
- Clear error messages when cache/server out of sync
- Document when to run `bw sync` to refresh
- Validate cache state before operations

**Contingency**: If cache corruption occurs, user runs `bw sync --force` to rebuild. Document troubleshooting steps.

### Risk 4: API Error Handling Complexity
**Severity**: Medium
**Probability**: High

**Description**: API may return various error types (validation, permission, network) that need proper handling.

**Mitigation**:
- Parse API error responses comprehensively
- Map API errors to clear user messages
- Handle all HTTP status codes (400, 401, 403, 404, 500, 503)
- Retry transient errors (network, 5xx)
- Log detailed errors for debugging

**Contingency**: If unexpected API error encountered, show generic error with details and suggest filing issue.

### Risk 5: Validation Rule Drift from TypeScript CLI
**Severity**: Medium
**Probability**: Medium

**Description**: Rust CLI validation rules may diverge from TypeScript CLI, causing compatibility issues.

**Mitigation**:
- Reference TypeScript CLI validation logic directly
- Create compatibility test suite comparing validation
- Document validation rules explicitly
- Test with same test data as TypeScript CLI
- Monitor TypeScript CLI changes

**Contingency**: If divergence detected, update Rust validation to match. Document intentional differences if required.

### Risk 6: Encryption Performance Issues
**Severity**: Low
**Probability**: Low

**Description**: SDK encryption may be slow for batch operations or large items.

**Mitigation**:
- Benchmark encryption performance early
- Profile SDK calls for bottlenecks
- Consider parallel encryption if needed
- Show progress for long operations
- Set realistic performance expectations

**Contingency**: If performance unacceptable, work with SDK team on optimization or implement workarounds.

## Dependencies & Blockers

### Blockers (Must Complete Before Starting)
1. ✅ **Enhancement 1**: Project Bootstrap - Required for SDK setup
2. ✅ **Enhancement 2**: Storage Layer - Required for cache updates
3. ✅ **Enhancement 3**: API Client - Required for CRUD endpoints
4. ✅ **Enhancement 4**: Authentication - Required for session management
5. ✅ **Enhancement 5**: Vault Read Operations - Required for item retrieval and validation

**Status**: All blockers complete, ready to proceed with architecture phase.

### External Dependencies
1. **Bitwarden SDK** (`bitwarden-core` crate)
   - Used for: Cipher encryption, data validation
   - Status: Available at `../sdk/`
   - Critical for: All create/edit operations

2. **Bitwarden API**
   - Used for: CRUD endpoints
   - Status: Production API documented
   - Authentication: Bearer token
   - Endpoints: `/api/ciphers`, `/api/folders`, etc.

3. **Storage Layer** (Enhancement 2)
   - Used for: Cache updates after operations
   - Interface: `Storage` trait with atomic writes
   - Status: Complete

4. **API Client** (Enhancement 3)
   - Used for: HTTP requests to Bitwarden API
   - Interface: `ApiClient` trait
   - Status: Complete

5. **Vault Read Commands** (Enhancement 5)
   - Used for: Item retrieval before edit/delete
   - Interface: Vault service and commands
   - Status: Complete

## Technical Flags for Architect

### Architecture Decision Points

**AD-1: Validation Layer Design**
- **Decision Needed**: How to structure validation logic for reusability
- **Options**: Separate validator service vs inline validation vs trait-based
- **Recommendation**: Create `ItemValidator` service with separate validation functions per item type
- **Rationale**: Testable, reusable, extensible for new validation rules

**AD-2: Encryption Workflow**
- **Decision Needed**: Where to integrate SDK encryption in command flow
- **Options**: In command handlers vs separate service layer vs in API client
- **Recommendation**: Create `EncryptionService` wrapper around SDK, called by command handlers before API submission
- **Rationale**: Separation of concerns, testable with mocks, clear error handling

**AD-3: Cache Update Strategy**
- **Decision Needed**: When and how to update local cache after operations
- **Options**: Optimistic (update before API) vs pessimistic (update after API) vs manual sync
- **Recommendation**: Pessimistic update - only update cache after successful API response
- **Rationale**: Consistency, no cache corruption on API failures, matches TypeScript CLI

**AD-4: Error Handling Architecture**
- **Decision Needed**: How to structure error types for CRUD operations
- **Options**: Generic Result type vs domain-specific error enum vs error context
- **Recommendation**: Domain-specific error enum (`VaultWriteError`) with context variants for validation, encryption, API, cache errors
- **Rationale**: Clear error types, specific error messages, easy to handle different failure modes

**AD-5: Command Structure**
- **Decision Needed**: How to organize create/edit/delete commands
- **Options**: Separate modules vs single module with subcommands vs flat structure
- **Recommendation**: Separate modules (`commands::create`, `commands::edit`, etc.) with shared services
- **Rationale**: Clear organization, easy navigation, separate concerns

**AD-6: Confirmation Prompt Design**
- **Decision Needed**: How to implement confirmation prompts
- **Options**: Inline in commands vs shared prompt service vs trait
- **Recommendation**: Create shared `ConfirmationService` with `confirm()` method respecting `--nointeraction` flag
- **Rationale**: Reusable, consistent behavior, testable

### High-Level Technical Challenges

**TC-1: SDK Encryption Integration**
- **Challenge**: Correctly encrypting all item fields using SDK API
- **Approach**: Study SDK examples, create wrapper service, comprehensive testing
- **Complexity**: High (SDK API learning curve, multiple cipher types)

**TC-2: Input Validation Completeness**
- **Challenge**: Validating all item fields comprehensively before submission
- **Approach**: Reference TypeScript CLI validation, create validation matrix, test with invalid data
- **Complexity**: Medium (many fields and types to validate)

**TC-3: Cache Consistency**
- **Challenge**: Ensuring cache always consistent with server after operations
- **Approach**: Atomic updates, error handling, clear rollback strategy
- **Complexity**: Medium (error handling complexity)

**TC-4: Destructive Operation Safety**
- **Challenge**: Preventing accidental data loss from delete operations
- **Approach**: Confirmation prompts, soft delete default, clear warnings
- **Complexity**: Low (straightforward implementation)

**TC-5: API Error Response Handling**
- **Challenge**: Handling various API error types and providing clear messages
- **Approach**: Parse API error JSON, map to user-friendly messages, test error scenarios
- **Complexity**: Medium (many error types to handle)

**TC-6: Attachment Upload**
- **Challenge**: Efficient upload of large files with progress indication
- **Approach**: Streaming upload, progress callbacks, chunked reading
- **Complexity**: Medium to High (file handling, progress tracking)

## Documentation Requirements

### User Documentation
1. **Command Help Text** - Document all write commands, flags, examples
2. **Item Creation Guide** - How to create different item types with templates
3. **Validation Error Reference** - Common validation errors and solutions
4. **Destructive Operations Guide** - Safe practices for delete operations
5. **Troubleshooting Guide** - Common errors and solutions

### Developer Documentation
1. **Architecture Overview** - Design of validation, encryption, and cache update
2. **SDK Integration Guide** - How to use encryption APIs correctly
3. **Error Handling Guide** - Error types and handling patterns
4. **Testing Guide** - How to test CRUD operations
5. **Cache Consistency Strategy** - How atomic updates work

## Appendix: Reference Materials

### TypeScript CLI References
Referenced in specification:
- `apps/cli/src/vault/commands/create.command.ts` - Create command implementation
- `apps/cli/src/vault/commands/edit.command.ts` - Edit command implementation
- `apps/cli/src/vault/commands/delete.command.ts` - Delete command implementation
- `apps/cli/src/vault/commands/restore.command.ts` - Restore command implementation
- `apps/cli/src/vault/commands/share.command.ts` - Share command implementation
- `apps/cli/src/vault/commands/move.command.ts` - Move command implementation

### Bitwarden SDK References
- [SDK Cipher Encryption](https://sdk-api-docs.bitwarden.com/) - Cipher encryption APIs
- [SDK Crypto Crate](https://sdk-api-docs.bitwarden.com/bitwarden_crypto/index.html) - Cryptographic operations

### API Documentation
- [Bitwarden CLI Help](https://bitwarden.com/help/cli/) - Official CLI documentation
- [Vaultwarden API Reference](https://deepwiki.com/dani-garcia/vaultwarden/3-api-reference) - API endpoint documentation

### Related Enhancements
- **Enhancement 1**: Project Bootstrap - SDK setup
- **Enhancement 2**: Storage Layer - Atomic cache updates
- **Enhancement 3**: API Client - CRUD API endpoints
- **Enhancement 4**: Authentication - Session management
- **Enhancement 5**: Vault Read Operations - Item retrieval and decryption

---

## Summary for Next Phase (Architecture)

**Readiness**: ✅ Ready for architecture phase

**Key Deliverables for Architect**:
1. Design `ItemValidator` service for comprehensive validation
2. Design `EncryptionService` wrapper around SDK
3. Design `ConfirmationService` for destructive operations
4. Design cache update strategy with atomic writes
5. Design error handling with domain-specific error types
6. Design command module structure and organization
7. Design data flow for create/edit/delete operations
8. Design testing strategy including encryption and validation tests

**Critical Decisions for Architect**:
- Validation layer structure and rules
- Encryption workflow integration with SDK
- Cache update timing and atomicity
- Error type hierarchy and propagation
- Command organization and shared services
- Confirmation prompt implementation
- API error response handling

**Success Criteria for Architecture Phase**:
- [ ] Clear module organization defined (commands, services, validation)
- [ ] Data models specified for requests and responses
- [ ] SDK encryption integration pattern documented
- [ ] Validation rules documented comprehensively
- [ ] Cache update strategy documented with error handling
- [ ] Error handling strategy documented with error types
- [ ] Confirmation prompt design documented
- [ ] Testing strategy defined (unit, integration, security)
- [ ] Performance considerations identified
- [ ] Security measures documented

---

**Requirements Analysis Status**: ✅ COMPLETE
**Next Agent**: Architect
**Workflow Status**: READY_FOR_ARCHITECTURE
