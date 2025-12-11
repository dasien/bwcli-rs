---
enhancement: 12-vault-create-edit
agent: architect
task_id: task_1765414487_22144
timestamp: 2025-12-10T16:30:00Z
status: READY_FOR_IMPLEMENTATION
---

# Implementation Plan: Vault Create/Edit CLI Commands

## Executive Summary

This plan defines the architecture for connecting existing `WriteService` functionality to CLI command handlers. The core business logic (encryption, validation, API calls, caching) is already implemented in `bw-core`. This implementation focuses on:

1. **Input parsing module** - Base64/JSON input handling with stdin support
2. **Template generation** - TypeScript CLI-compatible JSON templates
3. **Command handler implementation** - Wire up existing WriteService methods
4. **Output formatting** - Return decrypted item views on success

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                           CLI Layer (bw-cli)                        │
├─────────────────────────────────────────────────────────────────────┤
│  commands/vault.rs                                                  │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐  ┌────────────┐   │
│  │ execute_   │  │ execute_   │  │ execute_   │  │ execute_   │   │
│  │ create()   │  │ edit()     │  │ delete()   │  │ restore()  │   │
│  └─────┬──────┘  └─────┬──────┘  └─────┬──────┘  └─────┬──────┘   │
│        │               │               │               │           │
│  ┌─────▼───────────────▼───────────────▼───────────────▼─────┐    │
│  │                 Input Parser (input.rs)                    │    │
│  │   parse_item_input() / parse_folder_input()               │    │
│  │   - Base64 decode (TypeScript CLI compat)                 │    │
│  │   - Raw JSON parsing (starts with '{')                    │    │
│  │   - Stdin reading                                         │    │
│  └───────────────────────────┬───────────────────────────────┘    │
│                              │                                     │
│  ┌───────────────────────────▼───────────────────────────────┐    │
│  │                Template Module (templates.rs)              │    │
│  │   get_item_template() / get_folder_template()             │    │
│  └───────────────────────────────────────────────────────────┘    │
└─────────────────────────────────┬───────────────────────────────────┘
                                  │
┌─────────────────────────────────▼───────────────────────────────────┐
│                         Core Layer (bw-core)                        │
├─────────────────────────────────────────────────────────────────────┤
│  WriteService (existing)         │  VaultService (existing)         │
│  ┌─────────────────────────┐    │  ┌─────────────────────────┐     │
│  │ create_cipher()         │    │  │ get_item() → CipherView │     │
│  │ update_cipher()         │    │  └─────────────────────────┘     │
│  │ delete_cipher()         │    │                                   │
│  │ restore_cipher()        │    │                                   │
│  │ move_cipher()           │    │                                   │
│  │ create_folder()         │    │                                   │
│  │ update_folder()         │    │                                   │
│  │ delete_folder()         │    │                                   │
│  └─────────────────────────┘    │                                   │
└─────────────────────────────────────────────────────────────────────┘
```

## Component Design

### 1. Input Parser Module (`bw-cli/src/commands/input.rs`)

**Purpose:** Parse JSON input from CLI arguments in multiple formats

**Design Pattern:** Strategy-based input detection

```rust
/// Parse item JSON input from various formats
///
/// Supports:
/// 1. Base64-encoded JSON (TypeScript CLI compatible)
/// 2. Raw JSON (detected by leading '{')
/// 3. Stdin (detected by "-" argument)
pub fn parse_item_input(input: &str) -> Result<CipherView, InputError> {
    // Detection logic:
    // 1. If input == "-", read from stdin
    // 2. If input starts with '{', parse as raw JSON
    // 3. Otherwise, base64 decode then parse JSON
}

/// Parse folder JSON input
pub fn parse_folder_input(input: &str) -> Result<FolderInput, InputError> {
    // Same detection logic as parse_item_input
}

/// Folder input structure (subset of FolderView for creation)
pub struct FolderInput {
    pub name: String,
}
```

**Error Types:**
```rust
#[derive(Error, Debug)]
pub enum InputError {
    #[error("Invalid base64 encoding: {0}")]
    Base64DecodeError(String),

    #[error("Invalid JSON: {0}")]
    JsonParseError(String),

    #[error("Failed to read stdin: {0}")]
    StdinError(String),

