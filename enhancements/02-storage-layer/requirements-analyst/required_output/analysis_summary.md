---
enhancement: 02-storage-layer
agent: requirements-analyst
task_id: task_1764793113_32500
timestamp: 2025-12-03T19:45:00Z
status: READY_FOR_ARCHITECTURE
---

# Requirements Analysis: Storage Layer Implementation

## Executive Summary

This analysis covers the requirements for implementing a persistent storage layer for the Bitwarden CLI Rust migration. The storage system must provide JSON-based file storage with optional encryption for sensitive values, while maintaining full backward compatibility with the existing TypeScript CLI storage format.

**Priority**: Critical (Blocking enhancements 3-8)
**Complexity**: Medium
**Risk Level**: Medium-High (data persistence and security concerns)

## What Needs to Be Built

### Core Storage Capabilities

**1. Platform-Aware Storage Path Resolution**
- Determine storage directory based on platform and environment
- Priority order: `./bw-data` (relative to executable) → `BITWARDENCLI_APPDATA_DIR` env var → Platform defaults
- Platform defaults:
  - **macOS**: `~/Library/Application Support/Bitwarden CLI`
  - **Windows**: `%APPDATA%/Bitwarden CLI`
  - **Linux/Other**: `$XDG_CONFIG_HOME/Bitwarden CLI` or `~/.config/Bitwarden CLI`

**2. JSON File Storage (LowDB Compatible)**
- Store configuration as JSON in `data.json` file within storage directory
- Support basic operations: `get`, `set`, `remove`, `has`
- All values stored as JSON-serializable types
- Maintain compatibility with TypeScript CLI's LowDB format

**3. Secure Storage with Session Encryption**
- Encrypt sensitive values using `BW_SESSION` environment variable as encryption key
- Store encrypted values with `__PROTECTED__` key prefix
- Decrypt on read when `BW_SESSION` is available
- Return encrypted value (base64) when `BW_SESSION` is not available
- Use AES encryption compatible with TypeScript implementation

**4. Atomic Write Operations**
- Prevent corruption during write operations
- Use temp file + atomic rename pattern
- Handle concurrent access scenarios safely

**5. State Data Structures**
The storage layer must support these state structures (specific schema defined by architecture phase):
- Environment URLs (API, Identity, Web Vault URLs)
- Access/Refresh tokens
- User profile information
- Vault sync state/timestamp
- KDF configuration (PBKDF2/Argon2id parameters)
- Encrypted vault keys

## User Stories

### US-1: Persistent Configuration
**As a** CLI user
**I want** my server configuration to persist between CLI invocations
**So that** I don't have to specify `--server` on every command

**Acceptance Criteria**:
- [ ] Storage directory is created on first use with appropriate permissions (0700)
- [ ] `data.json` file is created with user-only read/write permissions (0600)
- [ ] Configuration values written in one CLI invocation are readable in the next
- [ ] Invalid JSON in storage file is handled gracefully with user-friendly error

### US-2: Secure Token Storage
**As a** CLI user
**I want** my authentication tokens encrypted at rest
**So that** other processes cannot steal my credentials from disk

**Acceptance Criteria**:
- [ ] Tokens are stored with `__PROTECTED__` prefix
- [ ] Encrypted values use base64 encoding
- [ ] Tokens can only be decrypted with correct `BW_SESSION` key
- [ ] Attempting to read encrypted value without `BW_SESSION` returns the encrypted string
- [ ] `BW_SESSION` key is never written to disk

### US-3: Custom Storage Location
**As a** developer/CI user
**I want** to specify a custom storage directory
**So that** I can isolate test environments or use custom paths

**Acceptance Criteria**:
- [ ] Setting `BITWARDENCLI_APPDATA_DIR=/custom/path` uses that directory
- [ ] Relative paths are resolved correctly
- [ ] Storage works correctly in custom location
- [ ] Permission errors are reported clearly

### US-4: Portable Installation
**As a** portable app user
**I want** CLI data stored relative to the executable
**So that** I can run from a USB drive without leaving traces on the host system

**Acceptance Criteria**:
- [ ] If `./bw-data` directory exists next to executable, it is used
- [ ] Storage operations work correctly with relative paths
- [ ] Works across platforms (Windows, macOS, Linux)

