# Code Changes Detail

## Summary of Code Changes

This document provides details on the code changes made to implement vault create/edit commands.

## New Files

### `crates/bw-cli/src/commands/input.rs`

Purpose: Input parsing for CLI commands that accept JSON data

Key types:
- `InputError` - Error enum for parsing failures
- `FolderInput` - Simple struct for folder create/edit input

Key functions:
- `parse_item_input(input: &str) -> Result<CipherView, InputError>` - Parse item JSON
- `parse_folder_input(input: &str) -> Result<FolderInput, InputError>` - Parse folder JSON

Implementation notes:
- Detects input format automatically (base64 vs raw JSON vs stdin)
- Uses `base64::engine::general_purpose::STANDARD` for encoding
- Limits input size to 1MB to prevent DoS attacks

### `crates/bw-cli/src/commands/templates.rs`

Purpose: Generate JSON templates for item creation

Key types:
- `TemplateError` - Error enum for unknown template types

Key functions:
- `get_item_template(template_type: &str) -> Result<Value, TemplateError>` - Get template by type

Template functions (private):
- `login_template()` - Login item (type=1)
- `secure_note_template()` - Secure note (type=2)
- `card_template()` - Card/payment (type=3)
- `identity_template()` - Identity (type=4)
- `folder_template()` - Folder
- `field_template()` - Custom field
- `uri_template()` - Login URI

## Modified Files

### `crates/bw-cli/src/commands/mod.rs`

Changes:
- Added `pub mod input;`
- Added `pub mod templates;`
- Added `pub use input::*;`
- Added `pub use templates::*;`

### `crates/bw-cli/src/commands/vault.rs`

New imports:
```rust
use crate::commands::input::{parse_folder_input, parse_item_input};
use crate::commands::templates::get_item_template;
use bw_core::models::vault::CipherView;
use bw_core::services::vault::{
    CipherService, ConfirmationService, ValidationService, VaultError, WriteService,
};
```

New helper functions:

1. `create_write_service(ctx: &AppContext, no_interaction: bool) -> WriteService`
   - Creates WriteService with all required dependencies
   - Passes `no_interaction` flag for confirmation dialogs

2. `merge_cipher_views(existing: CipherView, updates: CipherView) -> CipherView`
   - Merges update data into existing cipher view
   - Preserves ID, creation_date, deleted_date, attachments
   - Updates only fields that are specified in updates

Implemented command handlers:

1. `execute_create` (~65 lines)
   - Handles `CreateCommands::Item` and `CreateCommands::Folder`
   - Returns `Response::error("Not yet implemented")` for Attachment and OrgCollection

2. `execute_edit` (~85 lines)
   - Handles `EditCommands::Item` and `EditCommands::Folder`
   - Validates item is not in trash before editing
   - Uses `merge_cipher_views()` for partial updates

3. `execute_delete` (~45 lines)
   - Handles `DeleteCommands::Item` and `DeleteCommands::Folder`
   - Supports `--permanent` flag for items

4. `execute_restore` (~25 lines)
   - Restores items from trash
   - Validates item is actually deleted

5. `execute_move` (~35 lines)
   - Moves item to folder
   - Handles "null" string to remove from folder

6. Updated `execute_get` (~45 lines added)
   - Added `GetCommands::Template` handling
   - Added `GetCommands::Folder` handling

## Code Patterns

### Error Handling Pattern

All commands follow this pattern:
```rust
match service_call().await {
    Ok(result) => {
        // Success path - return decrypted view
        Ok(Response::success(result))
    }
    Err(VaultError::SpecificError) => {
        Ok(Response::error("User-friendly message"))
    }
    Err(e) => Ok(Response::error(e.to_string())),
}
```

### Service Creation Pattern

```rust
let session = get_session(global_args)?;
let write_service = create_write_service(ctx, global_args.nointeraction);
let vault_service = create_vault_service(ctx);
```

### Input Parsing Pattern

```rust
let cipher_view = match parse_item_input(&item_cmd.json) {
    Ok(view) => view,
    Err(e) => return Ok(Response::error(format!("Invalid input: {}", e))),
};
```

## Lines of Code

| File | Lines Added | Lines Modified |
|------|-------------|----------------|
| `input.rs` | ~130 | 0 |
| `templates.rs` | ~180 | 0 |
| `mod.rs` | 4 | 0 |
| `vault.rs` | ~280 | ~10 |
| **Total** | ~594 | ~10 |