    #[error("Missing required field: {0}")]
    MissingField(String),
}
```

**Stdin Detection:**
```rust
fn read_stdin() -> Result<String, InputError> {
    use std::io::{self, Read};
    let mut buffer = String::new();
    io::stdin()
        .read_to_string(&mut buffer)
        .map_err(|e| InputError::StdinError(e.to_string()))?;
    Ok(buffer.trim().to_string())
}
```

### 2. Template Module (`bw-cli/src/commands/templates.rs`)

**Purpose:** Generate JSON templates for CLI users

**Design Decision:** Use static JSON templates (not programmatic CipherView construction) for exact TypeScript CLI compatibility.

```rust
/// Get item template by type
pub fn get_item_template(template_type: &str) -> Result<Value, TemplateError> {
    match template_type {
        "item" | "item.login" => Ok(login_template()),
        "item.secureNote" | "item.securenote" => Ok(secure_note_template()),
        "item.card" => Ok(card_template()),
        "item.identity" => Ok(identity_template()),
        "folder" => Ok(folder_template()),
        "item.field" => Ok(field_template()),
        "item.login.uri" => Ok(uri_template()),
        _ => Err(TemplateError::UnknownTemplate(template_type.to_string())),
    }
}
```

**Template Structures** (matching TypeScript CLI exactly):

```rust
fn login_template() -> Value {
    json!({
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
        "secureNote": null,
        "card": null,
        "identity": null,
        "reprompt": 0
    })
}

fn secure_note_template() -> Value {
    json!({
        "organizationId": null,
        "collectionIds": null,
        "folderId": null,
        "type": 2,
        "name": "Item name",
        "notes": "Some notes about this item.",
        "favorite": false,
        "fields": [],
        "login": null,
        "secureNote": {
            "type": 0
        },
        "card": null,
        "identity": null,
        "reprompt": 0
    })
}

fn card_template() -> Value {
    json!({
        "organizationId": null,
        "collectionIds": null,
        "folderId": null,
        "type": 3,
        "name": "Item name",
        "notes": "Some notes about this item.",
        "favorite": false,
        "fields": [],
        "login": null,
        "secureNote": null,
        "card": {
            "cardholderName": "John Doe",
            "brand": "visa",
            "number": "4242424242424242",
            "expMonth": "04",
            "expYear": "2025",
            "code": "123"
        },
        "identity": null,
        "reprompt": 0
    })
}

fn identity_template() -> Value {
    json!({
        "organizationId": null,
        "collectionIds": null,
        "folderId": null,
        "type": 4,
        "name": "Item name",
        "notes": "Some notes about this item.",
        "favorite": false,
        "fields": [],
        "login": null,
        "secureNote": null,
        "card": null,
        "identity": {
            "title": "Mr",
            "firstName": "John",
            "middleName": "William",
            "lastName": "Doe",
            "address1": "123 Main St",
            "address2": "Apt 1",
            "address3": null,
            "city": "New York",
            "state": "NY",
            "postalCode": "10001",
            "country": "US",
            "company": "Acme Inc",
            "email": "jdoe@example.com",
            "phone": "555-123-4567",
            "ssn": "123-45-6789",
            "username": "jdoe",
            "passportNumber": "123456789",
            "licenseNumber": "D1234567"
        },
        "reprompt": 0
    })
}

fn folder_template() -> Value {
    json!({
        "name": "Folder name"
    })
}

fn field_template() -> Value {
    json!({
        "name": "Field name",
        "value": "Some value",
        "type": 0
    })
}