### US-5: Migration from TypeScript CLI
**As a** existing Bitwarden CLI user
**I want** my existing configuration automatically recognized
**So that** I don't have to reconfigure when switching to the Rust CLI

**Acceptance Criteria**:
- [ ] Rust CLI can read TypeScript CLI's `data.json` format
- [ ] Field names match exactly (case-sensitive)
- [ ] Encrypted values with `__PROTECTED__` prefix decrypt correctly
- [ ] Missing optional fields are handled with sensible defaults

### US-6: Resilient to Corruption
**As a** CLI user
**I want** the CLI to handle corrupted storage gracefully
**So that** a corrupted file doesn't permanently break my CLI

**Acceptance Criteria**:
- [ ] Corrupted JSON is detected and reported clearly
- [ ] Backup of corrupted file is created (`.bak` extension)
- [ ] User is guided on recovery options
- [ ] Storage is re-initialized with defaults after corruption

## Functional Requirements

### FR-1: Storage Path Resolution (MUST HAVE)
**What**: Determine the correct storage directory based on environment
**Why**: Users need platform-appropriate storage locations and override options
**Validation**: Path resolution logic tests for all platforms and override scenarios

**Details**:
1. Check for `./bw-data` directory relative to executable path
2. If exists, use `./bw-data`
3. Else, check `BITWARDENCLI_APPDATA_DIR` environment variable
4. If set, resolve to absolute path and use it
5. Else, use platform-specific default path
6. Create directory if it doesn't exist (permissions: 0700)

**Edge Cases**:
- Directory exists but is not writable → clear error message
- Path contains special characters or Unicode → handle correctly
- Symlinks in path → resolve safely
- Very long paths (Windows MAX_PATH) → handle or error clearly

### FR-2: JSON Storage Operations (MUST HAVE)
**What**: Read/write operations on JSON-based storage
**Why**: Core data persistence mechanism
**Validation**: Unit tests for all CRUD operations

**Operations**:
- `get<T>(key: String) -> Result<Option<T>>` - Retrieve value by key
- `set<T>(key: String, value: T) -> Result<()>` - Store value by key
- `remove(key: String) -> Result<()>` - Delete value by key
- `has(key: String) -> Result<bool>` - Check if key exists

**Constraints**:
- Keys are strings
- Values must be JSON-serializable
- Nested JSON objects supported
- File operations must be atomic

### FR-3: Secure Storage with BW_SESSION (MUST HAVE)
**What**: Encrypt/decrypt sensitive values using BW_SESSION key
**Why**: Protect credentials and tokens at rest
**Validation**: Encryption/decryption round-trip tests

**Encryption Behavior**:
- Keys prefixed with `__PROTECTED__` indicate encrypted values
- Encryption uses `BW_SESSION` environment variable as key
- `BW_SESSION` is base64-encoded symmetric key (256-bit)
- Encrypted output is base64-encoded
- Decryption attempts without `BW_SESSION` return `None` or encrypted string

**Implementation Notes**:
- Must use Bitwarden SDK crypto primitives (NOT custom crypto)
- Algorithm must match TypeScript CLI (AES-256-CBC or compatible)
- Include integrity check (HMAC or authenticated encryption)

### FR-4: Atomic File Writes (MUST HAVE)
**What**: Prevent corruption during write operations
**Why**: Ensure data integrity even during crashes or concurrent access
**Validation**: Concurrent access tests, crash recovery tests

**Implementation Pattern**:
1. Write new data to temporary file in same directory (`.tmp` suffix)
2. Flush and sync file to disk
3. Atomically rename temp file to `data.json`
4. OS guarantees atomicity of rename operation

**Considerations**:
- Temp file same filesystem as target (for atomic rename)
- Handle disk full scenarios
- Clean up orphaned temp files on startup

### FR-5: State Data Structures (MUST HAVE)
**What**: Define Rust structs for persisted state
**Why**: Type-safe access to configuration and session data
**Validation**: Serialization tests, compatibility tests with TypeScript format

**Required State Types**:
1. **Environment URLs** - API, Identity, WebVault, Icons URLs
2. **Authentication State** - Access token, refresh token, token expiry
3. **User Profile** - User ID, email, account information
4. **Sync State** - Last sync timestamp, sync token
5. **KDF Configuration** - Algorithm (PBKDF2/Argon2id), iterations, memory, parallelism
6. **Vault Keys** - Encrypted master key, encrypted private key

