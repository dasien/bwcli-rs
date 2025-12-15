# Technical Investigation Notes: SDK Migration - Vault

## Codebase Analysis

### Current Implementation Summary

The CLI vault implementation consists of approximately 3,262 lines across services and models:

**Services (`crates/bw-core/src/services/vault/`)**:
| File | Lines | Purpose |
|------|-------|---------|
| `cipher_service.rs` | 419 | Manual field-by-field encrypt/decrypt |
| `write_service.rs` | 616 | CRUD operations with cache management |
| `sync_service.rs` | ~161 | Sync from API to storage |
| `search_service.rs` | ~154 | Filter/search operations |
| `validation_service.rs` | ~300 | Input validation |
| `totp_service.rs` | 111 | TOTP generation (already uses SDK) |
| `confirmation_service.rs` | ~50 | User prompts |
| `errors.rs` | ~50 | Error types |
| `mod.rs` | 332 | Main VaultService coordinator |

**Models (`crates/bw-core/src/models/vault/`)**:
| File | Lines | Purpose |
|------|-------|---------|
| `cipher.rs` | 613 | Cipher/CipherView + subtypes |
| `folder.rs` | 27 | Folder/FolderView types |
| `collection.rs` | 39 | Collection types |
| `cipher_request.rs` | ~150 | API request types |

### Key Patterns Observed

#### 1. Key Management Pattern
The current implementation retrieves the user key and passes it explicitly:

```rust
// From VaultService::list_items
let user_key = self.get_user_key(session).await?;
let ciphers = self.get_ciphers().await?;
let filtered = self.search_service.filter_ciphers(&ciphers, filters);
self.cipher_service
    .decrypt_ciphers(&filtered.into_values().collect::<Vec<_>>(), &user_key)
    .await
```

The `KeyService` derives the user key from the session token.

#### 2. Encryption Pattern
`CipherService` uses `bitwarden_crypto` directly:

```rust
// Current encryption approach
fn encrypt_string(&self, plain_text: &str, key: &SymmetricCryptoKey) -> Result<String, VaultError> {
    let enc_string: EncString = plain_text
        .encrypt_with_key(key)
        .map_err(|e| VaultError::EncryptionError(format!("Encryption failed: {}", e)))?;
    Ok(enc_string.to_string())
}
```

#### 3. Decryption Pattern
Similar direct use of `bitwarden_crypto`:

```rust
// Current decryption approach
fn decrypt_string(&self, enc_string: &str, key: &SymmetricCryptoKey) -> Result<String, VaultError> {
    let enc: EncString = enc_string.parse()
        .map_err(|e| VaultError::DecryptionError(format!("Invalid EncString format: {}", e)))?;
    enc.decrypt_with_key(key)
        .map_err(|e| VaultError::DecryptionError(format!("Decryption failed: {}", e)))
}
```

#### 4. TOTP Service (SDK Reference Pattern)
The TOTP service already uses the SDK and provides a working pattern:

```rust
use bitwarden_vault::generate_totp;

pub async fn generate_code(&self, totp_secret: &str) -> Result<String, VaultError> {
    let response = generate_totp(totp_secret.to_string(), None)
        .map_err(|e| VaultError::TotpError(e.to_string()))?;
    Ok(response.code)
}
```

This shows that SDK functions can be called directly without complex client initialization for stateless operations.

### Storage Format Analysis

Current storage uses `HashMap<String, Cipher>` with custom `Cipher` type serialized to JSON.

**Key serde attributes on CLI Cipher**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cipher {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_id: Option<String>,
    pub folder_id: Option<String>,
    #[serde(rename = "type")]
    pub cipher_type: CipherType,
    // ... etc
}
```

**CLI CipherType enum**:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CipherType {
    Login = 1,
    SecureNote = 2,
    Card = 3,
    Identity = 4,
    SshKey = 5,
}
```
Custom serde implementation serializes as u8 integers.

### SDK Dependencies Already Present

From `bw-core/Cargo.toml`:
```toml
bitwarden-core = { workspace = true, features = ["internal"] }
bitwarden-crypto.workspace = true
bitwarden-auth.workspace = true
bitwarden-vault.workspace = true
bitwarden-generators.workspace = true
bitwarden-encoding.workspace = true
bitwarden-error.workspace = true
```

The SDK crates are already integrated into the workspace.

## CLI Command Analysis

### Commands Affected by Migration

**Read Operations** (use decryption):
- `bw list items` - Lists all vault items
- `bw list folders` - Lists all folders
- `bw list collections` - Lists all collections
- `bw get item <id>` - Get specific item
- `bw get username <id>` - Get username field
- `bw get password <id>` - Get password field
- `bw get uri <id>` - Get URI field
- `bw get totp <id>` - Get TOTP code (already uses SDK)
- `bw get folder <id>` - Get specific folder

