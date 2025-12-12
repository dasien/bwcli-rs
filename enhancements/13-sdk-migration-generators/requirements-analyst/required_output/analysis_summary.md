---
enhancement: 13-sdk-migration-generators
agent: requirements-analyst
task_id: task_1765574948_66994
timestamp: 2025-12-12T00:00:00Z
status: READY_FOR_ARCHITECTURE
---

# Requirements Analysis Summary: SDK Migration - Password/Passphrase Generators

## Executive Summary

This enhancement replaces the custom password/passphrase generator implementation in `bw-core` with the SDK's `bitwarden-generators` crate. The migration eliminates approximately 500 lines of duplicated code while maintaining CLI interface compatibility with documented behavioral changes.

**Risk Level**: Low - Well-defined scope with clear acceptance criteria and existing SDK dependency.

## Functional Requirements

### FR-1: Password Generation via SDK
**Description**: Replace custom password generation with SDK's `bitwarden_generators::password()` function.

**User Story**: As a CLI user, I want `bw generate` to produce secure passwords using the SDK so that the CLI benefits from shared, tested password generation logic.

**Acceptance Criteria**:
- [ ] `bw generate` produces passwords using SDK's `PasswordGeneratorRequest`
- [ ] Default password length changed from 14 to 16 characters
- [ ] All character set options (lowercase, uppercase, numbers, special) work correctly
- [ ] Setting minimum to 0 disables that character set (e.g., `--number 0` excludes numbers)
- [ ] Custom length via `--length` flag works correctly

### FR-2: Passphrase Generation via SDK
**Description**: Replace custom passphrase generation with SDK's `bitwarden_generators::passphrase()` function.

**User Story**: As a CLI user, I want `bw generate --passphrase` to produce passphrases using the SDK so that passphrase generation is consistent across Bitwarden products.

**Acceptance Criteria**:
- [ ] `bw generate --passphrase` produces passphrases using SDK's `PassphraseGeneratorRequest`
- [ ] Default word count of 3 works correctly
- [ ] Custom word count via `--words` flag works correctly
- [ ] Custom separator via `--separator` flag works correctly
- [ ] Capitalization via `--capitalize` flag works correctly
- [ ] Number inclusion via `--includeNumber` flag works correctly (SDK behavior: number appended to random word)

### FR-3: CLI Argument Mapping
**Description**: Map existing CLI command arguments to SDK request types.

**User Story**: As a CLI user, I want all existing command flags to continue working so that my scripts and workflows are not broken.

**Acceptance Criteria**:
- [ ] All existing `GenerateCommand` fields map to SDK request types
- [ ] Response format unchanged (raw output by default, JSON with `--response`)
- [ ] Error messages remain user-friendly

### FR-4: SDK Error Mapping
**Description**: Map SDK generator errors to CLI-appropriate error responses.

**Acceptance Criteria**:
- [ ] `PasswordError` variants map to meaningful CLI error messages
- [ ] `PassphraseError` variants map to meaningful CLI error messages
- [ ] Error responses maintain JSON format when `--response` flag is set

### FR-5: Code Removal
**Description**: Remove custom generator module from `bw-core` after migration.

**Acceptance Criteria**:
- [ ] `crates/bw-core/src/services/generator/` directory deleted entirely
- [ ] Generator module removed from `services/mod.rs`
- [ ] Build succeeds with no warnings related to unused code

### FR-6: RNG Documentation
**Description**: Add explanatory comment about randomness source choices.

**Acceptance Criteria**:
- [ ] Comment added explaining SDK's use of `thread_rng()` vs direct `OsRng`
- [ ] Comment notes both approaches are cryptographically secure

## Non-Functional Requirements

### NFR-1: Security
- **Requirement**: Generated passwords/passphrases must be cryptographically secure
- **Validation**: SDK's `thread_rng()` uses ChaCha12 seeded from `OsRng` - this is verified secure
- **Status**: Satisfied by SDK implementation

### NFR-2: Performance
- **Requirement**: No perceivable difference in generation speed
- **Validation**: Both implementations use similar algorithmic approaches
- **Status**: Expected to be satisfied (verify during testing)

### NFR-3: Maintainability
- **Requirement**: Single source of truth for generator logic
- **Validation**: Removing ~500 lines of duplicated code
- **Status**: Satisfied by migration

## Behavioral Changes (Breaking)

The following documented behavioral changes will occur:

| Feature | Current CLI | After SDK Migration |
|---------|-------------|---------------------|
| Default password length | 14 characters | 16 characters |
| Passphrase `--includeNumber` | Number appended at end (`word1-word2-word3-1234`) | Number appended to random word (`word14-word2-word3`) |

**Migration Note**: Users relying on exact output format should be aware of these changes.

## Integration Points

### Files to Modify
1. `crates/bw-cli/src/commands/tools.rs` - Update `execute_generate()` function (lines 101-168)
2. `crates/bw-core/src/services/mod.rs` - Remove `pub mod generator;` line (line 19)

### Files to Delete
1. `crates/bw-core/src/services/generator/mod.rs`
2. `crates/bw-core/src/services/generator/password.rs`
3. `crates/bw-core/src/services/generator/passphrase.rs`
4. `crates/bw-core/src/services/generator/wordlist.rs`
5. `crates/bw-core/src/services/generator/errors.rs`
6. `crates/bw-core/src/services/generator/eff_large_wordlist.txt`

### Dependencies
- `bitwarden-generators` crate (already a workspace dependency)
- Types: `PasswordGeneratorRequest`, `PassphraseGeneratorRequest`, `PasswordError`, `PassphraseError`

## Constraints & Limitations

### Technical Constraints
1. SDK's passphrase number behavior differs from current CLI (accepted)
2. SDK uses `thread_rng()` instead of `OsRng` (accepted - both are secure)
3. Cannot inject custom RNG into SDK without upstream changes

### Scope Boundaries
**In Scope**:
- Password generation migration to SDK
- Passphrase generation migration to SDK
- Default length change (14 → 16)
- Code removal
- RNG documentation

**Out of Scope**:
- Custom RNG injection into SDK
- Backward compatibility for passphrase number placement
- Username generation command (should-have, not MVP)
- `--avoid-ambiguous` flag (should-have, not MVP)

## Project Phases

### Phase 1: Core Migration (MVP)
1. Update `tools.rs` to use SDK types
2. Map CLI arguments to SDK request types
3. Map SDK errors to CLI errors
4. Add RNG explanation comment
5. Remove custom generator module

### Phase 2: Optional Enhancements (Post-MVP)
1. Add `--avoid-ambiguous` flag (SDK supports this)
2. Expose username generation command

## Testing Requirements

### Unit Tests
- Verify CLI argument mapping to SDK request types
- Verify error mapping from SDK errors to CLI errors

### Integration Tests
- `bw generate` produces valid 16-character passwords by default
- `bw generate --length 20` produces 20-character password
- `bw generate --passphrase` produces 3-word passphrase
- `bw generate --passphrase --words 5` produces 5-word passphrase
- `bw generate --passphrase --includeNumber` includes number in a word
- `bw generate --number 0 --special 0` excludes numbers and special chars

### Manual Validation
- Compare output format with TypeScript CLI
- Verify password meets requirements (length, character sets)
- Verify passphrase word count and separator

## Success Criteria

### Definition of Done
- [ ] `bw generate` produces password using SDK
- [ ] `bw generate --passphrase` produces passphrase using SDK
- [ ] Custom generator module removed from `bw-core`
- [ ] All existing generate tests pass (updated as needed)
- [ ] RNG explanation comment added
- [ ] Build succeeds with no warnings related to unused code

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| SDK type incompatibility | Low | Medium | SDK types are well-documented; mapping is straightforward |
| Test failures from behavioral changes | Medium | Low | Update tests to reflect new default length and passphrase behavior |
| Unused code warnings | Low | Low | Ensure complete removal of generator module |

## Open Questions

**Resolved**:
1. Accept SDK passphrase behavior (number on random word) ✓
2. Change default password length to 16 ✓
3. Accept `thread_rng()` (add explanatory comment) ✓

**None remaining** - all decisions have been made in the enhancement specification.

## Recommendations for Architecture

1. **Direct SDK Usage**: The SDK's `bitwarden_generators` crate is already a workspace dependency. The architecture should use the SDK types directly in `tools.rs` rather than creating an intermediate abstraction layer.

2. **Error Mapping**: Create simple mapping from `PasswordError`/`PassphraseError` to `anyhow::Error` with user-friendly messages.

3. **Minimal Changes**: The `GenerateCommand` struct in `tools.rs` should remain unchanged; only the `execute_generate()` function body needs modification.

4. **RNG Comment Placement**: The explanatory comment about RNG choices should be placed at the top of the `execute_generate()` function or in a separate documentation section.