**Design Considerations**:
- Use `serde` for JSON serialization
- Use `#[serde(rename = "...")]` to match TypeScript field names exactly
- Use `#[serde(default)]` for optional fields
- Use `Option<T>` for nullable fields
- Consider using `secrecy` crate for sensitive string types

### FR-6: TypeScript Storage Migration (MUST HAVE)
**What**: Read existing TypeScript CLI storage format
**Why**: Seamless migration for existing users
**Validation**: Integration tests with real TypeScript CLI storage files

**Compatibility Requirements**:
- Parse same JSON structure as TypeScript CLI
- Handle all existing field names (case-sensitive)
- Support same encryption format for `__PROTECTED__` keys
- Handle missing optional fields gracefully
- Preserve unknown fields (forward compatibility)

### FR-7: Error Handling and Recovery (MUST HAVE)
**What**: Handle storage errors gracefully with clear user feedback
**Why**: Users need actionable error messages, not crashes
**Validation**: Error scenario tests

**Error Scenarios**:
- Storage directory not writable → "Cannot write to [path]. Check permissions."
- JSON parse error → "Storage file corrupted. Backup created at [path].bak"
- Encryption key invalid → "Invalid BW_SESSION key. Cannot decrypt stored data."
- Disk full → "Insufficient disk space to save configuration."
- File locked by another process → "Storage file in use. Try again."

**Recovery Actions**:
- Create backup of corrupted file before reinitializing
- Provide clear recovery steps in error messages
- Log detailed error context for debugging (without sensitive data)

## Non-Functional Requirements

### NFR-1: Performance
**Target**: Storage operations complete in <50ms under normal conditions
**Measurement**: Benchmark tests for get/set/remove operations
**Rationale**: Storage is on critical path for CLI startup

**Optimizations**:
- Lazy initialization (only init on first access)
- Consider in-memory cache for frequently accessed values (future enhancement)
- Efficient JSON parsing with `serde_json`

### NFR-2: Security
**Target**: Sensitive data encrypted at rest, minimal exposure in memory
**Measurement**: Security audit, code review
**Rationale**: CLI stores credentials and vault keys

