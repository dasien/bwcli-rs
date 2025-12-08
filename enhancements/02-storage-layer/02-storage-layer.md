---
slug: storage-layer
status: NEW
created: 2024-12-02
author: Migration Team
priority: critical
---

# Enhancement: CLI Rust Migration - Storage Layer

## Overview
**Goal:** Implement persistent storage and application state management for the Rust CLI, matching the behavior of the TypeScript Low DB-based storage.

**User Story:**
As a CLI user, I want my configuration and session state to persist between invocations so that I don't have to repeatedly provide authentication and settings.

## Context & Background
**Current State:**
- TypeScript CLI uses LowdbStorageService for JSON file storage
- NodeEnvSecureStorageService encrypts sensitive values using BW_SESSION
- State includes: tokens, user info, environment URLs, sync data, encrypted keys
- Storage at platform-appropriate paths
- This is enhancement 2 of 8, depends on enhancement 1

**Technical Context:**
- Rust project at `bwcli-rs/`
- Need cross-platform path resolution  
- JSON file-based storage matching lowdb behavior
- Session-key encrypted storage for sensitive data
- Must read existing TypeScript CLI state if present

**Dependencies:**
- Enhancement: project-bootstrap (must be complete)
- directories crate for platform paths
- serde, serde_json for serialization
- File system I/O with atomic writes

## Requirements

### Functional Requirements
1. Determine storage directory based on platform and environment
2. JSON file-based storage (lowdb compatibility)
3. Secure storage with BW_SESSION encryption
4. Get, set, remove, has operations
5. Handle __PROTECTED__ prefix for encrypted values
6. Atomic file writes to prevent corruption
7. State structures for: env URLs, tokens, user info, sync state, KDF config

### Non-Functional Requirements
- **Performance:** Storage operations <50ms typical
- **Memory:** Minimal overhead, efficient JSON handling
- **Reliability:** Atomic writes, handle corruption gracefully, no data loss
- **Compatibility:** Read existing TypeScript CLI storage

### Must Have (MVP)
- [ ] Platform-specific path resolution
- [ ] Support BITWARDENCLI_APPDATA_DIR override
- [ ] Support relative ./bw-data directory check
- [ ] JSON storage trait with get/set/remove/has
- [ ] Secure storage with BW_SESSION encryption
- [ ] State structures defined
- [ ] Atomic file writes
- [ ] Error handling for missing/corrupted files
- [ ] Migration from TypeScript storage format

### Should Have (if time permits)
- [ ] Storage backup before writes
- [ ] Storage versioning
- [ ] Multiple profile support
- [ ] Storage compaction/cleanup

### Won't Have (out of scope)
- Database storage (reason: maintaining JSON compatibility)
- Network sync of storage (reason: not in original)
- Encryption beyond BW_SESSION (reason: separate concern)

## Open Questions

1. Should we use sled or stick with JSON files like TypeScript?
2. How to handle concurrent access to storage files?
3. Should we add storage versioning from the start?
4. What happens if BW_SESSION changes mid-operation?
5. How to handle storage migration from old formats?
6. Should we validate storage integrity on load?

## Constraints & Limitations
**Technical Constraints:**
- Must maintain compatibility with TypeScript CLI storage
- Storage must be human-readable (JSON)
- Must work without elevated permissions
- Must handle platform path differences
- Cannot lose user data during writes

**Business/Timeline Constraints:**
- Blocking enhancement 3 (API client needs config storage)
- Blocking enhancement 4 (authentication needs token storage)
- Critical path item

## Success Criteria
**Definition of Done:**
- [ ] Storage persists across CLI invocations
- [ ] Secure storage encrypts sensitive values correctly
- [ ] Correct platform-specific paths used
- [ ] BITWARDENCLI_APPDATA_DIR override works
- [ ] Relative ./bw-data directory detection works
- [ ] Atomic writes prevent corruption
- [ ] Can read existing TypeScript CLI storage
- [ ] Unit tests for all operations pass
- [ ] Integration tests with file system pass

**Acceptance Tests:**
1. Given no existing storage, when writing data, then creates directory and file
2. Given existing TypeScript storage, when reading, then loads correctly
3. Given encrypted value, when reading without BW_SESSION, then returns encrypted string
4. Given encrypted value, when reading with BW_SESSION, then decrypts correctly
5. Given concurrent writes, when saving, then no corruption occurs
6. Given corrupted storage file, when reading, then handles gracefully
7. Given BITWARDENCLI_APPDATA_DIR set, when determining path, then uses env var
8. Given ./bw-data exists, when determining path, then uses relative directory

## Security & Safety Considerations
- Encrypt sensitive data with BW_SESSION
- File permissions: user-readable only where possible
- Validate all file paths to prevent traversal
- Don't log sensitive storage values
- Clear sensitive data from memory after use
- Handle symlinks safely
- Atomic writes to prevent partial updates

## UI/UX Considerations (if applicable)
- Clear error messages for storage issues
- Helpful suggestions for fixing storage problems
- Document storage location in help/documentation
- Warn before overwriting storage

## Testing Strategy
**Unit Tests:**
- Test JSON serialization/deserialization
- Test secure storage encryption/decryption
- Test path resolution for each platform
- Test atomic write logic
- Test storage operations: get, set, remove, has
- Test error handling

**Integration Tests:**
- Test actual file I/O operations
- Test reading TypeScript CLI storage
- Test concurrent access scenarios
- Test permission errors
- Test corrupted file handling

**Manual Test Scenarios:**
1. Delete storage, run CLI, verify creation
2. Manually corrupt storage, verify error handling
3. Test on Windows, macOS, Linux
4. Set BITWARDENCLI_APPDATA_DIR, verify path
5. Create ./bw-data directory, verify usage

## References & Research
- apps/cli/src/platform/services/lowdb-storage.service.ts
- apps/cli/src/platform/services/node-env-secure-storage.service.ts
- apps/cli/src/service-container/service-container.ts
- XDG Base Directory Specification (Linux)
- Windows Known Folders
- macOS Standard Directories
- directories crate documentation

## Notes for PM Subagent
- Verify exact storage format requirements
- Confirm backward compatibility requirements
- Flag if TypeScript storage format is unclear
- Ensure migration path is documented

## Notes for Architect Subagent
- Design storage trait for testability (mock implementations)
- Separate JSON storage from secure storage concerns
- Plan for storage versioning and migration
- Consider trait-based storage sources
- Design state structures with forward compatibility
- Plan for concurrent access if needed

## Notes for Implementer Subagent
- Use directories crate for platform paths
- Implement atomic writes using temp file + rename
- Use serde(default) for missing fields
- Match TypeScript field names with #[serde(rename)]
- Add extensive error context
- Use tempfile crate for atomic operations
- Implement Debug trait carefully (sanitize sensitive data)
- Consider storage file locking

## Notes for Testing Subagent
- Test on all target platforms
- Test with various permission scenarios
- Mock filesystem for unit tests
- Test with Unicode paths
- Test with very long paths
- Verify atomic write behavior
- Test TypeScript storage compatibility thoroughly
- Test encryption/decryption with various keys