fn uri_template() -> Value {
    json!({
        "match": null,
        "uri": "https://example.com"
    })
}
```

### 3. WriteService Helper Factory

**Purpose:** Create WriteService instances in CLI commands

**Location:** `bw-cli/src/commands/vault.rs`

```rust
/// Create a WriteService instance with all dependencies
fn create_write_service(ctx: &AppContext) -> WriteService {
    let account_manager = Arc::new(AccountManager::new(ctx.storage()));
    let cipher_service = Arc::new(CipherService::new(Arc::new(ctx.sdk().clone())));
    let validation_service = Arc::new(ValidationService::new());
    let confirmation_service = Arc::new(ConfirmationService::new(/* nointeraction from global */));

    WriteService::new(
        ctx.api_client(),
        ctx.storage(),
        cipher_service,
        validation_service,
        confirmation_service,
        account_manager,
    )
}
```

### 4. Command Handler Implementations

#### 4.1 Create Item (`bw create item <json>`)

```rust
pub async fn execute_create(
    cmd: CreateCommands,
    global_args: &GlobalArgs,
    ctx: &AppContext,
) -> anyhow::Result<Response> {
    match cmd {
        CreateCommands::Item(item_cmd) => {
            let session = get_session(global_args)?;

            // 1. Parse input (base64/JSON/stdin)
            let cipher_view = parse_item_input(&item_cmd.json)?;

            // 2. Create via WriteService
            let write_service = create_write_service(ctx);
            let created = write_service.create_cipher(cipher_view, session).await
                .map_err(|e| Response::error(e.to_string()))?;

            // 3. Return decrypted view
            let vault_service = create_vault_service(ctx);
            match vault_service.get_item(&created.id, session).await {
                Ok(decrypted) => Ok(Response::success(decrypted)),
                Err(e) => Ok(Response::error(e.to_string())),
            }
        }

        CreateCommands::Folder(folder_cmd) => {
            let session = get_session(global_args)?;

            // 1. Parse folder input
            let folder_input = parse_folder_input(&folder_cmd.json)?;

            // 2. Create via WriteService
            let write_service = create_write_service(ctx);
            let created = write_service.create_folder(folder_input.name, session).await
                .map_err(|e| Response::error(e.to_string()))?;

            // 3. Return decrypted view (folder names need decryption)
            let vault_service = create_vault_service(ctx);
            let folders = vault_service.list_folders(None, session).await?;
            let decrypted = folders.iter()
                .find(|f| f.id == created.id)
                .cloned()
                .ok_or(VaultError::FolderNotFound)?;

            Ok(Response::success(decrypted))
        }

        CreateCommands::Attachment(_) | CreateCommands::OrgCollection(_) => {
            Ok(Response::error("Not yet implemented"))
        }
    }
}
```

#### 4.2 Edit Item (`bw edit item <id> <json>`)

```rust
pub async fn execute_edit(
    cmd: EditCommands,
    global_args: &GlobalArgs,
    ctx: &AppContext,
) -> anyhow::Result<Response> {
    match cmd {
        EditCommands::Item(item_cmd) => {
            let session = get_session(global_args)?;

            // 1. Get existing item
            let vault_service = create_vault_service(ctx);
            let existing = vault_service.get_item(&item_cmd.id, session).await
                .map_err(|e| Response::error(format!("Item not found: {}", e)))?;

            // 2. Check not deleted
            if existing.deleted_date.is_some() {
                return Ok(Response::error("Cannot edit items in trash. Restore first."));
            }

            // 3. Parse input and merge
            let updates = parse_item_input(&item_cmd.json)?;
            let merged = merge_cipher_views(existing, updates);

            // 4. Update via WriteService
            let write_service = create_write_service(ctx);
            let updated = write_service.update_cipher(&item_cmd.id, merged, session).await
                .map_err(|e| Response::error(e.to_string()))?;

            // 5. Return decrypted view
            match vault_service.get_item(&updated.id, session).await {
                Ok(decrypted) => Ok(Response::success(decrypted)),
                Err(e) => Ok(Response::error(e.to_string())),
            }
        }

        EditCommands::Folder(folder_cmd) => {
            let session = get_session(global_args)?;

            // 1. Parse folder input
            let folder_input = parse_folder_input(&folder_cmd.json)?;

            // 2. Update via WriteService
            let write_service = create_write_service(ctx);
            write_service.update_folder(&folder_cmd.id, folder_input.name, session).await
                .map_err(|e| Response::error(e.to_string()))?;

            // 3. Return decrypted view
            let vault_service = create_vault_service(ctx);
            let folders = vault_service.list_folders(None, session).await?;
            let decrypted = folders.iter()
                .find(|f| f.id == folder_cmd.id)
                .cloned()
                .ok_or(VaultError::FolderNotFound)?;

            Ok(Response::success(decrypted))
        }

        _ => Ok(Response::error("Not yet implemented")),
    }
}
```

**Merge Strategy for Edit:**
```rust
/// Merge updates into existing cipher view
///
/// Strategy: Update fields that are present in updates,
/// preserve fields that are not specified (null/missing).
fn merge_cipher_views(existing: CipherView, updates: CipherView) -> CipherView {
    CipherView {
        // ID must match existing
        id: existing.id,

        // These can be updated
        organization_id: updates.organization_id.or(existing.organization_id),
        folder_id: if updates.folder_id.is_some() { updates.folder_id } else { existing.folder_id },
        cipher_type: updates.cipher_type,  // Type can change
        name: if updates.name.is_empty() { existing.name } else { updates.name },
        notes: updates.notes.or(existing.notes),
        favorite: updates.favorite,
        collection_ids: if updates.collection_ids.is_empty() {
            existing.collection_ids
        } else {
            updates.collection_ids
        },

        // Preserve metadata
        revision_date: existing.revision_date,  // WriteService updates this
        creation_date: existing.creation_date,
        deleted_date: existing.deleted_date,

        // Type-specific data - take updates if provided
        login: updates.login.or(existing.login),
        secure_note: updates.secure_note.or(existing.secure_note),
        card: updates.card.or(existing.card),
        identity: updates.identity.or(existing.identity),

        attachments: existing.attachments,  // Preserve - separate management
        fields: if updates.fields.is_empty() { existing.fields } else { updates.fields },
    }
}
```

#### 4.3 Delete Item (`bw delete item <id> [--permanent]`)

```rust
pub async fn execute_delete(
    cmd: DeleteCommands,
    global_args: &GlobalArgs,
    ctx: &AppContext,
) -> anyhow::Result<Response> {
    match cmd {
        DeleteCommands::Item(item_cmd) => {
            let write_service = create_write_service(ctx);

            match write_service.delete_cipher(
                &item_cmd.id,
                item_cmd.permanent,
                global_args.nointeraction,
            ).await {
                Ok(_) => {
                    let msg = if item_cmd.permanent {
                        "Item permanently deleted"
                    } else {
                        "Item moved to trash"
                    };
                    Ok(Response::success_message(msg))
                }
                Err(VaultError::OperationCancelled) => {
                    Ok(Response::success_message("Deletion cancelled"))
                }
                Err(e) => Ok(Response::error(e.to_string())),
            }
        }

        DeleteCommands::Folder(folder_cmd) => {
            let write_service = create_write_service(ctx);

            match write_service.delete_folder(&folder_cmd.id).await {
                Ok(_) => Ok(Response::success_message("Folder deleted")),
                Err(e) => Ok(Response::error(e.to_string())),
            }
        }

        _ => Ok(Response::error("Not yet implemented")),
    }
}
```

#### 4.4 Restore Item (`bw restore item <id>`)

```rust
pub async fn execute_restore(
    cmd: RestoreCommand,
    global_args: &GlobalArgs,
    ctx: &AppContext,
) -> anyhow::Result<Response> {
    let session = get_session(global_args)?;
    let write_service = create_write_service(ctx);

    match write_service.restore_cipher(&cmd.id).await {
        Ok(restored) => {
            // Return decrypted view
            let vault_service = create_vault_service(ctx);
            match vault_service.get_item(&restored.id, session).await {
                Ok(decrypted) => Ok(Response::success(decrypted)),
                Err(e) => Ok(Response::error(e.to_string())),
            }
        }
        Err(VaultError::ItemNotDeleted) => {
            Ok(Response::error("Item is not in trash"))
        }
        Err(e) => Ok(Response::error(e.to_string())),
    }
}
```

#### 4.5 Move Item (`bw move <itemId> <folderId>`)

```rust
pub async fn execute_move(
    cmd: MoveCommand,
    global_args: &GlobalArgs,
    ctx: &AppContext,
) -> anyhow::Result<Response> {
    let session = get_session(global_args)?;
    let write_service = create_write_service(ctx);

    // Handle "null" string to remove from folder
    let folder_id = if cmd.folder_id == "null" {
        None
    } else {
        Some(cmd.folder_id.as_str())
    };

    match write_service.move_cipher(&cmd.item_id, folder_id, session).await {
        Ok(moved) => {
            // Return decrypted view
            let vault_service = create_vault_service(ctx);
            match vault_service.get_item(&moved.id, session).await {
                Ok(decrypted) => Ok(Response::success(decrypted)),
                Err(e) => Ok(Response::error(e.to_string())),
            }
        }
        Err(e) => Ok(Response::error(e.to_string())),
    }
}
```

#### 4.6 Get Template (`bw get template <type>`)

```rust
// Add to execute_get match arm
GetCommands::Template(template_cmd) => {
    match get_item_template(&template_cmd.template_type) {
        Ok(template) => {
            if global_args.raw {
                // Raw output: just the JSON
                Ok(Response::success_raw(serde_json::to_string_pretty(&template)?))
            } else {
                Ok(Response::success(template))
            }
        }
        Err(e) => Ok(Response::error(e.to_string())),
    }
}
```

## File Organization

### New Files

```
crates/bw-cli/src/commands/
├── vault.rs          # Existing - update execute_* functions
├── input.rs          # NEW - Input parsing module
└── templates.rs      # NEW - Template generation module
```

### Modified Files

| File | Changes |
|------|---------|
| `bw-cli/src/commands/vault.rs` | Implement execute_create, execute_edit, execute_delete, execute_restore, execute_move, update execute_get for templates |
| `bw-cli/src/commands/mod.rs` | Add `pub mod input;` and `pub mod templates;` |

## Error Handling Strategy

### Error Flow

```
User Input → InputError (parsing) → CliError::InvalidInput → Response::error
           → VaultError (business logic) → Response::error
           → anyhow::Error (infrastructure) → propagate as Err
