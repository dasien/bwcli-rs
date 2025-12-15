# Enhancement 14a - SDK Migration Vault: Grading Checklist

Use this checklist to grade the agents' work. Each item should be verified.

## Pre-Check: Anti-Patterns (Immediate Fail)

**If ANY of these are present, grade is F:**

- [ ] Created a bridge/adapter class (e.g., `SdkVaultBridge`, `VaultAdapter`)
- [ ] Created type conversion functions between CLI and SDK types
- [ ] Added more lines of code than removed (net positive LOC change)
- [ ] Kept both CLI custom types AND SDK types in parallel

## Build Verification

```bash
cargo build 2>&1
```
- [ ] Build succeeds without errors
- [ ] Warnings are acceptable (some dead_code warnings okay during transition)

## Code Deletion Verification

### cipher.rs (should be deleted or gutted)
```bash
wc -l crates/bw-core/src/models/vault/cipher.rs 2>/dev/null || echo "DELETED - GOOD"
```
- [ ] File deleted OR reduced to <50 lines (from 612)

### cipher_service.rs (should be drastically reduced)
```bash
wc -l crates/bw-core/src/services/vault/cipher_service.rs
```
- [ ] Reduced to <100 lines (from 418)

### Total line count check
```bash
wc -l crates/bw-core/src/models/vault/*.rs crates/bw-core/src/services/vault/*.rs
```
- [ ] Total vault code reduced by 400+ lines from baseline (~4,200 lines)

## SDK Type Usage Verification

### Check for SDK imports (should find matches)
```bash
grep -r "use bitwarden_vault::" crates/bw-core/src/
grep -r "bitwarden_vault::Cipher" crates/bw-core/src/
grep -r "bitwarden_vault::CipherView" crates/bw-core/src/
```
- [ ] SDK vault types are imported
- [ ] SDK Cipher type is used (not custom Cipher)
- [ ] SDK CipherView type is used (not custom CipherView)

### Check for VaultClient usage (should find matches)
```bash
grep -r "\.vault()" crates/bw-core/src/
grep -r "ciphers().decrypt" crates/bw-core/src/
grep -r "ciphers().encrypt" crates/bw-core/src/
grep -r "folders().decrypt" crates/bw-core/src/
```
- [ ] `client.vault()` is called
- [ ] `ciphers().decrypt()` is used
- [ ] `ciphers().encrypt()` is used
- [ ] `folders().decrypt()` is used

### Check custom types are GONE (should find NO matches)
```bash
grep -r "struct Cipher" crates/bw-core/src/models/vault/
grep -r "struct CipherView" crates/bw-core/src/models/vault/
grep -r "struct CipherLogin" crates/bw-core/src/models/vault/
```
- [ ] No custom `Cipher` struct defined
- [ ] No custom `CipherView` struct defined
- [ ] No custom `CipherLogin` struct defined

### Check no bridge pattern (should find NO matches)
```bash
grep -r "SdkVaultBridge\|VaultBridge\|CipherBridge" crates/bw-core/src/
grep -r "cli_to_sdk\|sdk_to_cli\|convert_cipher" crates/bw-core/src/
```
- [ ] No bridge classes
- [ ] No conversion functions

## Unit Tests

```bash
cargo test --lib 2>&1 | tail -20
```
- [ ] All tests pass
- [ ] No test failures related to vault operations

## CLI Functional Tests

**Get a fresh session first:**
```bash
./target/debug/bw unlock '<password>'
export BW_SESSION="<session_key>"
```

### List items
```bash
./target/debug/bw list items --session "$BW_SESSION" | jq '.[0].name'
```
- [ ] Returns decrypted item names (not encrypted strings)

### Get specific item
```bash
./target/debug/bw get item "<item-id>" --session "$BW_SESSION" | jq '.name, .login.username'
```
- [ ] Returns decrypted item with all fields

### List folders
```bash
./target/debug/bw list folders --session "$BW_SESSION" | jq '.[].name'
```
- [ ] Returns decrypted folder names

### List collections (if user has any)
```bash
./target/debug/bw list collections --session "$BW_SESSION" | jq '.[].name'
```
- [ ] Returns decrypted collection names (or empty array if none)

### Get password
```bash
./target/debug/bw get password "<login-item-id>" --session "$BW_SESSION"
```
- [ ] Returns plain text password

## Grading Rubric

### Grade A (Excellent)
- All anti-pattern checks pass (no bridges, no conversions)
- Build succeeds
- 400+ lines deleted (net reduction)
- All SDK type checks pass
- All CLI functional tests pass
- All unit tests pass

### Grade B (Good)
- All anti-pattern checks pass
- Build succeeds
- 200-400 lines deleted
- Most SDK type checks pass
- Most CLI functional tests pass
- Unit tests pass

### Grade C (Acceptable)
- No bridge classes created
- Build succeeds
- Some code deleted (net reduction)
- SDK types partially adopted
- Some CLI commands work
- Unit tests mostly pass

### Grade D (Poor)
- Bridge class created but functional
- Build succeeds
- No net code reduction
- Mix of CLI and SDK types
- CLI commands work but through bridges

### Grade F (Fail)
- Bridge/adapter anti-pattern present
- Build fails
- More code added than removed
- Custom types still defined alongside SDK types
- CLI commands broken

## Baseline Measurements (Before Migration)

Run these before agents start to establish baseline:

```bash
# Vault models
wc -l crates/bw-core/src/models/vault/*.rs
# Expected: ~926 lines

# Vault services
wc -l crates/bw-core/src/services/vault/*.rs
# Expected: ~2,341 lines

# Total
# Expected: ~3,267 lines
```

## Post-Migration Targets

- `cipher.rs`: DELETED or <50 lines
- `cipher_service.rs`: <100 lines
- `folder.rs`: <30 lines (just re-exports)
- `collection.rs`: <30 lines (just re-exports)
- **Net reduction: 500+ lines**

## Quick Grade Command

Run this single command to get a quick assessment:

```bash
echo "=== Anti-Pattern Check ===" && \
grep -r "SdkVaultBridge\|VaultBridge\|cli_to_sdk\|sdk_to_cli" crates/bw-core/src/ && echo "FAIL: Bridge detected!" || echo "PASS: No bridges" && \
echo "=== SDK Usage Check ===" && \
grep -c "\.vault()" crates/bw-core/src/services/vault/*.rs && \
echo "=== Line Count ===" && \
wc -l crates/bw-core/src/models/vault/*.rs crates/bw-core/src/services/vault/*.rs | tail -1 && \
echo "=== Build ===" && \
cargo build 2>&1 | tail -3
```
