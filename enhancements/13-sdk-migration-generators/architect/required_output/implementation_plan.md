---
enhancement: 13-sdk-migration-generators
agent: architect
task_id: task_1765575064_68682
timestamp: 2025-12-12T00:15:00Z
status: READY_FOR_IMPLEMENTATION
---

# Implementation Plan: SDK Migration - Password/Passphrase Generators

## Executive Summary

This plan details the migration from the custom `bw-core::services::generator` module to the SDK's `bitwarden-generators` crate. The implementation is straightforward: update the `execute_generate()` function in `tools.rs` to use SDK types directly, then remove the now-unused custom generator module.

**Estimated Complexity**: Low - Direct type mapping with no architectural changes needed.

## Architecture Overview

### Current Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         bw-cli                                   │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │ commands/tools.rs                                          │  │
│  │   execute_generate() ─────────────────────────────────────┼──┼───┐
│  └───────────────────────────────────────────────────────────┘  │   │
└─────────────────────────────────────────────────────────────────┘   │
                                                                       │
┌─────────────────────────────────────────────────────────────────┐   │
│                         bw-core                                  │   │
│  ┌───────────────────────────────────────────────────────────┐  │   │
│  │ services/generator/ (TO BE REMOVED)                        │◄─┼───┘
│  │   ├── mod.rs                                               │  │
│  │   ├── password.rs      (~165 lines)                        │  │
│  │   ├── passphrase.rs    (~190 lines)                        │  │
│  │   ├── errors.rs        (~25 lines)                         │  │
│  │   ├── wordlist.rs      (~25 lines)                         │  │
│  │   └── eff_large_wordlist.txt                               │  │
│  └───────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

### Target Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         bw-cli                                   │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │ commands/tools.rs                                          │  │
│  │   execute_generate() ─────────────────────────────────────┼──┼───┐
│  └───────────────────────────────────────────────────────────┘  │   │
└─────────────────────────────────────────────────────────────────┘   │
                                                                       │
┌─────────────────────────────────────────────────────────────────┐   │
│                  bitwarden-generators (SDK)                      │◄──┘
│  ┌───────────────────────────────────────────────────────────┐  │
│  │ password()           → PasswordGeneratorRequest            │  │
│  │ passphrase()         → PassphraseGeneratorRequest          │  │
│  │ PasswordError        → Error variants                      │  │
│  │ PassphraseError      → Error variants                      │  │
│  └───────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

## Technical Design

### 1. SDK Type Mapping

#### Password Generation

| CLI Argument | Current Type (`PasswordOptions`) | SDK Type (`PasswordGeneratorRequest`) |
|--------------|----------------------------------|--------------------------------------|
| `--length` | `length: usize` | `length: u8` |
| `--lowercase` | `min_lowercase: usize` + `include_lowercase: bool` | `lowercase: bool` + `min_lowercase: Option<u8>` |
| `--uppercase` | `min_uppercase: usize` + `include_uppercase: bool` | `uppercase: bool` + `min_uppercase: Option<u8>` |
| `--number` | `min_numbers: usize` + `include_numbers: bool` | `numbers: bool` + `min_number: Option<u8>` |
| `--special` | `min_special: usize` + `include_special: bool` | `special: bool` + `min_special: Option<u8>` |
| N/A | `exclude_chars: Option<String>` | `avoid_ambiguous: bool` |

**Key Differences:**
- SDK uses `u8` instead of `usize` for length and minimums
- SDK default length is 16 (current is 14)
- SDK default for `special` is `false` (current is `true`)
- SDK has `avoid_ambiguous` (not exposed in current CLI)

#### Passphrase Generation

| CLI Argument | Current Type (`PassphraseOptions`) | SDK Type (`PassphraseGeneratorRequest`) |
|--------------|-----------------------------------|----------------------------------------|
| `--words` | `num_words: usize` | `num_words: u8` |
| `--separator` | `separator: String` | `word_separator: String` |
| `--capitalize` | `capitalize: bool` | `capitalize: bool` |
| `--includeNumber` | `include_number: bool` | `include_number: bool` |

**Key Differences:**
- SDK uses `u8` instead of `usize` for word count
- SDK default separator is space `" "` (current is hyphen `"-"`)
- SDK appends number to random word; current appends at end

### 2. API Design

The updated `execute_generate()` function will use SDK types directly without an intermediate abstraction layer. This follows the recommendation from the requirements analysis.