```

### Error Message Quality

All errors should be:
1. **Actionable** - Tell user what to do
2. **Specific** - Identify the exact problem
3. **Contextual** - Include relevant IDs/values

Examples:
- `"Invalid JSON: expected '}' at line 5, column 10"`
- `"Item not found: abc123-def456"`
- `"Cannot edit items in trash. Use 'bw restore abc123' first."`
- `"Invalid folder ID format: 'not-a-uuid'. Expected UUID format."`

## Testing Strategy

### Unit Tests (bw-cli)

**Input Parsing Tests:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_raw_json() {
        let input = r#"{"type":1,"name":"Test"}"#;
        let result = parse_item_input(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_base64_json() {
        let json = r#"{"type":1,"name":"Test"}"#;
        let encoded = base64::engine::general_purpose::STANDARD.encode(json);
        let result = parse_item_input(&encoded);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_invalid_json() {
        let result = parse_item_input("{invalid}");
        assert!(matches!(result, Err(InputError::JsonParseError(_))));
    }

    #[test]
    fn test_parse_invalid_base64() {
        let result = parse_item_input("not-valid-base64-or-json");
        assert!(matches!(result, Err(InputError::Base64DecodeError(_))));
    }
}
```

**Template Tests:**
```rust
#[test]
fn test_login_template_structure() {
    let template = get_item_template("item.login").unwrap();
    assert_eq!(template["type"], 1);
    assert!(template["login"].is_object());
}

#[test]
fn test_folder_template_structure() {
    let template = get_item_template("folder").unwrap();
    assert!(template["name"].is_string());
}

#[test]
fn test_unknown_template() {
    let result = get_item_template("unknown");
    assert!(result.is_err());
}
```

