---
slug: vault-create-edit
status: NEW
created: 2024-12-10
author: Claude
priority: high
---

# Enhancement: Implement Vault Create/Edit CLI Commands

## Overview
**Goal:** Connect existing bw-core WriteService to CLI command handlers, enabling users to create, edit, delete, restore, and move vault items from the command line.

**User Story:**
As a CLI user, I want to create and modify vault items using JSON input so that I can manage my passwords programmatically and through scripts.

## Context & Background
**Current State:**
- bw-core WriteService is fully implemented with all CRUD operations
- CLI command handlers exist as stubs returning "Not yet implemented"
- CipherView and Folder models match TypeScript CLI format
- ValidationService and CipherService encryption are operational
- API endpoints module exists with proper path constants

**Technical Context:**
- Rust project at `bwcli-rs/`
- WriteService handles business logic: validation, encryption, API calls, cache updates
- TypeScript CLI uses base64-encoded JSON for input
- Must decrypt response and return to user after create/edit operations
- Session key required for encryption (--session flag or BW_SESSION env var)

**Dependencies:**
- Enhancement 01-05: Project bootstrap, storage, API, auth, vault read (complete)
- Enhancement 06: Vault write operations in bw-core (complete)
- Enhancement 11: Encrypt/decrypt commands (complete)
- Bitwarden SDK for encryption operations

## Requirements

### Functional Requirements
1. `bw create item <json>` creates encrypted vault item
2. `bw create folder <json>` creates encrypted folder
3. `bw edit item <id> <json>` updates existing item
4. `bw edit folder <id> <json>` updates folder name
5. `bw delete item <id>` soft deletes to trash
6. `bw delete item <id> --permanent` permanently deletes
7. `bw delete folder <id>` deletes folder
8. `bw restore item <id>` restores from trash
9. `bw move <id> <folderId>` moves item to folder
10. `bw get template <type>` returns JSON template for item types
11. Support base64-encoded JSON input (TypeScript CLI compatible)
12. Support reading JSON from stdin
13. Return decrypted item after create/edit operations

### Non-Functional Requirements
- **Performance:** Operations complete in <2 seconds
- **Memory:** Efficient handling of input data
- **Reliability:** Validate all input before API calls
- **Compatibility:** Input/output format matches TypeScript CLI exactly

### Must Have (MVP)
- [ ] `bw create item <json>` with all cipher types (login, note, card, identity)
- [ ] `bw create folder <json>`
- [ ] `bw edit item <id> <json>`
- [ ] `bw edit folder <id> <json>`
- [ ] `bw delete item <id>` (soft delete)
- [ ] `bw delete item <id> --permanent`
- [ ] `bw delete folder <id>`
- [ ] `bw restore item <id>`
- [ ] `bw move <id> <folderId>` (and `bw move <id> null` for no folder)
- [ ] `bw get template item.login`
- [ ] `bw get template item.secureNote`
- [ ] `bw get template item.card`
- [ ] `bw get template item.identity`
- [ ] `bw get template folder`
- [ ] Base64 JSON input parsing
- [ ] Stdin input support
- [ ] Input validation with clear error messages
- [ ] Return decrypted cipher/folder after mutations

### Should Have (if time permits)
- [ ] `bw create attachment --file <path> --itemid <id>`
- [ ] `bw delete attachment <id> --itemid <id>`
- [ ] `bw edit item-collections <id> <collection_ids>`
- [ ] `bw create org-collection <json> --organizationid <id>`
- [ ] `bw edit org-collection <id> <json> --organizationid <id>`
- [ ] `bw delete org-collection <id> --organizationid <id>`

