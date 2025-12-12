# SDK Migration - Generators: Verification Checklist

Use this checklist to verify the implementation is correct.

## Pre-Implementation State
Before migration, capture current behavior:

### Password Generation
- [ ] Default length: 14 characters
- [ ] Character sets: lowercase, uppercase, numbers, special
- [ ] Default minimums: 1 number, 1 special
- [ ] Uses `OsRng` for randomness

### Passphrase Generation
- [ ] Default word count: 3
- [ ] Default separator: `-`
- [ ] `--includeNumber` appends number at END as separate element
  - Example: `word1-word2-word3-1234`

## Post-Implementation Verification

### 1. Build Verification
- [ ] `cargo build` succeeds
- [ ] No warnings about unused code in generator module
- [ ] `crates/bw-core/src/services/generator/` directory is deleted

### 2. Password Generation Tests

```bash
# Default password (should be 16 chars now)
./target/debug/bw generate
# Expected: 16-character password with lowercase, uppercase, numbers, special

# Custom length
./target/debug/bw generate --length 20
# Expected: 20-character password

# Numbers only
./target/debug/bw generate --lowercase 0 --uppercase 0 --special 0
# Expected: Password with only numbers

# No special characters
./target/debug/bw generate --special 0
# Expected: Password without special characters

# Minimum requirements
./target/debug/bw generate --lowercase 3 --uppercase 3 --number 3 --special 3
# Expected: Password with at least 3 of each character type
```

### 3. Passphrase Generation Tests

```bash
# Default passphrase
./target/debug/bw generate --passphrase
# Expected: 3 words separated by hyphens

# Custom word count
./target/debug/bw generate --passphrase --words 5
# Expected: 5 words separated by hyphens

# Custom separator
./target/debug/bw generate --passphrase --separator "."
# Expected: Words separated by dots

# Capitalize
./target/debug/bw generate --passphrase --capitalize
# Expected: Each word capitalized (e.g., "Word-Another-Third")

# Include number (BEHAVIORAL CHANGE)
./target/debug/bw generate --passphrase --includeNumber
# Expected: Number appended to ONE of the words (e.g., "word4-another-third")
# NOT: "word-another-third-1234" (old behavior)
```

### 4. Error Handling Tests

```bash
# Invalid length (too short)
./target/debug/bw generate --length 2
# Expected: Error about invalid length

# No character sets enabled
./target/debug/bw generate --lowercase 0 --uppercase 0 --number 0 --special 0
# Expected: Error about no character sets

# Invalid word count
./target/debug/bw generate --passphrase --words 2
# Expected: Error about word count (min 3)

./target/debug/bw generate --passphrase --words 25
# Expected: Error about word count (max 20)
```

### 5. Code Verification

- [ ] `tools.rs` imports from `bitwarden_generators`, not `bw_core::services::generator`
- [ ] RNG explanation comment is present in `tools.rs`
- [ ] Default password length is 16 in the code
- [ ] No references to `bw_core::services::generator` anywhere in codebase
- [ ] `mod generator;` removed from `bw-core/src/services/mod.rs`

### 6. Grep Verification

```bash
# Should return NO results:
grep -r "bw_core::services::generator" crates/

# Should return NO results:
grep -r "generate_password\|generate_passphrase" crates/bw-core/

# Should find imports in tools.rs:
grep -r "bitwarden_generators" crates/bw-cli/
```

## Behavioral Changes Summary

| Feature | Before | After |
|---------|--------|-------|
| Default password length | 14 | 16 |
| Passphrase number placement | End (`word-word-word-1234`) | Random word (`word4-word-word`) |
| RNG | `OsRng` | `thread_rng()` (ChaCha seeded from OsRng) |

## Sign-off

- [ ] All verification tests pass
- [ ] Behavioral changes documented
- [ ] Ready for merge