### Integration Tests

**End-to-end flow tests** (require test server or mocking):

```rust
#[tokio::test]
async fn test_create_login_item() {
    // Setup mock API server
    // Create item via CLI
    // Verify item exists in vault
}

#[tokio::test]
async fn test_edit_preserves_unmodified_fields() {
    // Create item with all fields
    // Edit only the name
    // Verify other fields unchanged
}

#[tokio::test]
async fn test_delete_to_trash_then_restore() {
    // Create item
    // Soft delete
    // Verify in trash
    // Restore
    // Verify not in trash
}

#[tokio::test]
async fn test_move_to_folder_then_remove() {
    // Create item and folder
    // Move item to folder
    // Move item to null (no folder)
}
```

## Security Considerations

### Input Validation

1. **JSON Size Limits** - Prevent DoS via large payloads
   ```rust
   const MAX_INPUT_SIZE: usize = 1_000_000; // 1MB
   ```

2. **No Logging of Sensitive Fields** - Never log:
   - Passwords
   - TOTP secrets
   - Card numbers
   - SSN
   - Session keys

3. **Input Sanitization** - ValidationService already handles:
   - Field length limits
   - UUID format validation
   - TOTP URI format validation

### Session Handling

- Session key is required for all write operations
- Session is never stored, only passed through
- Uses existing `get_session()` helper that checks `--session` or `BW_SESSION`

