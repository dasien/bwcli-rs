---
slug: sdk-migration-generators
status: NEW
created: 2024-12-12
author: Migration Team
priority: medium
---

# Enhancement: SDK Migration - Password/Passphrase Generators

## Overview
**Goal:** Replace the custom password/passphrase generator implementation in `bw-core` with the SDK's `bitwarden-generators` crate to eliminate code duplication and leverage tested, shared code.

**User Story:**
As a developer, I want the CLI to use the SDK's generator implementation so that we have a single source of truth for password generation logic and can benefit from SDK improvements automatically.

## Context & Background
**Current State:**
- CLI has custom generator implementation in `crates/bw-core/src/services/generator/`
- SDK has `bitwarden-generators` crate with equivalent functionality
- Both implementations are functionally similar but have subtle differences
- CLI version uses `OsRng`, SDK uses `thread_rng()` (both secure)
- CLI passphrase appends number at end, SDK appends to random word

**Technical Context:**
- `bitwarden-generators` is already a workspace dependency
- SDK provides `PasswordGeneratorRequest`, `PassphraseGeneratorRequest` types
- SDK also includes username generation (bonus feature)
- CLI currently has ~500 lines of duplicated generator code

**Dependencies:**
- Enhancement: project-bootstrap (complete)
- Enhancement: tool-commands (complete - implements generate command)
- Bitwarden SDK (`bitwarden-generators` crate)

## Requirements

### Functional Requirements
1. Replace custom generator with SDK's `bitwarden-generators` crate
2. Maintain CLI interface compatibility (`bw generate` command)
3. Update default password length from 14 to 16
4. Accept SDK passphrase behavior (number appended to random word)
5. Add explanatory comments about RNG choices

### Non-Functional Requirements
- **Performance:** No perceivable difference in generation speed
- **Security:** Both `OsRng` and `thread_rng()` are cryptographically secure
- **Maintainability:** Single source of truth for generator logic

### Must Have (MVP)
- [ ] Update `tools.rs` to use `bitwarden_generators` types directly
- [ ] Map CLI command args to SDK request types
- [ ] Map SDK errors to CLI errors
- [ ] Remove custom generator module from `bw-core`
- [ ] Update tests to use SDK types
- [ ] Add RNG explanation comment

### Should Have (if time permits)
- [ ] Add `--avoid-ambiguous` flag to CLI (leverages SDK feature)
- [ ] Expose username generation command

### Won't Have (out of scope)
- Custom RNG injection into SDK
- Backward compatibility for passphrase number placement

## Open Questions
None - decisions made:
1. Accept SDK passphrase behavior (number on random word)
2. Change default password length to 16
3. Accept `thread_rng()` (add explanatory comment)

## Constraints & Limitations
**Technical Constraints:**
- SDK's passphrase number behavior differs from current CLI
- SDK uses `thread_rng()` instead of `OsRng` (acceptable)

**Behavioral Changes:**
- Passphrase with `--includeNumber`: `word1-word2-word3-1234` → `word14-word2-word3`
- Default password length: 14 → 16

## Success Criteria
**Definition of Done:**
- [ ] `bw generate` produces password using SDK
- [ ] `bw generate --passphrase` produces passphrase using SDK
- [ ] Custom generator module removed from `bw-core`
- [ ] All existing generate tests pass
- [ ] RNG explanation comment added
- [ ] Build succeeds with no warnings related to unused code

**Acceptance Tests:**
1. `bw generate` produces 16-character password by default
2. `bw generate --length 20` produces 20-character password
3. `bw generate --passphrase` produces 3-word passphrase
4. `bw generate --passphrase --words 5` produces 5-word passphrase
5. `bw generate --passphrase --includeNumber` includes number in a word
6. `bw generate --number 0 --special 0` excludes numbers and special chars

