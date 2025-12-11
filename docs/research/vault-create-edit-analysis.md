# Vault Create/Edit Commands Analysis

This document analyzes the vault create, edit, delete, restore, and move commands to inform the implementation of CLI commands that connect our existing bw-core WriteService to user-facing commands.

## Current State

### Existing Core Infrastructure (bw-core)

Our bw-core crate already has a fully implemented `WriteService` that handles the core business logic:

**Location:** `crates/bw-core/src/services/vault/write_service.rs`

| Method | Purpose | Status |
|--------|---------|--------|
| `create_cipher()` | Create new vault item | Implemented |
| `update_cipher()` | Update existing item | Implemented |
| `delete_cipher()` | Soft/permanent delete | Implemented |
| `restore_cipher()` | Restore from trash | Implemented |
| `move_cipher()` | Change item folder | Implemented |
| `create_folder()` | Create new folder | Implemented |
| `update_folder()` | Update folder name | Implemented |
| `delete_folder()` | Delete folder | Implemented |

### Existing CLI Stubs (bw-cli)

**Location:** `crates/bw-cli/src/commands/vault.rs`

CLI command handlers exist but return "Not yet implemented":
- `execute_create()` - lines 496-501
- `execute_edit()` - lines 504-509
- `execute_delete()` - lines 512-517
- `execute_restore()` - lines 520-525
- `execute_move()` - lines 528-533

### Data Models

Our `CipherView` model is comprehensive and matches the TypeScript CLI's `CipherExport` structure:

**Location:** `crates/bw-core/src/models/vault/cipher.rs`

| Field | Type | Notes |
|-------|------|-------|
| `cipher_type` | `CipherType` enum | Login=1, SecureNote=2, Card=3, Identity=4, SshKey=5 |
| `name` | `String` | Required, max 1000 chars |
| `notes` | `Option<String>` | Optional, max 10000 chars |
| `folder_id` | `Option<String>` | UUID of folder or null |
| `organization_id` | `Option<String>` | UUID if shared |
| `login` | `Option<CipherLoginView>` | For type=Login |
| `card` | `Option<CipherCardView>` | For type=Card |
| `identity` | `Option<CipherIdentityView>` | For type=Identity |
| `fields` | `Vec<CipherFieldView>` | Custom fields |

## TypeScript CLI Behavior Analysis

### Input Format

The TypeScript CLI accepts **base64-encoded JSON**:

```typescript
// create.command.ts:68-69
const reqJson = Buffer.from(requestJson, "base64").toString();
req = JSON.parse(reqJson);
```

**Workflow:**
1. User creates JSON using `bw get template item.login`
2. User edits the template
3. User base64-encodes: `echo '{"type":1,...}' | base64`
4. User runs: `bw create item <base64-string>`

Alternatively, data can be piped via stdin.

### Create Flow (TypeScript)

```typescript
// create.command.ts
async createCipher(req: CipherExport) {
  const cipherView = CipherExport.toView(req);         // 1. Parse JSON to view
  const isCipherTypeRestricted = ...;                  // 2. Check org restrictions
  const cipher = await this.cipherService.encrypt(...); // 3. Encrypt
  const newCipher = await this.cipherService.createWithServer(cipher); // 4. API call
  const decCipher = await this.cipherService.decrypt(newCipher); // 5. Decrypt response
  return Response.success(new CipherResponse(decCipher)); // 6. Return decrypted
}
```

### Edit Flow (TypeScript)

```typescript
// edit.command.ts
async editCipher(id: string, req: CipherExport) {
  const cipher = await this.cipherService.get(id);     // 1. Get existing
  let cipherView = await this.cipherService.decrypt(cipher); // 2. Decrypt
  if (cipherView.isDeleted) return Response.badRequest(...); // 3. Check not deleted
  cipherView = CipherExport.toView(req, cipherView);   // 4. Merge changes
  const encCipher = await this.cipherService.encrypt(cipherView); // 5. Re-encrypt
  const updatedCipher = await this.cipherService.updateWithServer(encCipher); // 6. API
  return Response.success(...);
}
```

**Key insight:** Edit merges new data into existing cipher, preserving unchanged fields.

### Delete Flow (TypeScript)

```typescript
// delete.command.ts
async deleteCipher(id: string, options: Options) {
  const cipher = await this.cipherService.get(id);
  // Permission checks...
  if (options.permanent) {
    await this.cipherService.deleteWithServer(id);
  } else {
    await this.cipherService.softDeleteWithServer(id);
  }
  return Response.success();
}
```

### Restore Flow (TypeScript)

```typescript
// restore.command.ts
async restoreCipher(id: string) {
  const cipher = await this.cipherService.get(id);
  if (cipher.deletedDate == null) {
    return Response.badRequest("Cipher is not in trash.");
  }
  await this.cipherService.restoreWithServer(id);
  return Response.success();
}
```

## Implementation Gap Analysis

### What We Have

| Component | Status | Location |
|-----------|--------|----------|
| WriteService core logic | Complete | `bw-core/src/services/vault/write_service.rs` |
| CipherView model | Complete | `bw-core/src/models/vault/cipher.rs` |
| CipherService encryption | Complete | `bw-core/src/services/vault/cipher_service.rs` |
| ValidationService | Complete | `bw-core/src/services/vault/validation_service.rs` |
| API endpoints module | Complete | `bw-core/src/services/api/endpoints.rs` |
| CLI command stubs | Stubs only | `bw-cli/src/commands/vault.rs` |

### What We Need to Implement

