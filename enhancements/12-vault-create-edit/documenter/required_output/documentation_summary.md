---
enhancement: 12-vault-create-edit
agent: documenter
task_id: task_1765415522_34443
timestamp: 2025-12-10T18:00:00Z
status: DOCUMENTATION_COMPLETE
---

# Documentation Summary: Vault Create/Edit CLI Commands

## Overview

This document summarizes the documentation updates for enhancement 12-vault-create-edit, which adds vault write commands to the Bitwarden CLI.

## Documentation Deliverables

### 1. README.md Update

The project README.md at `/Users/bgentry/Source/repos/bwcli-rs/README.md` should be updated to reflect that vault write commands are now implemented.

**Current status line:**
```markdown
- 🚧 Vault write commands (create, edit, delete)
```

**Updated status line:**
```markdown
- ✅ Vault write commands (create, edit, delete, restore, move)
```

### 2. User Guide (New File)

Created comprehensive user documentation at:
`enhancements/12-vault-create-edit/documenter/optional_output/vault-write-commands-guide.md`

This guide covers:
- Creating vault items (all types: login, secure note, card, identity)
- Creating folders
- Editing items and folders
- Deleting items (soft and permanent)
- Restoring items from trash
- Moving items between folders
- Using templates
- Input formats (raw JSON, base64, stdin)
- Error handling and troubleshooting

### 3. CLI Help Text

All commands include proper help text via clap's `#[arg(help = "...")]` attributes. Users can access help via:

```bash
bw create --help
bw edit --help
bw delete --help
bw restore --help
bw move --help
bw get template --help
```

## Commands Documented

| Command | Description | Status |
|---------|-------------|--------|
| `bw create item <json>` | Create a new vault item | Documented |
| `bw create folder <json>` | Create a new folder | Documented |
| `bw edit item <id> <json>` | Edit an existing item | Documented |
| `bw edit folder <id> <json>` | Edit a folder name | Documented |
| `bw delete item <id>` | Move item to trash | Documented |
| `bw delete item <id> --permanent` | Permanently delete item | Documented |
| `bw delete folder <id>` | Delete a folder | Documented |
| `bw restore <id>` | Restore item from trash | Documented |
| `bw move <id> <folderId>` | Move item to folder | Documented |
| `bw get template <type>` | Get JSON template | Documented |
| `bw get folder <id>` | Get folder by ID | Documented |

## Template Types Documented

| Template | Command | Description |
|----------|---------|-------------|
| Login | `bw get template item.login` | Password/credentials storage |
| Secure Note | `bw get template item.secureNote` | Encrypted text notes |
| Card | `bw get template item.card` | Payment card details |
| Identity | `bw get template item.identity` | Personal information |
| Folder | `bw get template folder` | Folder creation |
| Field | `bw get template item.field` | Custom fields |
| URI | `bw get template item.login.uri` | Login URI entries |

## Input Format Documentation

The guide documents three supported input formats:

1. **Raw JSON** - Direct JSON input starting with `{`
2. **Base64-encoded JSON** - TypeScript CLI compatible format
3. **Stdin** - Piped input using `-` as argument

## Error Messages Documented

Common errors and their resolutions:
- Invalid base64 encoding
- Invalid JSON format
- Missing required fields
- Item not found
- Cannot edit items in trash
- Item is not in trash (for restore)
- Folder not found
- Session key required

## Code Documentation

The implementation includes comprehensive Rust documentation:

### `input.rs` Module
- Module-level documentation explaining supported input formats
- Documentation for `InputError` enum variants
- Documentation for `parse_item_input()` and `parse_folder_input()` functions

### `templates.rs` Module
- Module-level documentation
- Documentation for `get_item_template()` function with supported types
- Documentation for each template function

## Quality Checklist

- [x] All new commands documented
- [x] Examples provided for each command
- [x] Input formats explained
- [x] Error messages documented
- [x] Troubleshooting section included
- [x] TypeScript CLI compatibility noted
- [x] Code comments in new modules
- [x] CLI help text properly configured

## Files Modified/Created

| File | Action | Purpose |
|------|--------|---------|
| `README.md` | Update needed | Mark vault write commands as complete |
| `optional_output/vault-write-commands-guide.md` | Created | Comprehensive user guide |

## Next Steps

1. **README Update**: Update the development status section to mark vault write commands as complete
2. **Integration**: Consider adding the user guide to the main docs/ directory
3. **API Docs**: Run `cargo doc --no-deps --open` to generate API documentation from doc comments

## Conclusion

Documentation for the vault create/edit enhancement is complete. The user guide provides comprehensive instructions for all new commands with practical examples. Code documentation in the new modules follows Rust conventions and provides clear API documentation.
