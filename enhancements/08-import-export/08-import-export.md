---
slug: import-export
status: NEW
created: 2024-12-02
author: Migration Team
priority: medium
---

# Enhancement: CLI Rust Migration - Import/Export Commands

## Overview
**Goal:** Implement export and import commands to migrate data to/from Bitwarden and other password managers.

**User Story:**
As a CLI user, I want to export my vault for backup and import data from other password managers so that I can migrate my data and maintain backups.

## Context & Background
**Current State:**
- TypeScript CLI implements export (csv, json, encrypted_json) and import with format detection
- Export supports vault-wide and organization-specific exports
- Import supports 50+ password manager formats
- Import handles format detection and data transformation
- Export can be password-protected (encrypted_json)
- This is enhancement 8 of 8, depends on enhancements 1-6

**Technical Context:**
- Rust project at `bwcli-rs/`
- Export requires decrypting all vault items
- Import requires parsing various formats and encrypting
- Large vaults require efficient processing
- Some formats require complex transformation logic

**Dependencies:**
- Enhancement: project-bootstrap (must be complete)
- Enhancement: storage-layer (for accessing vault data)
- Enhancement: api-client (for bulk import)
- Enhancement: authentication-commands (for session)
- Enhancement: vault-read-operations (for export)
- Enhancement: vault-write-operations (for import)
- Bitwarden SDK for encryption/decryption

## Requirements

### Functional Requirements
1. Export command with formats: csv, json, encrypted_json
2. Export with --organizationid for org exports
3. Export with --password for encrypted_json
4. Export with --output to specify file path
5. Import command with automatic format detection
6. Import with --formats flag to list supported formats
7. Import with --format to specify format explicitly
8. Import major formats: bitwarden (csv/json), lastpass, 1password, keepass, dashlane
9. Parse and transform various input formats
10. Validate import data before processing
11. Bulk item creation during import
12. Progress indication for large imports

### Non-Functional Requirements
- **Performance:** Export <10s for 1000 items, import <30s for 1000 items
- **Memory:** Stream processing for large exports/imports
- **Reliability:** Validate data, handle errors, don't lose data
- **Compatibility:** Export format matches TypeScript CLI exactly

### Must Have (MVP)
- [ ] `bw export` with csv format
- [ ] `bw export --format json`
- [ ] `bw export --format encrypted_json --password <pwd>`
- [ ] `bw export --output <file>`
- [ ] `bw import bitwarden-csv <file>`
- [ ] `bw import bitwarden-json <file>`
- [ ] `bw import --formats` to list formats
- [ ] Format detection for bitwarden exports
- [ ] Data validation before import
- [ ] Progress indication

### Should Have (if time permits)
- [ ] Import lastpass format
- [ ] Import 1password format
- [ ] Import keepass format
- [ ] Import dashlane format
- [ ] Import chrome format
- [ ] Import other major formats (10-20 total)
- [ ] Organization-specific export
- [ ] Partial import (skip errors)
- [ ] Import dry-run mode

### Won't Have (out of scope)
- Real-time import sync (reason: CLI is batch-oriented)
- Import deduplication (reason: complex logic)
- Automatic format detection for all formats (reason: too complex)

## Open Questions

1. How many import formats are essential for MVP?
2. Should we implement all 50+ formats or focus on top 10?
3. How to handle import errors (fail-fast or continue)?
4. Should we support incremental imports?
5. How to handle duplicate items during import?
6. Should export include attachments?

## Constraints & Limitations
**Technical Constraints:**
- Export decrypts entire vault in memory (or stream)
- Import encrypts items before upload
- Some formats have lossy conversions
- Must validate import data structure
- Large imports may hit API rate limits

**Business/Timeline Constraints:**
- Final enhancement in migration
- Nice to have for feature parity
- Can be implemented incrementally

## Success Criteria
**Definition of Done:**
- [ ] `bw export` exports vault in all supported formats
- [ ] `bw export --format encrypted_json` works with password
- [ ] `bw import` imports bitwarden formats
- [ ] `bw import --formats` lists supported formats
- [ ] Import validates data before processing
- [ ] Progress indication for large operations
- [ ] Export format matches TypeScript CLI
- [ ] All tests pass
- [ ] Documentation complete

**Acceptance Tests:**
1. Given authenticated session, when running `bw export`, then CSV file created
2. Given --format json, when exporting, then JSON file created
3. Given --format encrypted_json --password, when exporting, then encrypted file created
4. Given bitwarden CSV, when importing, then all items created
5. Given bitwarden JSON, when importing, then all items created
6. Given encrypted export, when importing without password, then error returned
7. Given invalid format, when importing, then clear error message
8. Given --formats flag, when running import, then all formats listed
9. Given large vault, when exporting, then progress shown
10. Given large import, when importing, then progress shown

## Security & Safety Considerations
- Don't log exported data
- Warn about unencrypted exports
- Clear decrypted data from memory
- Validate file paths for output
- Prevent overwriting existing files without confirmation
- Encrypt import data before uploading
- Validate import data structure
- Handle sensitive fields properly

## UI/UX Considerations (if applicable)
- Warn about unencrypted exports
- Show progress for large operations
- Confirm before overwriting files
- Clear success/failure messages
- Show count of imported items
- Display import errors clearly
- Support --quiet for scripting

## Testing Strategy
**Unit Tests:**
- Test CSV generation
- Test JSON generation
- Test encrypted export
- Test format parsers
- Test data transformation
- Test validation logic

**Integration Tests:**
- Test export with test vault
- Test import with sample files
- Test round-trip (export then import)
- Test with various vault sizes
- Test encrypted export/import
- Test error handling

**Manual Test Scenarios:**
1. Export vault in all formats
2. Import own exports (round-trip test)
3. Import sample files from other managers
4. Test with large vault (1000+ items)
5. Test encrypted export with password
6. Test import error handling
7. Compare export format with TypeScript CLI
8. Test progress indication

## References & Research
- apps/cli/src/tools/commands/export.command.ts
- apps/cli/src/tools/commands/import.command.ts
- apps/cli/src/tools/import/importers/*.importer.ts (50+ formats)
- CSV/JSON formatting standards
- Bitwarden export format specification
- Other password manager export formats

## Notes for PM Subagent
- Prioritize bitwarden formats for MVP
- Determine which import formats are essential
- Flag if 50+ formats are too ambitious
- Verify encrypted export requirements

## Notes for Architect Subagent
- Design for streaming large exports
- Separate parsing from transformation
- Use strategy pattern for import formats
- Plan for format extensibility
- Consider memory usage for large vaults
- Design progress reporting system
- Plan validation pipeline

## Notes for Implementer Subagent
- Use csv and serde_json crates
- Implement streaming for large files
- Create importer trait for formats
- Validate data before processing
- Add progress bars with indicatif
- Follow TypeScript export format exactly
- Handle encoding properly (UTF-8)
- Add comprehensive error context
- Consider parallel processing for imports

## Notes for Testing Subagent
- Test export formats thoroughly
- Test round-trip operations
- Test with various vault sizes
- Create sample import files
- Test error scenarios
- Verify encrypted export security
- Test import validation
- Test progress reporting
- Compare output with TypeScript CLI
- Test with real-world import files