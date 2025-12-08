---
enhancement: 07-tool-commands
agent: requirements-analyst
task_id: task_1764954344_19703
timestamp: 2025-12-05T00:00:00Z
status: READY_FOR_ARCHITECTURE
---

# Requirements Analysis: Tool Commands Enhancement

## Executive Summary

This enhancement implements four categories of tool commands for the Bitwarden CLI Rust migration:

1. **Password Generation** (`bw generate`) - Create secure passwords and passphrases
2. **Send Operations** (`bw send`) - Temporary secure sharing with encryption
3. **Receive** (`bw receive`) - Access shared Send content
4. **Encode** (`bw encode`) - Base64 encoding utility

These commands provide standalone utilities that work independently of the vault, though Send operations require authentication. This is enhancement 7 of 8 in the migration roadmap.

## Project Context

### Current Implementation State

**Completed Infrastructure:**
- ✅ CLI argument parsing structure exists (crates/bw-cli/src/commands/tools.rs, send.rs)
- ✅ Command structures defined with proper clap attributes
- ✅ GlobalArgs support with session, quiet, response, raw, pretty flags
- ✅ Response formatting system in place
- ✅ Storage layer available for encryption keys and session data
- ✅ API client infrastructure available
- ❌ SDK integration is mock-only (crates/bw-core/src/services/sdk.rs)
- ❌ All command implementations are stubs returning "Not yet implemented"
- ❌ Receive command not yet defined in CLI structure

**Key Finding:** The CLI scaffolding is complete but all business logic needs implementation.

### Dependency Analysis

**Hard Dependencies (Must Be Complete):**
1. Enhancement 1: Project Bootstrap ✅ (Complete - structure exists)
2. Enhancement 2: Storage Layer ✅ (Complete - needed for Send keys)
3. Enhancement 3: API Client ✅ (Complete - needed for Send operations)
4. Enhancement 4: Authentication ✅ (Complete - needed for Send CRUD)

**Technical Dependencies:**
- Bitwarden SDK: Currently mocked, real SDK needed for:
  - Send encryption/decryption
  - Password generation (or implement with `rand` crate)
- `rand` crate with `OsRng`: Available (0.8 in workspace)
- `base64` crate: Available (0.22 in workspace)

## User Stories

### US-1: Password Generation
**As a** CLI user
**I want to** generate secure passwords with customizable options
**So that** I can create strong passwords for accounts without storing them in my vault

**Acceptance Criteria:**
- [ ] `bw generate` creates 14-character password by default
- [ ] `--length N` controls password length
- [ ] `--uppercase`, `--lowercase`, `--number`, `--special` control minimum character counts
- [ ] Generated passwords are cryptographically secure
- [ ] Output is plaintext (not JSON) by default
- [ ] `--response` flag returns JSON format
- [ ] Password generation completes in <100ms

**Priority:** Must Have (MVP)

### US-2: Passphrase Generation
**As a** CLI user
**I want to** generate memorable passphrases
**So that** I can create secure but easy-to-type passwords

**Acceptance Criteria:**
- [ ] `bw generate --passphrase` creates 3-word passphrase by default
- [ ] `--words N` controls word count
- [ ] `--separator X` controls delimiter (default: -)
- [ ] `--capitalize` capitalizes first letter of each word
- [ ] `--include-number` adds number to passphrase
- [ ] Uses secure word list
- [ ] Output format matches password generation

**Priority:** Must Have (MVP)

### US-3: Text Send Creation
**As a** CLI user
**I want to** create secure temporary text Sends
**So that** I can share sensitive information with expiration and access limits

**Acceptance Criteria:**
- [ ] `bw send create` accepts JSON-encoded Send object
- [ ] `--text "content"` provides text content directly
- [ ] `--hidden` flag hides text by default
- [ ] Returns Send URL on success
- [ ] Encrypts content before upload
- [ ] Requires authentication (--session or BW_SESSION)
- [ ] Supports expiration date in JSON
- [ ] Supports max access count in JSON
- [ ] Supports password protection in JSON
- [ ] Operations complete in <2s

**Priority:** Must Have (MVP)

### US-4: Send Management
**As a** CLI user
**I want to** list, retrieve, and delete my Sends
**So that** I can manage temporary shared content

**Acceptance Criteria:**
- [ ] `bw send list` shows all user's Sends with IDs, names, types
- [ ] `bw send get <id>` returns full Send details as JSON
- [ ] `bw send delete <id>` removes Send and returns success
- [ ] All operations require authentication
- [ ] Support `--response` flag for JSON output
- [ ] List shows expiration status
- [ ] Get includes access count and max access count

**Priority:** Must Have (MVP)

### US-5: Send Retrieval
**As a** CLI user
**I want to** retrieve content from a Send URL
**So that** I can access shared information

**Acceptance Criteria:**
- [ ] `bw receive <url>` retrieves Send content
- [ ] Works without authentication
- [ ] Prompts for password if Send is password-protected
- [ ] Decrypts content locally
- [ ] Returns text content or file download
- [ ] Handles expired Sends with clear error
- [ ] Handles access limit exceeded with clear error
- [ ] Supports `--nointeraction` flag to fail instead of prompt