**Requirements**:
- Never log sensitive values (tokens, keys, passwords)
- Use `secrecy::Secret<String>` for sensitive types
- Zero sensitive data from memory after use (zeroize crate)
- File permissions: storage directory 0700, data file 0600
- Validate file paths to prevent directory traversal
- Handle symlinks safely (don't follow for writes)

### NFR-3: Reliability
**Target**: Zero data loss during normal and abnormal termination
**Measurement**: Crash recovery tests, concurrent access tests
**Rationale**: Users depend on persisted authentication state

**Requirements**:
- Atomic writes prevent corruption from crashes
- Handle concurrent CLI invocations safely
- Detect and recover from corrupted storage
- Maintain backup of previous state during writes

### NFR-4: Compatibility
**Target**: 100% compatibility with TypeScript CLI storage format
**Measurement**: Cross-compatibility integration tests
**Rationale**: Users must be able to use both CLIs interchangeably

**Requirements**:
- Identical JSON structure
- Identical field names (case-sensitive)
- Identical encryption format
- Handle superset of TypeScript fields (forward compat)

### NFR-5: Cross-Platform Support
**Target**: Works identically on Windows, macOS, Linux
**Measurement**: Platform-specific test suite
**Rationale**: CLI is cross-platform tool

**Requirements**:
- Path handling works on all platforms
- File permissions set correctly per platform
- Character encoding handled consistently (UTF-8)
- Line endings normalized (store as LF, accept CRLF)

## Project Dependencies

### Upstream Dependencies (Blockers)
1. **Enhancement 01: Project Bootstrap** - REQUIRED
   - Status: Must be complete
   - Provides: Cargo workspace, core crate structure, CLI parsing framework
   - Impact: Cannot start without project foundation

### Downstream Dependencies (Blocked By This)
1. **Enhancement 03: API Client** - BLOCKED
   - Needs: Storage for API URLs, authentication tokens
2. **Enhancement 04: Authentication** - BLOCKED
   - Needs: Storage for session tokens, user profile, KDF config
3. **Enhancement 05: Vault Read Commands** - BLOCKED
   - Needs: Storage for sync state, vault keys
4. **Enhancement 06: Vault Write Commands** - BLOCKED
   - Needs: Storage for vault keys
5. **Enhancement 07: Tool Commands** - BLOCKED
   - Needs: Storage for configuration
6. **Enhancement 08: Import/Export** - BLOCKED
   - Needs: Storage for authentication state

### External Dependencies
1. **Rust Crates**:
   - `serde`, `serde_json` - JSON serialization (already in workspace)
   - `directories` - Platform path resolution (already in workspace)
   - `tempfile` - Atomic write operations (need to add)
   - `secrecy` - Secure string handling (already in workspace)
   - `zeroize` - Clear sensitive memory (already in workspace)
   - `anyhow`/`thiserror` - Error handling (already in workspace)

2. **Bitwarden SDK**:
   - `bitwarden-crypto` - Encryption/decryption operations
   - Must NOT reimplement crypto - use SDK

## Technical Challenges and Risk Areas

### Challenge 1: Encryption Compatibility
**Description**: Rust implementation must match TypeScript encryption format exactly
**Risk Level**: High
**Impact**: Existing users cannot migrate if encryption incompatible
**Mitigation**:
- Use Bitwarden SDK crypto (same library as TypeScript WASM)
- Create comprehensive cross-compatibility tests
- Test with real TypeScript CLI storage files
- Document encryption format explicitly

**Questions for Architecture**:
- What specific encryption algorithm does TypeScript CLI use?
- Is it AES-256-CBC or AES-256-GCM?
- What key derivation is used for BW_SESSION?
- Are there test vectors available?

### Challenge 2: Concurrent Access
**Description**: Multiple CLI processes may access storage simultaneously
**Risk Level**: Medium
**Impact**: Data corruption or lost updates
**Mitigation**:
- TypeScript CLI uses file locking (`proper-lockfile` library)
- Rust should implement similar locking mechanism
- Use atomic writes as secondary protection
- Consider using `fs2` crate for cross-platform file locking

**Questions for Architecture**:
- Do we need file locking like TypeScript CLI?
- What is the expected concurrency pattern?
- Can we document "don't run multiple CLI instances simultaneously"?

### Challenge 3: Storage Format Evolution
**Description**: Future CLI versions may need different storage schema
**Risk Level**: Medium
**Impact**: Breaking changes for users, complex migrations
**Mitigation**:
- Design state structures with forward compatibility
- Use `#[serde(default)]` extensively
- Preserve unknown fields during read/write
- Consider adding version field to storage
- Plan for migration system (future enhancement)

**Questions for Architecture**:
- Should we add storage version field now?
- How to handle schema changes in future?
- What's the migration strategy?

### Challenge 4: BW_SESSION Key Format
**Description**: BW_SESSION format and usage not fully documented in spec
**Risk Level**: Medium
**Impact**: Cannot decrypt existing storage
**Mitigation**:
- Analyze TypeScript implementation in detail
- Document exact key format and derivation
- Create test cases with known keys

**Questions for Architecture**:
- Is BW_SESSION a raw symmetric key or derived key?
- What is the exact format (base64? hex? key size?)
- How is it generated initially?
- Where is the generation logic in TypeScript CLI?

### Challenge 5: Atomic Rename on Windows
**Description**: Windows atomic rename behavior differs from Unix
**Risk Level**: Low-Medium
**Impact**: Potential corruption on Windows during write
**Mitigation**:
- Use `std::fs::rename` (handles platform differences)
- Use `tempfile` crate (battle-tested)
- Test specifically on Windows
- Consider `fs2` or Windows-specific APIs if needed

**Questions for Architecture**:
- Are there Windows-specific considerations?
- Does TypeScript CLI have Windows-specific code?

## Integration Points

### 1. CLI Commands (Consumer)
**Interface**: Storage trait with get/set/remove/has methods
**Usage**: All commands need access to storage service
**Coupling**: High - storage is core dependency

**Example Usage**:
```rust
// Pseudo-code
let storage = ServiceContainer::storage();
let api_url = storage.get::<String>("environmentUrls.api")?;
let token = storage.get_secure::<String>("tokens.access_token")?;
```

### 2. Service Container (Integration)
**Interface**: Create and initialize storage service
**Usage**: Service container holds storage instance
**Coupling**: High - central integration point

**Initialization Pattern**:
```rust
// Pseudo-code
let storage = create_storage(&settings)?;
storage.init()?;
container.set_storage(storage);
```

### 3. Bitwarden SDK Crypto (Dependency)
**Interface**: Encryption/decryption functions
**Usage**: Secure storage uses SDK for crypto operations
**Coupling**: Medium - only for encrypted values

**Example Usage**:
```rust
// Pseudo-code
use bitwarden_crypto::{EncryptService, SymmetricCryptoKey};
let key = SymmetricCryptoKey::from_b64(bw_session)?;
let decrypted = encrypt_service.decrypt(encrypted_value, &key)?;
```

### 4. Environment Variables (External)
**Interface**: `std::env::var("BW_SESSION")`, `std::env::var("BITWARDENCLI_APPDATA_DIR")`
**Usage**: Read session key and custom storage path
**Coupling**: Low - standard environment access

### 5. File System (External)
**Interface**: `std::fs`, `std::path`
**Usage**: File I/O, directory creation, path resolution
**Coupling**: High - core dependency

## Constraints and Assumptions

### Technical Constraints
1. **Must use JSON format** - Required for TypeScript compatibility
2. **Must use Bitwarden SDK crypto** - No custom crypto implementations
3. **Must work without elevated permissions** - Standard user permissions only
4. **Must handle platform path differences** - Cross-platform requirement
5. **Must not lose data during writes** - Data integrity requirement

### Business Constraints
1. **Critical path item** - Blocks all subsequent enhancements
2. **Backward compatibility required** - Cannot break existing users
3. **No behavior changes** - Storage must work exactly like TypeScript CLI

### Assumptions
1. **Single user per storage directory** - No multi-user scenarios
2. **Storage is local filesystem** - No network storage
3. **JSON files are small** - Typically <1MB, can load entirely into memory
4. **BW_SESSION format is stable** - Not changing during development
5. **SDK is available** - Bitwarden SDK crates are accessible

## Success Criteria and Validation

### Definition of Done
- [ ] Storage directory created at correct platform-specific path
- [ ] `BITWARDENCLI_APPDATA_DIR` environment variable override works
- [ ] `./bw-data` relative directory detection works
- [ ] JSON storage operations (get/set/remove/has) work correctly
- [ ] Secure storage encrypts values with `__PROTECTED__` prefix
- [ ] Values encrypted with BW_SESSION decrypt correctly
- [ ] Atomic writes prevent corruption (tested with crash simulation)
- [ ] Can read existing TypeScript CLI storage files
- [ ] Corrupted storage handled gracefully with backup
- [ ] Unit tests pass (>90% coverage of storage code)
- [ ] Integration tests pass (including cross-platform)
- [ ] Performance tests pass (<50ms for typical operations)

### Validation Approach

**1. Unit Tests**
- JSON serialization/deserialization of all state types
- Path resolution logic for each platform
- Atomic write mechanism (using temp file)
- Error handling for each error scenario
- Secure storage encryption/decryption

**2. Integration Tests**
- Read real TypeScript CLI `data.json` files
- Write and read back complex state
- Concurrent access simulation
- Corruption recovery workflow
- Permission error handling

**3. Manual Testing**
- Test on Windows, macOS, Linux
- Test with `BITWARDENCLI_APPDATA_DIR` set
- Test with `./bw-data` directory
- Manually corrupt `data.json`, verify recovery
- Test with various BW_SESSION keys

**4. Compatibility Testing**
- Generate storage with TypeScript CLI, read with Rust CLI
- Generate storage with Rust CLI, read with TypeScript CLI
- Verify encrypted values decrypt correctly in both directions

## Open Questions for Architecture Phase

### Critical Questions (Must Answer Before Implementation)
1. **Encryption Algorithm**: What exact encryption algorithm and mode does the TypeScript CLI use for `__PROTECTED__` values?
   - Is it AES-256-CBC, AES-256-GCM, or other?
   - What is the IV generation/storage mechanism?
   - What integrity check is used (HMAC, authenticated encryption)?

2. **BW_SESSION Format**: What is the exact format and derivation of BW_SESSION?
   - Is it a raw 256-bit key or derived from something?
   - What encoding (base64, hex)?
   - How is it initially generated?

3. **File Locking**: Do we need file locking like TypeScript CLI?
   - TypeScript uses `proper-lockfile` with retry logic
   - Is this necessary for Rust CLI?
   - What are the actual concurrent use cases?

4. **Storage Versioning**: Should we add a version field to storage now?
   - Easier to add now than retrofit later
   - How to handle future schema changes?

### Important Questions (Should Answer)
5. **State Structure Details**: What are the exact field names and structure for each state type?
   - Need TypeScript interface definitions or sample data
   - Which fields are required vs optional?
   - Which fields are encrypted?

6. **Migration Strategy**: How to handle incompatible storage changes in future versions?
   - One-way migration acceptable?
   - Maintain backward compatibility indefinitely?

7. **Error Recovery**: What's the expected user workflow when storage is corrupted?
   - Re-login? Re-sync? Manual recovery?

8. **Performance Requirements**: Are there any specific performance requirements beyond <50ms?
   - Expected storage file size?
   - Frequency of access?

### Nice to Have Questions
9. **Caching Strategy**: Should we cache frequently accessed values in memory?
   - Trade-off: memory usage vs performance
   - Would complicate concurrent access

10. **Multiple Profiles**: Should storage support multiple profiles/users?
    - TypeScript CLI doesn't currently
    - Could design in now for future

## Recommendations for Next Phase

### For Architecture Team

**1. Research and Document Encryption**
- Analyze TypeScript encryption implementation in detail
- Document exact algorithm, mode, IV handling, integrity check
- Create test vectors for validation
- Ensure Bitwarden SDK has required crypto primitives

**2. Design Storage Trait**
- Define trait interface for storage operations
- Support both plain and secure storage
- Enable mock implementations for testing
- Consider generic vs concrete types

**3. Design State Structures**
- Map TypeScript interfaces to Rust structs
- Document all field names (case-sensitive)
- Identify required vs optional fields
- Identify encrypted fields
- Plan for forward compatibility

**4. Design Error Types**
- Define error hierarchy for storage operations
- Include context for debugging
- Map to user-friendly messages
- Consider recovery actions

**5. Decide on File Locking**
- Analyze concurrent access patterns
- Decide if file locking needed
- Document concurrency constraints
- Plan testing approach

### For Implementation Team

**1. Start with Path Resolution**
- Implement platform detection and path logic
- Test on all platforms
- Handle edge cases (special chars, long paths)

**2. Implement Basic JSON Storage**
- Atomic write pattern with tempfile
- Basic get/set/remove/has operations
- Error handling
- Unit tests

**3. Add Secure Storage**
- Integrate Bitwarden SDK crypto
- Implement __PROTECTED__ key handling
- Encryption/decryption logic
- BW_SESSION parsing

**4. Implement State Structures**
- Define Rust structs with serde
- Match TypeScript field names exactly
- Test serialization

**5. Add TypeScript Compatibility**
- Read real TypeScript storage files
- Integration tests
- Handle edge cases

### For Testing Team

**1. Create Test Data Set**
- Generate TypeScript CLI storage files
- Include various scenarios (encrypted, plain, missing fields)
- Document expected values

**2. Build Compatibility Test Suite**
- Rust CLI reads TypeScript storage
- TypeScript CLI reads Rust storage (manual validation)
- Round-trip tests

**3. Build Concurrency Test Suite**
- Simulate concurrent CLI invocations
- Test file locking behavior
- Verify no corruption

**4. Build Platform Test Suite**
- Automated tests on Windows, macOS, Linux
- Path resolution verification
- Permission handling

## Summary

The storage layer is a critical foundation for the Bitwarden CLI Rust migration, requiring careful attention to:

1. **Backward Compatibility** - Must read existing TypeScript CLI storage flawlessly
2. **Security** - Proper encryption of sensitive values using BW_SESSION
3. **Reliability** - Atomic writes and corruption recovery
4. **Cross-Platform** - Consistent behavior on all platforms

**Key Success Factors**:
- Use Bitwarden SDK for all cryptographic operations
- Match TypeScript field names and structure exactly
- Implement robust error handling and recovery
- Comprehensive testing including cross-compatibility

**Primary Risks**:
- Encryption format compatibility issues
- Concurrent access edge cases
- Platform-specific path or permission issues

**Readiness for Architecture Phase**: ✅ Ready
All functional requirements are clearly defined. Architecture team can proceed with detailed technical design, focusing on the open questions identified above.