1. **JSON Input Parsing**: Base64 decode → JSON parse → CipherView
2. **CLI Command Handlers**: Connect to WriteService
3. **Template Generation**: `bw get template` for item types
4. **Response Formatting**: Return decrypted cipher after create/edit

## Data Flow: Create Item

```
User Input (base64 JSON)
        ↓
[1] CLI: Base64 decode + JSON parse → CipherView
        ↓
[2] CLI: Get session key from --session or BW_SESSION
        ↓
[3] CLI: Call WriteService.create_cipher(cipher_view, session)
        ↓
[4] WriteService: Validate via ValidationService
        ↓
[5] WriteService: Get user key from session key
        ↓
[6] WriteService: Encrypt via CipherService.encrypt_cipher()
        ↓
[7] WriteService: POST to API /api/ciphers
        ↓
[8] WriteService: Update local cache
        ↓
[9] CLI: Decrypt response and format output
```

## Data Flow: Edit Item

```
User Input (id + base64 JSON)
        ↓
[1] CLI: Get existing cipher from cache
        ↓
[2] CLI: Decrypt existing cipher
        ↓
[3] CLI: Parse new JSON, merge with existing
        ↓
[4] CLI: Call WriteService.update_cipher(id, merged_view, session)
        ↓
[5] WriteService: Same as create from step 4 onwards
```

## Best Practices Integration

Per our code quality guidelines (`docs/bw-core-code-quality-issues.md`):

### Use Existing Patterns

- **Endpoints module**: Use `endpoints::api::ciphers::BASE` etc.
- **Validation limits**: Use `limits::CIPHER_NAME_MAX_LEN` constants
- **Error handling**: No `.unwrap()` on user input; propagate errors

### Security Considerations

- Never log decrypted cipher data
- Use `Secret<String>` for sensitive inputs where possible
- Clear sensitive buffers via zeroization patterns
- Validate all user input before encryption/API calls

### Code Organization

```rust
// Proposed structure in bw-cli/src/commands/vault.rs

// Input parsing helpers
fn parse_cipher_json(input: &str) -> Result<CipherView, CliError>;
fn merge_cipher_view(existing: &CipherView, updates: &CipherView) -> CipherView;

// Command implementations
pub async fn execute_create(...) -> Result<Response> {
    // 1. Parse input
    // 2. Call WriteService
    // 3. Format response
}
```

## Template Generation

Templates should match TypeScript CLI format for compatibility:

```json
{
  "organizationId": null,
  "collectionIds": null,
  "folderId": null,
  "type": 1,
  "name": "Item name",
  "notes": "Some notes about this item.",
  "favorite": false,
  "fields": [],
  "login": {
    "uris": [
      {
        "match": null,
        "uri": "https://example.com"
      }
    ],
    "username": "jdoe",
    "password": "myp@ssword123",
    "totp": null
  },
  "reprompt": 0
}
```

## Error Handling

| Error Scenario | TypeScript Response | Rust Implementation |
|---------------|---------------------|---------------------|
| Missing requestJson | `Response.badRequest()` | Return error response |
| Invalid base64 | `Response.badRequest("Error parsing...")` | Return parse error |
| Item not found | `Response.notFound()` | `VaultError::ItemNotFound` |
| Editing deleted item | `Response.badRequest("...deleted...")` | Check `deleted_date` |
| Validation failure | `Response.error(e)` | `ValidationError` |
| API error | `Response.error(e)` | `VaultError::ApiError` |

## Testing Strategy

### Unit Tests

1. **JSON parsing**: Valid/invalid base64, valid/invalid JSON
2. **Merge logic**: Verify field preservation during edit
3. **Template generation**: All item types produce valid templates

### Integration Tests

1. **Create flow**: JSON → create → verify in cache
2. **Edit flow**: Create → edit → verify changes
3. **Delete/restore**: Create → delete → restore → verify
4. **Error cases**: Invalid IDs, missing auth, validation failures

### Compatibility Tests

1. Compare output format with TypeScript CLI
2. Verify created items can be read by TypeScript CLI
3. Verify TypeScript-created items can be edited by Rust CLI

## Scope Recommendations

### MVP (Must Have)

- [x] `bw create item <json>` - Create vault item
- [x] `bw create folder <json>` - Create folder
- [x] `bw edit item <id> <json>` - Edit vault item
- [x] `bw edit folder <id> <json>` - Edit folder
- [x] `bw delete item <id>` - Soft delete to trash
- [x] `bw delete item <id> --permanent` - Permanent delete
- [x] `bw delete folder <id>` - Delete folder
- [x] `bw restore item <id>` - Restore from trash
- [x] `bw move <id> <folderId>` - Move to folder
- [x] `bw get template item.login` etc. - Templates
- [x] Input validation
- [x] Error handling

### Should Have (Phase 2)

- [ ] `bw create attachment` - File attachments
- [ ] `bw delete attachment` - Remove attachment
- [ ] `bw edit item-collections` - Update collection membership
- [ ] `bw create org-collection` - Organization collections
- [ ] `bw share` - Move item to organization

### Won't Have (Out of Scope)

- GUI editing
- Interactive prompts for item creation
- Batch operations
- Undo functionality

## Summary

The implementation is straightforward because:

1. **Core logic exists**: WriteService in bw-core handles all business logic
2. **Models are complete**: CipherView matches TypeScript CipherExport
3. **API layer is ready**: BitwardenApiClient and endpoints module
4. **Clear pattern**: TypeScript CLI provides exact behavior to match

Main work is:
1. JSON input parsing (base64 decode + serde)
2. Connecting CLI handlers to WriteService
3. Response formatting
4. Template generation
5. Testing