**Priority:** Must Have (MVP)

### US-6: Base64 Encoding
**As a** CLI user
**I want to** encode data to base64
**So that** I can prepare data for API calls or other tools

**Acceptance Criteria:**
- [ ] `bw encode <data>` returns base64-encoded string
- [ ] Handles arbitrary text input
- [ ] Returns only encoded string (no JSON wrapper)
- [ ] Works without authentication

**Priority:** Must Have (MVP)

### US-7: File Send Creation (Optional)
**As a** CLI user
**I want to** create Sends with file attachments
**So that** I can securely share files temporarily

**Acceptance Criteria:**
- [ ] `bw send create --file <path>` uploads file
- [ ] Encrypts file content before upload
- [ ] Handles large files efficiently (streaming)
- [ ] Shows progress indicator for large files
- [ ] Respects API file size limits
- [ ] Returns Send URL on success

**Priority:** Should Have (Optional)

### US-8: Send Template
**As a** CLI user
**I want to** get a Send template JSON structure
**So that** I can understand the format for creating Sends

**Acceptance Criteria:**
- [ ] `bw send template` returns text Send template
- [ ] `bw send template file` returns file Send template
- [ ] Template includes all configurable fields
- [ ] Template has reasonable defaults

**Priority:** Should Have (Optional)

### US-9: Send Editing
**As a** CLI user
**I want to** modify existing Send properties
**So that** I can update expiration or access limits

**Acceptance Criteria:**
- [ ] `bw send edit <id> <json>` updates Send
- [ ] Can modify name, notes, expiration
- [ ] Can modify max access count
- [ ] Cannot modify Send type or content
- [ ] Returns updated Send details

**Priority:** Should Have (Optional)

### US-10: Send Password Removal
**As a** CLI user
**I want to** remove password protection from a Send
**So that** I can make it easier to access

**Acceptance Criteria:**
- [ ] `bw send remove-password <id>` removes password
- [ ] Requires authentication
- [ ] Updates Send on server
- [ ] Returns success confirmation

**Priority:** Should Have (Optional)

## Functional Requirements

### FR-1: Cryptographically Secure Random Generation
**Description:** Password and passphrase generation must use cryptographically secure random number generation.

**Details:**
- Use `rand::rngs::OsRng` for randomness
- No pseudo-random generators (like `thread_rng`) for password generation
- Word selection for passphrases must be uniformly random
- Character selection must be unbiased across character sets

**Rationale:** Weak randomness compromises password security.

**Validation:** Unit tests verify entropy source; security review confirms CSPRNG usage.

### FR-2: Password Character Set Constraints
**Description:** Password generation must enforce valid character set constraints.

**Details:**
- Default: uppercase + lowercase + numbers + special characters
- Length: 5-128 characters (reasonable bounds)
- Minimum counts: sum of minimums must not exceed length
- Character sets:
  - Lowercase: a-z
  - Uppercase: A-Z
  - Numbers: 0-9
  - Special: !@#$%^&*
- Support for excluding specific characters (optional)

**Validation:** Unit tests verify all combinations; edge cases tested.

### FR-3: Passphrase Word List
**Description:** Passphrases must use a curated word list for generation.

**Details:**
- Use EFF long wordlist (7776 words) or similar
- Words should be:
  - Easy to type
  - Easy to remember
  - Unambiguous
- Word count: 3-20 words (reasonable bounds)
- Separator: any single character, default "-"

**Open Question:** Should we embed word list or use SDK? (Flag for architect)

**Validation:** Verify word list source; test word selection distribution.

### FR-4: Send Encryption Format
**Description:** Send content must be encrypted in a format compatible with Bitwarden web vault.

**Details:**
- Use Bitwarden SDK encryption methods
- Encryption key derived from Send key
- Format must match web vault's Send implementation
- Encrypted payload structure:
  - Text Sends: encrypted text + metadata
  - File Sends: encrypted file data + metadata

**Open Question:** Does SDK provide Send encryption, or must we implement? (Flag for architect)

**Validation:** Cross-validate with web vault; ensure receive works across platforms.

### FR-5: Send API Operations
**Description:** Send CRUD operations must use Bitwarden Send API endpoints.