**Write Operations** (use encryption):
- `bw create item` - Create new vault item
- `bw create folder` - Create new folder
- `bw edit item <id>` - Edit existing item
- `bw edit folder <id>` - Edit existing folder
- `bw move <item> <folder>` - Move item to folder

**Neutral Operations** (no crypto):
- `bw delete item <id>` - Delete item
- `bw restore <id>` - Restore from trash
- `bw sync` - Sync vault (handles encrypted data as-is)

## Potential Migration Approaches

### Approach A: Direct SDK Type Usage

Replace CLI types with SDK re-exports:

```rust
// Before
use crate::models::vault::{Cipher, CipherView};

// After
use bitwarden_vault::{Cipher, CipherView};
```

**Pros**:
- Cleanest solution
- Automatic feature parity with SDK updates
- No adapter maintenance

**Cons**:
- May require storage migration if JSON differs
- Breaking change if external code depends on type details
- Less control over serialization format

### Approach B: Adapter Layer

Keep CLI types for storage, convert at SDK boundaries:

```rust
impl From<CliCipher> for SdkCipher { ... }
impl From<SdkCipherView> for CliCipherView { ... }
```

**Pros**:
- No storage format changes
- Gradual migration possible
- Preserves existing external interface

**Cons**:
- More code to maintain
- Performance overhead from conversions
- Must keep adapter in sync with SDK changes

### Approach C: Hybrid (Recommended)

Use SDK types internally, CLI types at external boundaries:

```rust
// Internal service layer
fn decrypt_internal(&self, cipher: &SdkCipher) -> Result<SdkCipherView, Error>

// External API layer
pub async fn get_item(&self, id: &str) -> Result<CliCipherView, Error> {
    let sdk_cipher = self.storage_to_sdk(&stored)?;
    let sdk_view = self.decrypt_internal(&sdk_cipher)?;
    Ok(self.sdk_to_cli_view(sdk_view))
}
```

This approach:
- Uses SDK for crypto (security benefit)
- Maintains CLI interface stability
- Allows gradual internal migration
- Can remove adapters later if storage format aligns

## Key Questions for Architecture Phase

1. **SDK Client Lifecycle**
   - Should `Client` be created once and stored in `AppContext`?
   - How to handle re-initialization after unlock?
   - Thread-safety considerations with `Arc<Client>`?

2. **Key Store Population**
   - How does `Client::internal.initialize_user_crypto()` work?
   - What parameters does it need from our session key?
   - Is there a simpler initialization path for CLI use case?

3. **Serialization Verification**
   - Need to compare actual JSON output of SDK types vs CLI types
   - Create test fixtures with sample ciphers
   - Verify all field names and null handling

4. **Error Type Mapping**
   - SDK uses `bitwarden_vault::VaultLocked`, `bitwarden_crypto::CryptoError`, etc.
   - Map to existing `VaultError` variants or create new ones?

5. **Async Compatibility**
   - Verify SDK methods' async characteristics
   - Current code is fully async - ensure compatibility

## Risk Analysis Details

### High Risk: Key Store Initialization

The SDK's key management is more sophisticated than direct key passing. The CLI currently does:

```rust
cipher.decrypt_with_key(&user_key)
```

The SDK expects:
```rust
client.vault().ciphers().decrypt(cipher)
// Key is retrieved from internal key store
```

**Mitigation**: Study SDK test code for initialization patterns. The SDK test utilities may provide a simpler path.

### Medium Risk: Storage Format

If SDK types serialize differently, existing vault data would be unreadable after upgrade.

**Mitigation**:
1. Create test that serializes same cipher with both type systems
2. Compare JSON byte-for-byte
3. If different, implement adapter layer (Approach B/C)

### Low Risk: Performance

SDK may add overhead from key store lookups vs direct key passing.

**Mitigation**:
1. Benchmark before migration
2. Benchmark after migration
3. Profile if degradation observed
4. SDK likely optimizes for common paths

## Existing Test Coverage

The TOTP service has tests that demonstrate SDK integration patterns:

```rust
#[tokio::test]
async fn test_generate_totp_base32_secret() {
    let service = TotpService::new();
    let result = service.generate_code("JBSWY3DPEHPK3PXP").await;
    assert!(result.is_ok());
    let code = result.unwrap();
    assert_eq!(code.len(), 6);
}
```

Similar test patterns should be created for cipher operations:

```rust
#[tokio::test]
async fn test_decrypt_cipher_via_sdk() {
    // Setup SDK client with test key
    // Create encrypted cipher
    // Decrypt via SDK
    // Verify all fields match expected values
}
```

## Next Steps for Architecture

1. **Prototype SDK Client initialization** with user key from session
2. **Create serialization comparison tests** for Cipher/Folder types
3. **Design adapter layer interface** if needed
4. **Define error mapping strategy**
5. **Plan phased migration** starting with read operations