```rust
// Pseudocode for updated execute_generate()
pub async fn execute_generate(
    cmd: GenerateCommand,
    global_args: &GlobalArgs,
    _ctx: &AppContext,
) -> anyhow::Result<Response> {
    use bitwarden_generators::{
        password, passphrase,
        PasswordGeneratorRequest, PassphraseGeneratorRequest,
        PasswordError, PassphraseError,
    };

    // RNG Explanation: The SDK uses thread_rng() (ChaCha12 seeded from OsRng)
    // which is cryptographically secure. This differs from our previous direct
    // OsRng usage but provides equivalent security guarantees.
    // See: https://docs.rs/rand/latest/rand/rngs/struct.ThreadRng.html

    if cmd.passphrase {
        let request = PassphraseGeneratorRequest { /* mapped fields */ };
        let result = passphrase(request)?;
        // format response
    } else {
        let request = PasswordGeneratorRequest { /* mapped fields */ };
        let result = password(request)?;
        // format response
    }
}
```

### 3. Error Mapping Strategy

Map SDK errors to user-friendly CLI messages:

| SDK Error | CLI Message |
|-----------|-------------|
| `PasswordError::NoCharacterSetEnabled` | "No character sets enabled. Enable at least one of: --lowercase, --uppercase, --number, --special" |
| `PasswordError::InvalidLength` | "Invalid password length. Length must be between 4 and 128 characters, and greater than the sum of all minimums" |
| `PassphraseError::InvalidNumWords { minimum, maximum }` | "Invalid word count. Number of words must be between {minimum} and {maximum}" |

Implementation approach: Use `anyhow::Context` to add user-friendly context to SDK errors:

```rust
let result = password(request)
    .map_err(|e| match e {
        PasswordError::NoCharacterSetEnabled => anyhow::anyhow!(
            "No character sets enabled. Enable at least one of: lowercase, uppercase, numbers, or special characters"
        ),
        PasswordError::InvalidLength => anyhow::anyhow!(
            "Invalid password length. Length must be at least 4 and greater than the sum of minimum character requirements"
        ),
    })?;
```

### 4. Default Value Changes

The following defaults will change as part of this migration:

| Setting | Current Default | New Default (SDK) | Impact |
|---------|-----------------|-------------------|--------|
| Password length | 14 | 16 | Stronger passwords by default |
| Special characters | Enabled | Disabled | **Breaking**: Users expecting special chars must use `--special` flag |
| Minimum numbers | 1 | 1 | No change |
| Minimum special | 1 | N/A (disabled) | N/A since special is disabled |

**Recommendation**: To maintain backward compatibility with the current CLI behavior for special characters, explicitly set `special: true` in the SDK request when no character set options are specified. This preserves the existing behavior where special characters are included by default.

### 5. File Changes

#### Files to Modify

**`crates/bw-cli/src/commands/tools.rs`** (lines 101-168)

Changes required:
1. Update imports to use SDK types
2. Add RNG explanation comment
3. Replace `PasswordOptions` construction with `PasswordGeneratorRequest`
4. Replace `PassphraseOptions` construction with `PassphraseGeneratorRequest`
5. Update error handling to map SDK errors
6. Update default length comment from 14 to 16

```rust
// Before (line 106-108)
use bw_core::services::generator::{
    PassphraseOptions, PasswordOptions, generate_passphrase, generate_password,
};

// After
use bitwarden_generators::{
    passphrase, password,
    PassphraseGeneratorRequest, PasswordGeneratorRequest,
    PasswordError, PassphraseError,
};
```

**`crates/bw-core/src/services/mod.rs`** (line 19)

Change required:
```rust
// Remove this line
pub mod generator;
```

#### Files to Delete

```
crates/bw-core/src/services/generator/
├── mod.rs           (~10 lines)
├── password.rs      (~280 lines with tests)
├── passphrase.rs    (~205 lines with tests)
├── errors.rs        (~25 lines)
├── wordlist.rs      (~25 lines)
└── eff_large_wordlist.txt  (~7776 lines)
```

**Total code removed**: ~550 lines of Rust code + ~7776 lines wordlist

## Implementation Steps

### Step 1: Update `tools.rs` to Use SDK Types

1. Change import statement from `bw_core::services::generator` to `bitwarden_generators`
2. Add RNG explanation comment at top of `execute_generate()` function
3. Update password generation logic:
   - Construct `PasswordGeneratorRequest` from CLI arguments
   - Call `password(request)` instead of `generate_password(&options)`
   - Map SDK errors to user-friendly messages
4. Update passphrase generation logic:
   - Construct `PassphraseGeneratorRequest` from CLI arguments
   - Call `passphrase(request)` instead of `generate_passphrase(&options)`
   - Map SDK errors to user-friendly messages