### Won't Have (out of scope)
- Interactive prompts for item creation (reason: CLI only)
- GUI editor (reason: not CLI)
- Batch operations (reason: complexity)
- Undo functionality (reason: API doesn't support)
- `bw share` command (reason: separate enhancement)
- `bw confirm` command (reason: separate enhancement)

## Open Questions

1. ~~How should we handle merge behavior during edit?~~ **RESOLVED:** Follow TypeScript CLI - CipherExport.toView() merges new data into existing, preserving unchanged fields.

2. ~~Should we support raw JSON input or only base64?~~ **RESOLVED:** Support both for developer convenience. Detect based on whether input starts with `{`.

3. How should we handle the `bw move <id> null` case to remove from folder?
   - Option A: Literal string "null"
   - Option B: Empty string
   - Option C: Special keyword like "none"

4. Should we support `--quiet` flag to suppress output for scripting?

## Constraints & Limitations
**Technical Constraints:**
- Must use existing WriteService methods (do not duplicate logic)
- Must use existing CipherService for encryption
- Must use existing ValidationService for input validation
- Must use existing endpoint constants
- Input format must be TypeScript CLI compatible
- Session key required for all operations

**Business/Timeline Constraints:**
- This enables full vault management capability
- Required for complete CLI feature parity

## Success Criteria
**Definition of Done:**
- [ ] All MVP commands implemented and working
- [ ] Input/output format matches TypeScript CLI
- [ ] Items created by Rust CLI readable by TypeScript CLI
- [ ] Items created by TypeScript CLI editable by Rust CLI
- [ ] All existing tests pass
- [ ] New unit tests for input parsing
- [ ] New integration tests for create/edit/delete flow
- [ ] Documentation updated

**Acceptance Tests:**
1. Given valid login JSON, when running `bw create item <base64>`, then item created and decrypted item returned
2. Given valid folder JSON, when running `bw create folder <base64>`, then folder created and returned
3. Given existing item ID, when running `bw edit item <id> <base64>`, then item updated with changes
4. Given existing item ID, when running `bw delete item <id>`, then item moved to trash
5. Given trashed item ID, when running `bw restore item <id>`, then item restored
6. Given item and folder IDs, when running `bw move <itemId> <folderId>`, then item moved
7. Given invalid JSON, when creating, then clear error message returned
8. Given non-existent ID, when editing, then "not found" error returned
9. Given deleted item, when editing, then "cannot edit deleted item" error returned
10. Given `--permanent` flag, when deleting, then item permanently removed

## Security & Safety Considerations
- **Input validation**: Validate all JSON fields before encryption
- **No sensitive data in logs**: Never log decrypted item content
- **Session key handling**: Require valid session for all operations
- **Confirmation for permanent delete**: Consider `--permanent` as explicit opt-in
- **Clear memory**: Use zeroization patterns for sensitive intermediate data

## UI/UX Considerations
- Return created/edited item as JSON (same as `bw get item`)
- Use `Response::success()` / `Response::error()` for consistent output
- Support `--raw` flag for unformatted output
- Clear error messages with suggestions for common mistakes
- Template command provides starting point for users

## Testing Strategy
**Unit Tests:**
- Base64 decode + JSON parse helper
- Merge logic for edit operations
- Template generation for all types
- Validation error messages
- Edge cases: empty strings, null values, invalid types

**Integration Tests:**
- Full create → get → verify flow
- Full edit → get → verify changes flow
- Full delete → restore → verify flow
- Move to folder → verify → move to null → verify
- Create via stdin input
- Error scenarios: invalid auth, not found, validation failures

**Manual Test Scenarios:**
1. Create each item type (login, note, card, identity)
2. Edit item with partial changes (verify unchanged fields preserved)
3. Delete and restore
4. Permanent delete (verify cannot restore)
5. Move between folders
6. Create using piped stdin
7. Compare with TypeScript CLI output format
8. Test with items created by TypeScript CLI

## References & Research
- TypeScript CLI: `apps/cli/src/vault/create.command.ts`
- TypeScript CLI: `apps/cli/src/commands/edit.command.ts`
- TypeScript CLI: `apps/cli/src/vault/delete.command.ts`
- TypeScript CLI: `apps/cli/src/commands/restore.command.ts`
- TypeScript models: `libs/common/src/models/export/cipher.export.ts`
- Analysis doc: `docs/vault-create-edit-analysis.md`
- Code quality: `docs/bw-core-code-quality-issues.md`
- Existing WriteService: `crates/bw-core/src/services/vault/write_service.rs`

## Notes for PM Subagent
- Verify all template types needed for MVP
- Confirm input format requirements match TypeScript CLI
- Flag if attachment handling should be in MVP
- Verify error message requirements

## Notes for Architect Subagent
- Use existing WriteService - do NOT duplicate business logic
- Design JSON parsing/merge helpers in CLI layer only
- Consider stdin input handling approach
- Plan template generation structure
- Use existing patterns:
  - `endpoints::api::ciphers::BASE` for API paths
  - `limits::CIPHER_NAME_MAX_LEN` for validation
  - `VaultError` for error propagation
- No new services in bw-core needed

## Notes for Implementer Subagent
- Start with input parsing helpers in vault.rs
- Connect to existing WriteService methods
- Implement template generation using CipherView defaults
- Follow TypeScript CLI input format exactly:
  ```rust
  // Decode base64 → UTF-8 → JSON → CipherView
  let json_str = base64::decode(input)?;
  let utf8 = String::from_utf8(json_str)?;
  let cipher_view: CipherView = serde_json::from_str(&utf8)?;
  ```
- For stdin: Check if isatty, read until EOF
- For edit merge: Load existing, apply non-null fields from input
- Use `Response::success(decrypted_item)` for output

## Notes for Testing Subagent
- Test all cipher types (login, note, card, identity)
- Test partial edit (only some fields changed)
- Test input validation error messages
- Test base64 vs raw JSON input
- Test stdin input
- Compare output format with TypeScript CLI
- Test interoperability:
  - Create with Rust → read with TypeScript
  - Create with TypeScript → edit with Rust
- Test error cases thoroughly
- Verify cache updates after operations

## Notes for Documenter Subagent
- Document all new commands in user guide
- Document template usage workflow
- Document JSON input format with examples
- Document stdin usage
- Document error messages and troubleshooting
- Add examples for each item type
