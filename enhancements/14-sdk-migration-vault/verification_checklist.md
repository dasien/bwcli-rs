# SDK Migration - Vault: Verification Checklist

Use this checklist to verify the implementation is correct.

## Pre-Implementation State
Capture current behavior before migration:

### Current Architecture
- [ ] `CipherService` has custom `decrypt_cipher()`, `encrypt_cipher()` methods
- [ ] Uses `SymmetricCryptoKey` directly from `bitwarden_crypto`
- [ ] Manual field-by-field encryption/decryption
- [ ] Does NOT support cipher keys (individual cipher encryption)
- [ ] Does NOT track password history
- [ ] CLI models in `crates/bw-core/src/models/vault/`
- [ ] Services in `crates/bw-core/src/services/vault/`

### Baseline Tests (run before migration)
```bash
# Ensure current functionality works
./target/debug/bw list items --session "$BW_SESSION" | head -5
./target/debug/bw get item <item-id> --session "$BW_SESSION"
./target/debug/bw get password <item-id> --session "$BW_SESSION"
./target/debug/bw get totp <item-id> --session "$BW_SESSION"
```

## Post-Implementation Verification

### 1. Build Verification
- [ ] `cargo build` succeeds
- [ ] No compilation errors related to SDK types
- [ ] Warnings acceptable (dead code warnings may remain during transition)

### 2. SDK Integration Verification

Check that SDK types are being used:
```bash
# Should find SDK imports in vault code
grep -r "bitwarden_vault::" crates/bw-core/src/services/vault/
grep -r "VaultClientExt" crates/bw-core/

# Should find VaultClient usage
grep -r "client.vault()" crates/bw-core/
```

### 3. Decryption Tests

```bash
# List items - names should be decrypted
./target/debug/bw list items --session "$BW_SESSION"
# Expected: JSON array with readable "name" fields

# Get specific item
./target/debug/bw get item <login-id> --session "$BW_SESSION"
# Expected: Full cipher with decrypted username, password, notes

# Get password
./target/debug/bw get password <login-id> --session "$BW_SESSION"
# Expected: Plain text password

# Get username
./target/debug/bw get username <login-id> --session "$BW_SESSION"
# Expected: Plain text username

# Get TOTP (if item has TOTP)
./target/debug/bw get totp <totp-item-id> --session "$BW_SESSION"
# Expected: 6-digit code

# Get notes
./target/debug/bw get notes <item-id> --session "$BW_SESSION"
# Expected: Plain text notes
```

### 4. Encryption Tests

```bash
# Create new login
echo '{"type":1,"name":"SDK Migration Test","login":{"username":"test@example.com","password":"testpass123"}}' | \
  ./target/debug/bw create item --session "$BW_SESSION"
# Expected: Success, returns created item with ID

# Verify the created item
./target/debug/bw get item <new-id> --session "$BW_SESSION"
# Expected: Decrypted item matches input

# Edit existing item
./target/debug/bw get item <id> --session "$BW_SESSION" | \
  jq '.name = "Updated Name"' | \
  ./target/debug/bw edit item <id> --session "$BW_SESSION"
# Expected: Success, name updated
```

### 5. TypeScript CLI Cross-Compatibility Tests

```bash
# Create item with Rust CLI
echo '{"type":1,"name":"Rust Created Item","login":{"username":"rust@test.com"}}' | \
  ./target/debug/bw create item --session "$BW_SESSION"

# Read with TypeScript CLI (must see the item without sync)
npx bw list items --session "$BW_SESSION" | grep "Rust Created Item"
# Expected: Item visible

# Create item with TypeScript CLI
echo '{"type":1,"name":"TS Created Item","login":{"username":"ts@test.com"}}' | \
  npx bw create item --session "$BW_SESSION"

# Read with Rust CLI (must see the item without sync)
./target/debug/bw list items --session "$BW_SESSION" | grep "TS Created Item"
# Expected: Item visible
```

### 6. All Cipher Types