5. Update `GenerateCommand` help text for `--length` default (14 → 16)

### Step 2: Add `bitwarden-generators` Dependency to `bw-cli`

The `bw-cli` crate needs direct access to the SDK generator types since we're removing the intermediate `bw-core::services::generator` module.

Add to `crates/bw-cli/Cargo.toml`:
```toml
# Bitwarden SDK
bitwarden-generators.workspace = true
```

### Step 3: Remove Custom Generator Module

1. Delete the entire `crates/bw-core/src/services/generator/` directory
2. Remove `pub mod generator;` from `crates/bw-core/src/services/mod.rs`

### Step 4: Build Verification

```bash
cargo fmt --check
cargo clippy --all-features --all-targets
cargo build --release
cargo test
```

### Step 5: Manual Testing

Test the following scenarios:
- `bw generate` → produces 16-character password with lowercase, uppercase, numbers, and special
- `bw generate --length 20` → produces 20-character password
- `bw generate --number 0 --special 0` → produces password without numbers or special characters
- `bw generate --passphrase` → produces 3-word passphrase with hyphen separator
- `bw generate --passphrase --words 5` → produces 5-word passphrase
- `bw generate --passphrase --separator "."` → produces passphrase with dot separator
- `bw generate --passphrase --capitalize --includeNumber` → produces capitalized passphrase with number

## Detailed Code Changes

### Updated `execute_generate()` Function

```rust
pub async fn execute_generate(
    cmd: GenerateCommand,
    global_args: &GlobalArgs,
    _ctx: &AppContext,
) -> anyhow::Result<Response> {
    use bitwarden_generators::{
        passphrase, password,
        PassphraseGeneratorRequest, PasswordGeneratorRequest,
        PasswordError, PassphraseError,
    };

    // RNG Note: The SDK uses rand::thread_rng() which is a ChaCha12 CSPRNG
    // seeded from OsRng. This is cryptographically secure and equivalent
    // to our previous direct OsRng usage. The thread-local design provides
    // better performance for repeated calls while maintaining security.

    if cmd.passphrase {
        // Generate passphrase using SDK
        let request = PassphraseGeneratorRequest {
            num_words: cmd.words.unwrap_or(3) as u8,
            word_separator: cmd.separator.unwrap_or_else(|| "-".to_string()),
            capitalize: cmd.capitalize,
            include_number: cmd.include_number,
        };

        let result = passphrase(request).map_err(|e| match e {
            PassphraseError::InvalidNumWords { minimum, maximum } => {
                anyhow::anyhow!(
                    "Invalid word count. Number of words must be between {} and {}",
                    minimum,
                    maximum
                )
            }
        })?;

        if global_args.response {
            Ok(Response::success_json(serde_json::json!({
                "data": result
            })))
        } else {
            Ok(Response::success_raw(result))
        }
    } else {
        // Generate password using SDK
        //
        // Character set logic:
        // - If minimum is explicitly set to 0, disable that character set
        // - Otherwise, enable the character set with the specified minimum
        // - Default behavior: all character sets enabled with special chars included
        let lowercase_enabled = cmd.lowercase != Some(0);
        let uppercase_enabled = cmd.uppercase != Some(0);
        let numbers_enabled = cmd.number != Some(0);
        let special_enabled = cmd.special != Some(0);

        let request = PasswordGeneratorRequest {
            length: cmd.length.unwrap_or(16) as u8,
            lowercase: lowercase_enabled,
            uppercase: uppercase_enabled,
            numbers: numbers_enabled,
            special: special_enabled,
            avoid_ambiguous: false,
            min_lowercase: cmd.lowercase.filter(|&v| v > 0).map(|v| v as u8),
            min_uppercase: cmd.uppercase.filter(|&v| v > 0).map(|v| v as u8),
            min_number: cmd.number.filter(|&v| v > 0).map(|v| v as u8),
            min_special: cmd.special.filter(|&v| v > 0).map(|v| v as u8),
        };

        let result = password(request).map_err(|e| match e {
            PasswordError::NoCharacterSetEnabled => {
                anyhow::anyhow!(
                    "No character sets enabled. Enable at least one of: lowercase, uppercase, numbers, or special characters"
                )
            }
            PasswordError::InvalidLength => {
                anyhow::anyhow!(
                    "Invalid password length. Length must be at least 4 and greater than the sum of minimum character requirements"
                )
            }
        })?;

        if global_args.response {
            Ok(Response::success_json(serde_json::json!({
                "data": result
            })))
        } else {
            Ok(Response::success_raw(result))
        }
    }
}
```

### Updated `GenerateCommand` Help Text