**Details:**
- Create: POST /sends with encrypted payload
- List: GET /sends (user's Sends only)
- Get: GET /sends/{id}
- Edit: PUT /sends/{id}
- Delete: DELETE /sends/{id}
- Remove Password: PUT /sends/{id}/remove-password
- All operations require Bearer token authentication
- Receive: GET /sends/access/{id} (public, no auth)

**Dependency:** API client must support these endpoints.

**Validation:** Integration tests with test account; API contract tests.

### FR-6: Send Access Validation
**Description:** Receive command must validate Send access conditions before returning content.

**Details:**
- Check expiration date (server-side validation)
- Check max access count (server-side validation)
- Prompt for password if required
- Handle deletion token for file cleanup
- Error cases:
  - Send not found
  - Send expired
  - Access limit exceeded
  - Invalid password
  - Decryption failure

**Validation:** Test all error scenarios; verify error messages are clear.

### FR-7: Base64 Encoding
**Description:** Encode command must perform standard base64 encoding.

**Details:**
- Use standard base64 alphabet (RFC 4648)
- Accept input from command-line argument
- No line wrapping in output
- Return only encoded string (no JSON unless --response)

**Validation:** Test against known base64 encodings.

### FR-8: Authentication Requirements
**Description:** Commands have different authentication requirements.

**Details:**
- **No authentication required:**
  - `bw generate`
  - `bw encode`
  - `bw receive`
- **Authentication required:**
  - `bw send create`
  - `bw send list`
  - `bw send get`
  - `bw send edit`
  - `bw send delete`
  - `bw send remove-password`

**Validation:** Verify unauthenticated commands work without session; verify authenticated commands fail without session.

## Non-Functional Requirements

### NFR-1: Performance
**Description:** Commands must complete within acceptable time limits.

**Targets:**
- Password/passphrase generation: <100ms
- Send create (text): <2s
- Send create (file <1MB): <5s
- Send list/get/delete: <2s
- Receive (text): <2s
- Encode: <100ms

**Rationale:** Users expect instant feedback for simple operations.

**Validation:** Performance tests with profiling.

### NFR-2: Memory Efficiency
**Description:** Commands must handle data efficiently, especially for file Sends.

**Requirements:**
- Password generation: minimal allocation
- File Sends: streaming upload/download, not full-file buffering
- Maximum memory for file operations: 10MB buffer
- Clear sensitive data (passwords, keys) from memory after use

**Rationale:** Large files should not cause OOM; security requires clearing secrets.

**Validation:** Memory profiling; test with large files; verify zeroization.

### NFR-3: Security
**Description:** All cryptographic operations must meet security best practices.

**Requirements:**
- CSPRNG for all random generation
- Zeroize sensitive data after use
- No logging of generated passwords or encryption keys
- Use Bitwarden SDK for encryption (don't roll custom crypto)
- Validate all inputs to prevent injection attacks
- File size limits to prevent resource exhaustion

**Rationale:** Security is paramount for password manager.

**Validation:** Security review; audit logs for leaks; fuzzing for injection.

### NFR-4: Compatibility
**Description:** Output format must match TypeScript CLI for migration compatibility.

**Requirements:**
- Password/passphrase output: plaintext string
- Send operations: JSON matching TypeScript format
- Error messages: similar wording and structure
- `--response` flag behavior: identical JSON structure
- Exit codes: match TypeScript CLI

**Rationale:** Users and scripts expect consistent behavior during migration.

**Validation:** Compare outputs with TypeScript CLI; automated compatibility tests.

### NFR-5: Reliability
**Description:** Commands must handle errors gracefully and provide clear feedback.

**Requirements:**
- Network errors: clear message, suggest retry
- API errors: show error code and message from server
- Validation errors: explain what's wrong and how to fix
- Expired Sends: clear message, no stack trace
- File not found: helpful error with path
- All errors respect `--quiet` and `--cleanexit` flags

**Rationale:** Poor error messages frustrate users.

**Validation:** Test all error paths; user acceptance testing.

## Technical Flags for Architecture Team

### TF-1: SDK Generator Integration
**Question:** Should we use Bitwarden SDK for password generation, or implement directly with `rand`?

**Context:**
- SDK may provide generator APIs matching web vault
- SDK is currently mocked in the codebase
- `rand` crate is available and sufficient for implementation

**Recommendation:** Check SDK capabilities. If SDK provides generator, use it for consistency. Otherwise, implement with `rand::rngs::OsRng`.

### TF-2: Send Encryption Implementation
**Question:** Does Bitwarden SDK provide Send-specific encryption, or must we implement it?

**Context:**
- Send encryption format must match web vault
- SDK provides general encryption but Send encryption may differ
- TypeScript CLI uses SDK methods for Send operations

**Recommendation:** Research SDK Send APIs. If not available, architect must design encryption matching web vault format.

### TF-3: File Send Streaming
**Question:** How to handle file Send uploads efficiently for large files?

**Context:**
- Files could be multiple MB or larger
- Memory efficiency requires streaming
- reqwest supports streaming request bodies
- API may require full payload or support chunked upload

**Recommendation:** Investigate API requirements for file uploads. Design streaming solution if API supports it, otherwise implement with reasonable file size limits.

### TF-4: Passphrase Word List Storage
**Question:** Should we embed EFF word list in binary, load from SDK, or load from external file?

**Context:**
- EFF long wordlist is ~7776 words, ~60KB
- Embedding increases binary size but guarantees availability
- SDK may provide word list
- External file adds deployment complexity

**Recommendation:** Prefer SDK if available. If not, embed word list in binary for simplicity and reliability.

### TF-5: Receive Command Structure
**Question:** Should `receive` be a top-level command or under `send receive`?

**Context:**
- TypeScript CLI uses `bw receive <url>` (top-level)
- Current Rust CLI doesn't define receive command yet
- Receive is semantically related to Send but operates on public data

**Recommendation:** Follow TypeScript CLI convention - make it a top-level command for consistency.

### TF-6: Send Model Definitions
**Question:** Where should Send models be defined and what structure should they have?

**Context:**
- No Send models exist in `crates/bw-core/src/models/`
- Need models for Send, SendText, SendFile, SendAccess
- Models should match API response structure
- Need separate request models for creation

**Recommendation:** Architect should define Send models in `crates/bw-core/src/models/send/` following existing vault model patterns.

## Open Questions

### OQ-1: Default Password Parameters
**Question:** What are the exact default values for password generation?

**TypeScript CLI Defaults:**
- Length: 14
- Uppercase: true
- Lowercase: true
- Numbers: true
- Special: true
- Minimum numbers: 1
- Minimum special: 1

**Action:** Confirm these match TypeScript CLI; document in architecture spec.

### OQ-2: Send File Size Limits
**Question:** What's the maximum file size for Send?

**Context:**
- API may enforce size limits
- Large files affect performance and memory
- Need to validate before upload

**Action:** Research API documentation; test with large files; document limit.

### OQ-3: Send Expiration Editing
**Question:** Can Send expiration date be edited after creation?

**Context:**
- Edit command exists but unclear what's mutable
- API may restrict changes to certain fields

**Action:** Test with TypeScript CLI; verify API behavior; document constraints.

### OQ-4: Receive Password Prompt
**Question:** Should receive prompt for password if not provided, or accept it as a flag?

**Context:**
- TypeScript CLI prompts interactively
- CLI supports `--nointeraction` flag
- May want `--password` flag for scripting

**Action:** Follow TypeScript CLI pattern; support both interactive and `--password` flag.

### OQ-5: Generate Excluded Characters
**Question:** Is the `--excludedCharacters` flag required for MVP?

**Context:**
- Listed in specification as a feature
- Not marked as "must have"
- Adds complexity to character set handling

**Action:** Clarify with stakeholders; implement if time permits.

### OQ-6: Send Authentication Scope
**Question:** Do Send operations require a vault unlock, or just login?

**Context:**
- Send operations access server, not local vault
- May only need access token, not encryption key
- Could work in "locked" state

**Action:** Test TypeScript CLI behavior; verify what authentication state is required.

## Project Phases & Milestones

### Phase 1: Password Generation (Priority: High)
**Goal:** Implement generate command with password and passphrase support.

**Tasks:**
1. Implement password generation logic with character sets
2. Implement passphrase generation with word list
3. Handle all command-line options and validation
4. Add unit tests for generation logic
5. Add integration tests for CLI interface
6. Compare output with TypeScript CLI

**Deliverables:**
- Working `bw generate` command
- Working `bw generate --passphrase` command
- All options functional (length, character sets, words, separator)
- Test suite covering edge cases
- Documentation for generation options

**Estimated Complexity:** Medium (3-5 days)

**Success Criteria:**
- Generates cryptographically secure passwords
- Output matches TypeScript CLI format
- All tests passing
- Performance meets <100ms target

### Phase 2: Encode Utility (Priority: High)
**Goal:** Implement encode command for base64 encoding.

**Tasks:**
1. Implement base64 encoding using `base64` crate
2. Handle output formatting
3. Add tests

**Deliverables:**
- Working `bw encode` command
- Test suite

**Estimated Complexity:** Low (1 day)

**Success Criteria:**
- Produces standard base64 output
- Matches TypeScript CLI output

### Phase 3: Send Models & Encryption (Priority: High)
**Goal:** Define Send data models and implement encryption.

**Tasks:**
1. Research SDK Send APIs and encryption methods
2. Define Send models (Send, SendText, SendFile, SendRequest)
3. Implement Send encryption/decryption using SDK
4. Add unit tests for encryption
5. Validate encryption format with web vault

**Deliverables:**
- Send models in `crates/bw-core/src/models/send/`
- Send encryption service in `crates/bw-core/src/services/send/`
- Encryption tests

**Estimated Complexity:** High (5-7 days)

**Success Criteria:**
- Models match API structure
- Encryption format compatible with web vault
- Encryption/decryption tests passing

### Phase 4: Send CRUD Commands (Priority: High)
**Goal:** Implement Send create, list, get, delete commands.

**Tasks:**
1. Implement API client methods for Send endpoints
2. Implement `bw send create` with text support
3. Implement `bw send list`
4. Implement `bw send get`
5. Implement `bw send delete`
6. Add authentication checks
7. Add integration tests with test account
8. Compare output with TypeScript CLI

**Deliverables:**
- Working Send CRUD commands
- API integration
- Test suite with live API tests
- Documentation

**Estimated Complexity:** High (5-7 days)

**Success Criteria:**
- Can create text Sends with encryption
- Can list and retrieve Sends
- Can delete Sends
- Output matches TypeScript CLI
- All tests passing

### Phase 5: Receive Command (Priority: High)
**Goal:** Implement receive command for accessing Send content.

**Tasks:**
1. Define `receive` command structure in CLI
2. Implement API call to Send access endpoint
3. Implement password prompting if needed
4. Implement decryption of received content
5. Handle error cases (expired, access limit)
6. Add tests for receive scenarios

**Deliverables:**
- Working `bw receive` command
- Error handling for all failure modes
- Test suite

**Estimated Complexity:** Medium (3-5 days)

**Success Criteria:**
- Can receive Send content by URL
- Handles password-protected Sends
- Clear errors for expired/invalid Sends
- Works without authentication

### Phase 6: Optional Send Features (Priority: Medium)
**Goal:** Implement nice-to-have Send features.

**Tasks:**
1. Implement `bw send template`
2. Implement `bw send edit`
3. Implement `bw send remove-password`
4. Implement `bw send create --file` (file Send support)
5. Add tests for optional features

**Deliverables:**
- Working optional commands
- File Send support (if time permits)
- Test coverage

**Estimated Complexity:** Medium-High (4-6 days)

**Success Criteria:**
- Template returns valid JSON
- Edit and remove-password work correctly
- File Send works with streaming (if implemented)

### Phase 7: Integration & Polish (Priority: High)
**Goal:** Ensure all commands work together and output matches TypeScript CLI.

**Tasks:**
1. Run full compatibility test suite against TypeScript CLI
2. Fix any format discrepancies
3. Verify error messages match
4. Performance profiling and optimization
5. Security review of crypto operations
6. Documentation updates

**Deliverables:**
- Full compatibility with TypeScript CLI
- Performance meeting targets
- Security sign-off
- Complete documentation

**Estimated Complexity:** Medium (3-4 days)

**Success Criteria:**
- All commands produce compatible output
- No security vulnerabilities
- Performance targets met
- Documentation complete

## Dependencies & Integration Points

### Upstream Dependencies (Required)
1. **Enhancement 1: Project Bootstrap** ✅
   - Provides: CLI structure, cargo configuration
   - Status: Complete

2. **Enhancement 2: Storage Layer** ✅
   - Provides: Secure storage for Send encryption keys
   - Status: Complete
   - Integration: Read/write Send keys from storage

3. **Enhancement 3: API Client** ✅
   - Provides: HTTP client for Send API calls
   - Status: Complete
   - Integration: Add Send endpoints to API client

4. **Enhancement 4: Authentication** ✅
   - Provides: Session management, authentication tokens
   - Status: Complete
   - Integration: Check authentication for Send CRUD operations

### External Dependencies
1. **Bitwarden SDK**
   - Currently: Mocked
   - Needed for:
     - Send encryption/decryption (critical)
     - Password generation (optional - can use rand)
   - Risk: If SDK doesn't provide Send APIs, must implement from scratch
   - Mitigation: Research SDK capabilities early in architecture phase

2. **EFF Word List**
   - Needed for: Passphrase generation
   - Options: Embed in binary, load from SDK, external file
   - Recommendation: Embed for reliability

### Downstream Dependencies (Blocked by this enhancement)
1. **Enhancement 8: Import/Export**
   - May use password generation for secure exports
   - Not strictly dependent, but could benefit

## Constraints & Limitations

### Technical Constraints
1. **SDK Availability:** Real Bitwarden SDK not yet integrated; currently mocked
2. **API Compatibility:** Must match existing Bitwarden API contract for Send operations
3. **Encryption Format:** Send encryption must match web vault format for interoperability
4. **File Size:** API enforces file size limits (exact limit TBD)
5. **Memory:** File operations must use streaming to avoid large memory allocation

### Business Constraints
1. **Migration Compatibility:** Output must match TypeScript CLI for smooth migration
2. **Timeline:** Part of larger 8-enhancement roadmap; should not block enhancement 8
3. **Feature Parity:** Goal is feature parity with TypeScript CLI, not innovation

### Implementation Constraints
1. **Testing:** Send operations require live API for integration tests (need test account)
2. **Cross-Platform:** Word list must work on all platforms (Windows, macOS, Linux)
3. **Security:** Crypto operations require security review before release

## Success Criteria

### Definition of Done
- [ ] All "Must Have" commands implemented and tested
- [ ] `bw generate` creates secure passwords with all options
- [ ] `bw generate --passphrase` creates secure passphrases
- [ ] `bw send create` creates encrypted text Sends
- [ ] `bw send list/get/delete` manage Sends correctly
- [ ] `bw receive` retrieves and decrypts Send content
- [ ] `bw encode` performs base64 encoding
- [ ] All commands respect global flags (--quiet, --response, etc.)
- [ ] Output format matches TypeScript CLI
- [ ] Unit tests cover all generation and encryption logic
- [ ] Integration tests validate end-to-end flows
- [ ] Performance meets targets (<100ms for generate, <2s for Send)
- [ ] Security review completed with no critical issues
- [ ] Documentation complete with examples
- [ ] No regression in existing commands

### Acceptance Tests

**AT-1: Basic Password Generation**
```bash
# Test: Generate default password
bw generate
# Expected: 14-character password with mixed character types
# Validation: Check length, character sets present, no JSON wrapper
```

**AT-2: Custom Password Options**
```bash
# Test: Generate 20-character password
bw generate --length 20
# Expected: 20-character password
# Validation: Verify length

# Test: Password with minimum requirements
bw generate --length 16 --number 3 --special 2
# Expected: 16-char password with at least 3 numbers and 2 special chars
# Validation: Count character types
```

**AT-3: Passphrase Generation**
```bash
# Test: Generate default passphrase
bw generate --passphrase
# Expected: 3 words separated by hyphens
# Validation: Split by separator, count words

# Test: Custom passphrase options
bw generate --passphrase --words 5 --separator _ --capitalize --include-number
# Expected: 5 capitalized words with underscores and a number
# Validation: Verify format
```

**AT-4: Text Send Creation**
```bash
# Test: Create text Send
bw send create --text "Secret message" --session <key>
# Expected: JSON with Send URL
# Validation: JSON contains "accessUrl", HTTP 200

# Test: Create Send with JSON
bw send create '{"name":"Test","text":{"text":"Content"},"deletionDate":"2025-12-31"}' --session <key>
# Expected: Send created with expiration
# Validation: Get Send and verify expiration date
```

**AT-5: Send Management**
```bash
# Test: List Sends
bw send list --session <key>
# Expected: JSON array of Sends
# Validation: Array contains expected fields

# Test: Get Send details
bw send get <id> --session <key>
# Expected: JSON object with full Send details
# Validation: Contains id, name, type, accessUrl

# Test: Delete Send
bw send delete <id> --session <key>
# Expected: Success message or empty response
# Validation: Get should fail after delete
```

**AT-6: Receive Send Content**
```bash
# Test: Receive public Send
bw receive https://send.bitwarden.com/#/access/ABC123
# Expected: Decrypted content printed
# Validation: Content matches original

# Test: Receive password-protected Send (with password flag)
bw receive https://send.bitwarden.com/#/access/ABC123 --password "test123"
# Expected: Decrypted content
# Validation: Content matches original

# Test: Receive expired Send
bw receive <expired-url>
# Expected: Error message indicating Send expired
# Validation: Non-zero exit code (unless --cleanexit)
```

**AT-7: Base64 Encoding**
```bash
# Test: Encode text
bw encode "Hello, World!"
# Expected: SGVsbG8sIFdvcmxkIQ==
# Validation: Exact match

# Test: Encode with response flag
bw encode "test" --response
# Expected: JSON with encoded value
# Validation: Valid JSON structure
```

**AT-8: Error Handling**
```bash
# Test: Generate with invalid length
bw generate --length 1000
# Expected: Error message about invalid length
# Validation: Clear error, exit code 1

# Test: Send without authentication
bw send list
# Expected: Error about missing authentication
# Validation: Message prompts for login or --session

# Test: Receive invalid URL
bw receive https://invalid-url
# Expected: Error message
# Validation: Clear error, exit code 1
```

**AT-9: Global Flags**
```bash
# Test: Quiet mode suppresses output
bw generate --quiet
# Expected: No output
# Validation: stdout empty

# Test: Response format
bw send list --response --session <key>
# Expected: JSON response
# Validation: Valid JSON

# Test: Clean exit on error
bw send list --cleanexit
# Expected: Exit code 0 even without auth
# Validation: Exit code is 0
```

**AT-10: Cross-Platform Compatibility**
```bash
# Test: Receive on different platform than create
# 1. Create Send on Windows CLI
# 2. Receive on Linux CLI
# Expected: Content decrypts correctly
# Validation: Cross-platform encryption compatibility
```

## Security Considerations

### SEC-1: Cryptographic Randomness
**Risk:** Use of weak RNG compromises password security.

**Mitigation:**
- Use `rand::rngs::OsRng` exclusively for password/passphrase generation
- Never use `thread_rng()` or other PRNGs for security-critical randomness
- Add tests to verify OsRng usage
- Security review of RNG implementation

### SEC-2: Memory Clearing
**Risk:** Sensitive data remains in memory after use, vulnerable to memory dumps.

**Mitigation:**
- Use `zeroize` crate for clearing passwords and keys
- Clear data immediately after use
- Apply `#[zeroize(drop)]` to types containing secrets
- Test with memory profiler to verify clearing

### SEC-3: Logging Sensitive Data
**Risk:** Generated passwords or encryption keys logged to files or console.

**Mitigation:**
- Never log generated passwords
- Never log encryption keys or tokens
- Review all `tracing` calls in sensitive code paths
- Use `secrecy::Secret` wrapper for sensitive data

### SEC-4: Encryption Implementation
**Risk:** Custom encryption implementation may have vulnerabilities.

**Mitigation:**
- Use Bitwarden SDK encryption methods (don't roll own crypto)
- If SDK unavailable, use well-vetted crypto libraries
- Security review of all encryption code
- Cross-validate with web vault to ensure format correctness

### SEC-5: Input Validation
**Risk:** Malicious inputs could cause crashes or injection attacks.

**Mitigation:**
- Validate all user inputs (length bounds, character sets, URL format)
- Sanitize inputs before passing to external systems
- Use parameterized API calls (not string concatenation)
- Test with fuzzing to find edge cases

### SEC-6: File Send Handling
**Risk:** Large or malicious file Sends could cause DoS or resource exhaustion.

**Mitigation:**
- Enforce file size limits
- Use streaming to avoid loading entire file in memory
- Validate file metadata before processing
- Timeout for long operations

### SEC-7: Send Password Handling
**Risk:** Passwords for Sends could be exposed or logged.

**Mitigation:**
- Use `dialoguer` crate for secure password input (hidden)
- Don't echo passwords to console
- Clear password from memory after use
- Support `--password` flag with environment variable for scripting

## UI/UX Considerations

### UX-1: Password Generation Feedback
**Experience:** User wants to know password strength.

**Approach:**
- Consider adding estimated entropy or strength indicator (optional)
- Keep output clean by default (just the password)
- Use `--response` for JSON with metadata

### UX-2: Send Creation Success
**Experience:** User needs to know Send was created and how to share it.

**Approach:**
- Return Send URL prominently
- Show expiration date if set
- Show access count limit if set
- Clear success message

### UX-3: Receive Password Prompt
**Experience:** User accessing password-protected Send needs to enter password.

**Approach:**
- Prompt interactively with hidden input
- Support `--password` flag for non-interactive use
- Support `--nointeraction` to fail if password needed
- Clear error if password wrong

### UX-4: Progress Indicators
**Experience:** Large file Send uploads need progress feedback.

**Approach:**
- Use `indicatif` crate for progress bar
- Show percentage and estimated time remaining
- Respect `--quiet` flag (no progress in quiet mode)
- Only show progress for operations >2 seconds

### UX-5: Error Messages
**Experience:** Users need clear guidance when commands fail.

**Approach:**
- Specific error messages, not generic "operation failed"
- Suggest fixes (e.g., "Not authenticated. Run 'bw login' or use --session")
- Include relevant context (e.g., "Send expired on 2025-11-30")
- Respect `--quiet` flag

### UX-6: Scripting Support
**Experience:** Users scripting with CLI need parseable output.

**Approach:**
- Support `--response` for JSON output on all commands
- Return plain values by default (no decorative text)
- Exit codes: 0 for success, 1 for failure (unless --cleanexit)
- `--nointeraction` prevents prompts that block scripts

## Testing Strategy

### Unit Tests
**Scope:** Test individual functions and logic in isolation.

**Coverage:**
- Password generation algorithm (character sets, length, minimums)
- Passphrase generation (word selection, separator, capitalization)
- Character set constraint validation
- Base64 encoding correctness
- Send encryption/decryption (with mock keys)
- Input validation (bounds checking, invalid inputs)
- Error handling (specific error types)

**Framework:** Standard Rust `#[test]` and `#[tokio::test]`

**Goals:**
- >80% code coverage
- All edge cases tested
- All error paths tested

### Integration Tests
**Scope:** Test full command execution end-to-end.

**Coverage:**
- CLI argument parsing for all commands
- Global flag behavior (--quiet, --response, --session)
- Output formatting (plain text, JSON)
- Send CRUD operations with live API (test account)
- Receive with various Send configurations
- Authentication checks (with and without --session)
- Error scenarios (expired Sends, invalid inputs)

**Requirements:**
- Test Bitwarden account with API access
- Network access to API (or mocked API server)
- Temporary file storage for file Send tests

**Goals:**
- All user stories have integration tests
- All acceptance criteria validated
- Cross-command workflows tested (create then receive)

### Compatibility Tests
**Scope:** Verify output matches TypeScript CLI.

**Approach:**
- Run same command on both TypeScript and Rust CLIs
- Compare outputs (accounting for timestamps, IDs)
- Verify JSON structure matches
- Verify error messages are similar

**Coverage:**
- Generate with various options
- Send operations (create, list, get, delete)
- Receive from Sends created by TypeScript CLI
- Encode command

**Goals:**
- 100% output compatibility for stable features
- No breaking changes in behavior

### Security Tests
**Scope:** Verify cryptographic security and no data leaks.

**Coverage:**
- Verify OsRng usage in password generation
- Test memory clearing with memory profiler
- Verify no sensitive data in logs
- Fuzz testing for input validation
- Cross-validate Send encryption with web vault

**Tools:**
- `cargo-audit` for dependency vulnerabilities
- Memory profiler (e.g., `valgrind`, `heaptrack`)
- Fuzzing with `cargo-fuzz`
- Manual security review

**Goals:**
- No critical security issues
- All sensitive data zeroized
- Fuzzing finds no crashes or panics

### Performance Tests
**Scope:** Validate performance meets targets.

**Coverage:**
- Password generation speed (100,000 iterations)
- Passphrase generation speed (100,000 iterations)
- Send create/list/get/delete latency
- Receive latency
- File Send with various sizes (1MB, 10MB, 100MB if supported)

**Metrics:**
- P50, P95, P99 latency
- Memory usage
- CPU usage

**Goals:**
- Generate: <100ms (P95)
- Send operations: <2s (P95)
- File Send: reasonable based on size and bandwidth
- Memory usage: <10MB for file operations

## Risk Assessment

### High Risks

**R-1: SDK Encryption APIs Unavailable**
- **Impact:** High - Cannot implement Send without encryption
- **Probability:** Medium - SDK is currently mocked
- **Mitigation:** Research SDK early; plan custom implementation if needed
- **Owner:** Architect

**R-2: API Compatibility Issues**
- **Impact:** High - Sends won't work with web vault
- **Probability:** Low - API is stable and documented
- **Mitigation:** Test with live API early; cross-validate with web vault
- **Owner:** Implementer

### Medium Risks

**R-3: Performance Targets Not Met**
- **Impact:** Medium - Poor user experience but functional
- **Probability:** Low - Operations are straightforward
- **Mitigation:** Profile early; optimize hot paths
- **Owner:** Implementer

**R-4: File Send Complexity**
- **Impact:** Medium - May need to defer file Send to later release
- **Probability:** Medium - Streaming adds complexity
- **Mitigation:** Mark file Send as optional; implement after text Send working
- **Owner:** Architect

### Low Risks

**R-5: Word List Availability**
- **Impact:** Low - Can embed word list if needed
- **Probability:** Low - EFF word list is freely available
- **Mitigation:** Embed word list in binary as fallback
- **Owner:** Implementer

**R-6: Cross-Platform Issues**
- **Impact:** Low - Minor platform-specific bugs
- **Probability:** Low - Using cross-platform libraries
- **Mitigation:** Test on all platforms; use CI for multiple OS
- **Owner:** Implementer

## References

### Bitwarden Documentation
- [Bitwarden CLI Documentation](https://bitwarden.com/help/cli/)
- [Send Documentation](https://bitwarden.com/help/send-basics/)
- Bitwarden API documentation (internal)

### TypeScript CLI Source
- `apps/cli/src/tools/commands/generate.command.ts` - Password generation
- `apps/cli/src/tools/send/*.ts` - Send commands
- `apps/cli/src/tools/commands/receive.command.ts` - Receive command
- `apps/cli/src/tools/commands/encode.command.ts` - Encode command

### External Resources
- [EFF Long Wordlist](https://www.eff.org/deeplinks/2016/07/new-wordlists-random-passphrases) - For passphrase generation
- [OWASP Password Generation Guidelines](https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html)
- [Rust rand crate documentation](https://docs.rs/rand/)
- [Rust base64 crate documentation](https://docs.rs/base64/)

### Internal Documentation
- Enhancement 02: Storage Layer - For Send key storage
- Enhancement 03: API Client - For Send endpoints
- Enhancement 04: Authentication - For session management

## Handoff Notes for Architecture Team

### Key Decisions Needed
1. **SDK vs. Direct Implementation:** Determine if SDK provides Send encryption and password generation, or if we implement directly
2. **Receive Command Location:** Confirm receive is top-level command (not under send)
3. **Send Model Structure:** Design Send models matching API and compatible with encryption
4. **File Send Strategy:** Design streaming approach for large files, or defer to later release
5. **Word List Strategy:** Choose word list source and storage method

### Critical Path Items
1. Research SDK capabilities for Send encryption (blocks Phase 3)
2. Define Send models (blocks Phase 3-5)
3. Add Send endpoints to API client (blocks Phase 4)
4. Design receive command structure (blocks Phase 5)

### Areas Requiring Specialist Input
- Cryptography: Send encryption format must match web vault
- API Design: Send endpoints must match existing API contract
- Performance: File Send streaming implementation
- Security: Review of all crypto and random generation

### Success Metrics for Next Phase
- Architecture document defines all models and interfaces
- SDK integration plan clear (use SDK vs. implement)
- File strategy decided (MVP or deferred)
- All open questions resolved

## Appendix: Command Reference

### Generate Command

```bash
# Basic usage
bw generate                          # 14-char password, default sets
bw generate --length 20              # 20-char password
bw generate --passphrase             # 3-word passphrase
bw generate --passphrase --words 5   # 5-word passphrase

# Password options
bw generate --length 16 --uppercase 2 --lowercase 2 --number 2 --special 2

# Passphrase options
bw generate --passphrase --words 4 --separator _ --capitalize --include-number
```

### Send Commands

```bash
# Create text Send
bw send create --text "Secret message" --session <key>
bw send create '{"name":"Test","text":{"text":"Content"},"deletionDate":"2025-12-31"}'

# List Sends
bw send list --session <key>

# Get Send
bw send get <send-id> --session <key>

# Delete Send
bw send delete <send-id> --session <key>

# Template
bw send template
bw send template file

# Edit Send (optional)
bw send edit <send-id> '{"name":"New Name"}'

# Remove password (optional)
bw send remove-password <send-id>

# Create file Send (optional)
bw send create --file /path/to/file --session <key>
```

### Receive Command

```bash
# Receive public Send
bw receive https://send.bitwarden.com/#/access/ABC123

# Receive with password
bw receive <url> --password "secret"

# Non-interactive (fail if password needed)
bw receive <url> --nointeraction
```

### Encode Command

```bash
# Encode text
bw encode "Hello, World!"

# Encode with JSON output
bw encode "test" --response
```

---

**End of Requirements Analysis**

*This document provides a comprehensive requirements analysis for the Tool Commands enhancement. The architecture team should use this as the foundation for designing the implementation approach, data models, and technical specifications.*