## Performance Considerations

### Efficient Operations

1. **Single API Call Per Operation** - WriteService handles this
2. **Atomic Cache Updates** - WriteService handles this
3. **Lazy Decryption** - Only decrypt what's needed for output

### Memory Efficiency

1. **Streaming stdin** - Use buffered reader, not loading entire input
2. **Avoid Cloning** - Use references where possible
3. **Drop Sensitive Data** - Let Rust's ownership handle cleanup

## Implementation Phases

### Phase 1: Input Parsing & Templates (Foundation)

**Tasks:**
1. Create `bw-cli/src/commands/input.rs`
   - `parse_item_input()` function
   - `parse_folder_input()` function
   - `InputError` enum
   - Stdin reading support

2. Create `bw-cli/src/commands/templates.rs`
   - `get_item_template()` function
   - All template definitions
   - `TemplateError` enum

3. Update `bw-cli/src/commands/mod.rs`
   - Export new modules

4. Implement `bw get template <type>` in `execute_get()`

**Deliverables:**
- Working template command
- Tested input parsing utilities

### Phase 2: Create & Edit Commands

**Tasks:**
1. Add `create_write_service()` helper to `vault.rs`

2. Implement `execute_create()`:
   - Item creation
   - Folder creation
   - Return decrypted response

3. Implement `merge_cipher_views()` function

4. Implement `execute_edit()`:
   - Item editing with merge
   - Folder editing
   - Trash check

**Deliverables:**
- Working `bw create item/folder` commands
- Working `bw edit item/folder` commands

### Phase 3: Delete, Restore, Move Commands

**Tasks:**
1. Implement `execute_delete()`:
   - Soft delete
   - Permanent delete with confirmation
   - Folder deletion

2. Implement `execute_restore()`:
   - Restore from trash
   - Proper error for non-trashed items

3. Implement `execute_move()`:
   - Move to folder
   - Move to null (no folder)

**Deliverables:**
- Working `bw delete item/folder` commands
- Working `bw restore item` command
- Working `bw move` command

### Phase 4: Testing & Documentation

**Tasks:**
1. Write unit tests for input parsing
2. Write unit tests for templates
3. Write integration tests for command flows
4. Verify TypeScript CLI compatibility
5. Update CLI help text as needed

**Deliverables:**
- Comprehensive test coverage
- All commands working end-to-end

## Dependencies

### Existing Dependencies (No Changes)

| Crate | Usage |
|-------|-------|
| `base64` | Already in bw-cli for encode command |
| `serde_json` | JSON parsing |
| `thiserror` | Error types |
| `clap` | CLI argument parsing |

### No New Dependencies Required

All functionality can be implemented with existing dependencies.

## Open Question Resolution

### OQ-1: `bw move <id> null` Handling

**Decision:** Accept literal string "null" for TypeScript CLI compatibility.

**Implementation:**
```rust
let folder_id = if cmd.folder_id == "null" {
    None
} else {
    Some(cmd.folder_id.as_str())
};
```

### OQ-2: `--quiet` Flag

**Decision:** Defer to Phase 2 (post-MVP). Current implementation always returns JSON response which is suitable for scripting.

## Success Criteria

- [ ] All MVP commands implemented and working
- [ ] Input/output JSON format matches TypeScript CLI exactly
- [ ] Round-trip create/edit between Rust and TypeScript CLIs works
- [ ] Unit tests pass for parsing and templates
- [ ] Integration tests pass for command flows
- [ ] Clear, actionable error messages for all failure modes
- [ ] `cargo fmt --check` passes
- [ ] `cargo clippy --all-features --all-targets` passes
- [ ] `cargo build --release` succeeds
- [ ] `cargo test` passes

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| JSON format mismatch with TypeScript CLI | Use static templates, test round-trip interop |
| Merge logic bugs in edit | Comprehensive unit tests for merge function |
| Stdin handling edge cases | Test with empty input, binary data, large input |
| API errors not propagated clearly | Map VaultError to user-friendly messages |