```rust
#[derive(Args)]
pub struct GenerateCommand {
    /// Generate a passphrase instead of password
    #[arg(long)]
    pub passphrase: bool,

    /// Password length (default: 16)
    #[arg(long)]
    pub length: Option<usize>,

    // ... rest unchanged
}
```

## Testing Strategy

### Unit Tests

The SDK has its own comprehensive test suite. CLI-level tests should verify:

1. **Argument to SDK request mapping** - Verify CLI args correctly translate to SDK types
2. **Error mapping** - Verify SDK errors produce appropriate CLI error messages
3. **Default values** - Verify default password is 16 chars with correct character sets

### Integration Tests

Add integration tests in `crates/bw-cli/tests/` to verify:

```rust
#[test]
fn test_generate_default_password() {
    // Verify default produces 16-char password with all character sets
}

#[test]
fn test_generate_custom_length() {
    // Verify --length flag works
}

#[test]
fn test_generate_exclude_character_sets() {
    // Verify --number 0 --special 0 excludes those sets
}

#[test]
fn test_generate_passphrase_default() {
    // Verify default passphrase has 3 words with hyphen separator
}

#[test]
fn test_generate_passphrase_options() {
    // Verify all passphrase options work
}

#[test]
fn test_generate_json_response() {
    // Verify --response flag produces JSON output
}
```

### Manual Validation Checklist

- [ ] `./target/debug/bw generate` produces 16-character password
- [ ] `./target/debug/bw generate --length 20` produces 20-character password
- [ ] `./target/debug/bw generate --number 0` produces password without numbers
- [ ] `./target/debug/bw generate --special 0` produces password without special chars
- [ ] `./target/debug/bw generate --passphrase` produces 3-word passphrase
- [ ] `./target/debug/bw generate --passphrase --words 5` produces 5-word passphrase
- [ ] `./target/debug/bw generate --passphrase --capitalize` produces capitalized passphrase
- [ ] `./target/debug/bw generate --passphrase --includeNumber` produces passphrase with number in word
- [ ] `./target/debug/bw generate --response` produces JSON output

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| SDK type conversion overflow (usize → u8) | SDK validates length (max 128) and word count (max 20), both fit in u8 |
| Breaking change in default length | Document change in release notes; 16 is more secure than 14 |
| Breaking change in passphrase number placement | Document change in release notes; SDK behavior is consistent with other Bitwarden clients |
| SDK dependency version mismatch | SDK is workspace dependency, already versioned at 1.0.0 |

## Success Criteria

- [ ] `bw generate` uses SDK's `password()` function
- [ ] `bw generate --passphrase` uses SDK's `passphrase()` function
- [ ] Custom generator module completely removed from `bw-core`
- [ ] All existing generate tests pass (with updates for new defaults)
- [ ] RNG explanation comment added
- [ ] Build succeeds with no warnings
- [ ] `cargo clippy` passes with no warnings
- [ ] Default password length is 16 characters
- [ ] CLI help text updated to reflect new defaults

## Appendix: SDK API Reference

### `PasswordGeneratorRequest`

```rust
pub struct PasswordGeneratorRequest {
    pub lowercase: bool,          // Include a-z (default: true)
    pub uppercase: bool,          // Include A-Z (default: true)
    pub numbers: bool,            // Include 0-9 (default: true)
    pub special: bool,            // Include !@#$%^&* (default: false)
    pub length: u8,               // Password length (default: 16)
    pub avoid_ambiguous: bool,    // Exclude I, O, l, 0, 1 (default: false)
    pub min_lowercase: Option<u8>,  // Minimum lowercase (default: None → 1 if enabled)
    pub min_uppercase: Option<u8>,  // Minimum uppercase (default: None → 1 if enabled)
    pub min_number: Option<u8>,     // Minimum numbers (default: None → 1 if enabled)
    pub min_special: Option<u8>,    // Minimum special (default: None → 1 if enabled)
}
```

### `PassphraseGeneratorRequest`

```rust
pub struct PassphraseGeneratorRequest {
    pub num_words: u8,            // Number of words (default: 3, range: 3-20)
    pub word_separator: String,   // Separator between words (default: " ")
    pub capitalize: bool,         // Capitalize first letter (default: false)
    pub include_number: bool,     // Append number to random word (default: false)
}
```

### Error Types

```rust
pub enum PasswordError {
    NoCharacterSetEnabled,  // All character sets disabled
    InvalidLength,          // Length < 4 or < sum of minimums
}

pub enum PassphraseError {
    InvalidNumWords { minimum: u8, maximum: u8 },  // num_words not in 3-20
}
```