## Security & Safety Considerations
- Both `OsRng` (current) and `thread_rng()` (SDK) are cryptographically secure
- `thread_rng()` uses ChaCha12 seeded from `OsRng`
- Comment explaining the RNG choice should be added for future reference

## Testing Strategy
**Unit Tests:**
- Verify SDK types can be constructed from CLI args
- Verify error mapping works correctly

**Integration Tests:**
- `bw generate` produces valid passwords
- `bw generate --passphrase` produces valid passphrases
- All flag combinations work correctly

**Manual Test Scenarios:**
1. Compare output format with TypeScript CLI
2. Verify password meets requirements (length, character sets)
3. Verify passphrase word count and separator

## References & Research
- SDK password generator: `sdk-internal/crates/bitwarden-generators/src/password.rs`
- SDK passphrase generator: `sdk-internal/crates/bitwarden-generators/src/passphrase.rs`
- Current CLI generator: `bwcli-rs/crates/bw-core/src/services/generator/`

## Notes for Implementer Subagent

### Key SDK Types to Use
```rust
use bitwarden_generators::{
    PasswordGeneratorRequest, PasswordError,
    PassphraseGeneratorRequest, PassphraseError,
};
```

### Password Generation Migration
```rust
// OLD (custom):
use bw_core::services::generator::{PasswordOptions, generate_password};
let options = PasswordOptions { length: 14, ... };
let password = generate_password(&options)?;

// NEW (SDK):
use bitwarden_generators::{PasswordGeneratorRequest, password::password};
let request = PasswordGeneratorRequest {
    lowercase: true,
    uppercase: true,
    numbers: true,
    special: true,
    length: 16,  // Changed from 14 to 16
    avoid_ambiguous: false,
    min_lowercase: None,
    min_uppercase: None,
    min_number: Some(1),
    min_special: Some(1),
};
let password = password(request)?;
```

### Passphrase Generation Migration
```rust
// OLD (custom):
use bw_core::services::generator::{PassphraseOptions, generate_passphrase};
let options = PassphraseOptions { num_words: 3, ... };
let passphrase = generate_passphrase(&options)?;

// NEW (SDK):
use bitwarden_generators::{PassphraseGeneratorRequest, passphrase::passphrase};
let request = PassphraseGeneratorRequest {
    num_words: 3,
    word_separator: "-".to_string(),
    capitalize: false,
    include_number: false,
};
let passphrase = passphrase(request)?;
```

### Files to Modify
1. `crates/bw-cli/src/commands/tools.rs` - Update `execute_generate` function
2. `crates/bw-core/src/services/mod.rs` - Remove `pub mod generator;`

### Files to Delete
1. `crates/bw-core/src/services/generator/mod.rs`
2. `crates/bw-core/src/services/generator/password.rs`
3. `crates/bw-core/src/services/generator/passphrase.rs`
4. `crates/bw-core/src/services/generator/wordlist.rs`
5. `crates/bw-core/src/services/generator/errors.rs`
6. `crates/bw-core/src/services/generator/eff_large_wordlist.txt`

### RNG Comment to Add
```rust
// Note on randomness sources for password generation:
//
// The SDK's bitwarden-generators uses `thread_rng()` which is a ChaCha-based
// CSPRNG seeded from `OsRng`. This is cryptographically secure.
//
// The previous CLI implementation used `OsRng` directly, which sources
// randomness directly from the operating system (e.g., /dev/urandom on Unix,
// BCryptGenRandom on Windows). While both approaches are secure, `OsRng` has
// the advantage of having no internal state that could be compromised if memory
// were somehow leaked. For high-security applications like password generation,
// `OsRng` is the more conservative choice.
//
// If contributing to the SDK, consider adding an option to use `OsRng` directly
// for applications that prefer it.
```

## Notes for Testing Subagent
- Run `bw generate` multiple times, verify 16-char passwords
- Run `bw generate --passphrase --includeNumber` and verify number is IN a word (not appended at end)
- Verify all flag combinations still work
- Compare with TypeScript CLI output format