```bash
# Login (type 1)
./target/debug/bw list items --session "$BW_SESSION" | jq '.[] | select(.type == 1) | .name' | head -3

# SecureNote (type 2)
./target/debug/bw list items --session "$BW_SESSION" | jq '.[] | select(.type == 2) | .name' | head -3

# Card (type 3) - if any exist
./target/debug/bw list items --session "$BW_SESSION" | jq '.[] | select(.type == 3) | .name' | head -3

# Identity (type 4) - if any exist
./target/debug/bw list items --session "$BW_SESSION" | jq '.[] | select(.type == 4) | .name' | head -3
```

### 7. Folder Operations

```bash
# List folders
./target/debug/bw list folders --session "$BW_SESSION"
# Expected: Decrypted folder names

# Create folder
./target/debug/bw create folder "Test Folder" --session "$BW_SESSION"
# Expected: Success

# Get folder
./target/debug/bw get folder <folder-id> --session "$BW_SESSION"
# Expected: Decrypted folder
```

### 8. Collection Operations

```bash
# List collections (if user has org access)
./target/debug/bw list collections --session "$BW_SESSION"
# Expected: Decrypted collection names
```

### 9. Error Handling

```bash
# Invalid session
./target/debug/bw list items --session "invalid-session"
# Expected: Error about invalid session

# Non-existent item
./target/debug/bw get item "00000000-0000-0000-0000-000000000000" --session "$BW_SESSION"
# Expected: Item not found error

# Invalid item JSON
echo '{"type":1}' | ./target/debug/bw create item --session "$BW_SESSION"
# Expected: Validation error (missing name)
```

### 10. Code Verification

#### Check SDK Usage
- [ ] `CipherService` uses `CiphersClient::decrypt()` instead of manual decryption
- [ ] `CipherService` uses `CiphersClient::encrypt()` instead of manual encryption
- [ ] `VaultService` creates SDK `Client` and uses `VaultClientExt`
- [ ] Folder operations use `FoldersClient`

#### Check Removed/Reduced Code
- [ ] Custom `decrypt_string()` method removed or deprecated
- [ ] Custom `encrypt_string()` method removed or deprecated
- [ ] Manual field-by-field decryption removed
- [ ] Manual field-by-field encryption removed

### 11. Unit Tests

```bash
# Run vault-specific tests
cargo test --package bw-core vault

# Run all tests
cargo test
```

### 12. Grep Verification

```bash
# Should find SDK imports
grep -r "use bitwarden_vault" crates/bw-core/src/
# Expected: Multiple matches in vault services

# Should NOT find manual EncString decryption in CipherService (or minimal)
grep -r "decrypt_with_key" crates/bw-core/src/services/vault/cipher_service.rs
# Expected: Removed or minimal (only for special cases)

# Should find VaultClient usage
grep -r "VaultClient" crates/bw-core/
# Expected: Matches in vault module
```

## Behavioral Changes Summary

| Operation | Before | After |
|-----------|--------|-------|
| Key management | Direct `SymmetricCryptoKey` | SDK `KeyStoreContext` |
| Cipher decrypt | Manual field-by-field | `CiphersClient::decrypt()` |
| Cipher encrypt | Manual field-by-field | `CiphersClient::encrypt()` |
| Cipher keys | Not supported | Supported (if SDK configured) |
| Password history | Not tracked | Potentially tracked |
| TOTP | Already SDK | Same (no change) |

## Expected Code Reduction

| File | Before | After (estimate) |
|------|--------|------------------|
| `cipher_service.rs` | 419 lines | ~100 lines (wrapper) |
| `models/vault/cipher.rs` | ~800 lines | Removed or re-export |
| Total reduction | - | ~500-1000 lines |

## Sign-off

- [ ] All decryption tests pass
- [ ] All encryption tests pass
- [ ] TypeScript CLI cross-compatibility verified
- [ ] All cipher types work
- [ ] Error handling works
- [ ] Unit tests pass
- [ ] Code uses SDK types
- [ ] Ready for merge
