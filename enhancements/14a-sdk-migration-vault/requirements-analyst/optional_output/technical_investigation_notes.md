# Technical Investigation Notes

## Current State Analysis

### Model File Inventory

| File | Lines | Status | Action |
|------|-------|--------|--------|
| `cipher.rs` | 612 | Custom types duplicating SDK | DELETE |
| `folder.rs` | 27 | Custom types duplicating SDK | DELETE |
| `collection.rs` | 39 | Custom types duplicating SDK | DELETE |
| `cipher_request.rs` | varies | API request format | REVIEW - may need to keep |
| `sync_response.rs` | varies | API response format | REVIEW - may need to keep |
| `organization.rs` | varies | May not have SDK equivalent | REVIEW |
| `validation_error.rs` | varies | CLI-specific errors | KEEP |

### Service File Inventory

| File | Lines | Status | Action |
|------|-------|--------|--------|
| `cipher_service.rs` | 418 | Manual encryption/decryption | SIMPLIFY to ~50 lines |
| `sync_service.rs` | 157 | Sync coordination | UPDATE for SDK types |
| `write_service.rs` | 615 | CRUD operations | UPDATE for SDK types |
| `search_service.rs` | 153 | Filtering logic | UPDATE for SDK types |
| `validation_service.rs` | varies | Input validation | UPDATE for SDK types |
| `confirmation_service.rs` | varies | User prompts | KEEP |
| `totp_service.rs` | varies | TOTP generation | KEEP |
| `errors.rs` | varies | Error types | UPDATE if needed |

## Key Type Differences

### ID Types

**CLI (Current)**:
```rust
pub id: String
```

**SDK (Target)**:
```rust
pub id: Option<CipherId>  // where CipherId: UUID newtype
```

**Migration Strategy**:
- Storage keys: Keep as `String`
- Internal operations: Use SDK types
- Conversion: `cipher.id.map(|id| id.to_string())`

### Date Types

**CLI (Current)**:
```rust
pub revision_date: String  // ISO 8601 string
```

**SDK (Target)**:
```rust
pub revision_date: DateTime<Utc>
```

**Migration Strategy**:
- SDK's serde implementation should serialize to ISO 8601
- Verify compatibility before implementation

### Encrypted Fields

**CLI (Current)**:
```rust
pub name: String  // Raw string containing EncString format
```

**SDK (Target)**:
```rust
pub name: EncString  // Proper typed encrypted string
```

**Migration Strategy**:
- SDK handles serialization automatically
- No explicit conversion needed at field level

## Previous Attempt Analysis (Enhancement 14)

### What Went Wrong

The previous implementation created `SdkVaultBridge` which:
1. Created JSON-based conversion functions between CLI and SDK types
2. Added ~300+ lines instead of removing code
3. Maintained both type systems in parallel
4. Didn't actually migrate - just added another layer

### Anti-Patterns to Avoid

1. **Bridge/Adapter Classes**
   - NO: `SdkVaultBridge`, `TypeConverter`, `CipherAdapter`
   - YES: Direct use of SDK types

2. **JSON Round-Trip Conversion**
   - NO: `cli_cipher_to_sdk_via_json()`
   - YES: Use SDK types from the start

3. **Parallel Type Systems**
   - NO: Keeping both `Cipher` and `bitwarden_vault::Cipher`
   - YES: Use only `bitwarden_vault::Cipher`

4. **Abstraction Layers**
   - NO: Wrapping SDK methods in CLI-specific interfaces
   - YES: Call SDK methods directly

## SDK Type Export Verification Needed

The architect MUST verify these types are publicly exported from SDK:

### bitwarden_vault
- [ ] `Cipher`
- [ ] `CipherView`
- [ ] `CipherListView`
- [ ] `CipherType`
- [ ] `CipherRepromptType`
- [ ] `Login`, `LoginView`
- [ ] `Card`, `CardView`
- [ ] `Identity`, `IdentityView`
- [ ] `SecureNote`, `SecureNoteView`
- [ ] `SshKey`, `SshKeyView`
- [ ] `Field`, `FieldView`, `FieldType`
- [ ] `Attachment`, `AttachmentView`
- [ ] `PasswordHistory`, `PasswordHistoryView`
- [ ] `LoginUri`, `LoginUriView`, `UriMatchType`
- [ ] `Folder`, `FolderView`, `FolderId`
- [ ] `CipherId`
- [ ] `VaultClientExt` trait

### bitwarden_collections
- [ ] `Collection`
- [ ] `CollectionView`
- [ ] `CollectionId`

### bitwarden_core
- [ ] `Client`
- [ ] `OrganizationId`
- [ ] `InitUserCryptoRequest`
- [ ] `InitUserCryptoMethod`

## Crypto Initialization Flow

### Current Flow
```
1. User enters password
2. CLI derives master key using KDF
3. CLI decrypts user key from protected storage
4. user_key passed to every encrypt/decrypt call
```

### Target Flow
```
1. User enters password
2. CLI derives master key using KDF
3. CLI decrypts user key from protected storage
4. CLI calls client.crypto().initialize_user_crypto()
5. SDK internally manages crypto state
6. Encrypt/decrypt calls don't need explicit key
```

### Questions for Architect
1. When exactly should crypto be initialized?
2. Does crypto state persist across API calls?
3. How does crypto state relate to session management?

## Storage Format Compatibility

### Current Storage Format
```json
{
  "uuid-string-1": {
    "id": "uuid-string-1",
    "organizationId": null,
    "folderId": null,
    "type": 1,
    "name": "2.encryptedBase64|...",
    "notes": null,
    "favorite": false,
    "login": {
      "username": "2.encryptedBase64|...",
      "password": "2.encryptedBase64|...",
      "uris": []
    },
    "revisionDate": "2024-01-01T00:00:00Z"
  }
}
```

### SDK Serialization (Expected)
The SDK's serde configuration should produce compatible output. Verification needed:
1. Field names use camelCase
2. Optional fields serialize as null when absent
3. Date fields serialize to ISO 8601
4. EncString fields serialize to "2.base64|..." format

## Line Count Targets

### Deletions
| File | Lines | Running Total |
|------|-------|---------------|
| cipher.rs | -612 | -612 |
| folder.rs | -27 | -639 |
| collection.rs | -39 | -678 |
| cipher_service.rs (partial) | -368 | -1046 |

### Additions (Expected)
| File | Lines | Running Total |
|------|-------|---------------|
| mod.rs re-exports | +20 | -1026 |
| crypto init code | +30 | -996 |
| SDK wrapper methods | +20 | -976 |

**Net Target**: At least -400 lines (conservative estimate: -976 lines)

## Test Impact Analysis

### Tests Likely to Need Updates
- Tests that reference custom CLI cipher types
- Tests that construct cipher objects directly
- Tests that mock CipherService methods

### Tests Likely to Pass Unchanged
- Integration tests using JSON fixtures
- Tests checking CLI output format
- Tests using storage service

## Dependencies to Add

```toml
# workspace Cargo.toml
bitwarden-collections = { path = "../sdk-internal/crates/bitwarden-collections", version = "=1.0.0" }

# bw-core Cargo.toml
bitwarden-collections.workspace = true
```
