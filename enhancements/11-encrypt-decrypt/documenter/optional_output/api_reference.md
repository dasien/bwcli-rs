# API Reference: Vault Encryption/Decryption

This document provides a complete API reference for the encryption/decryption functionality added in Enhancement 11.

## Module: `bw_core::services::storage::protected_storage`

Protected storage encryption/decryption using SDK crypto primitives.

### Constants

#### `PROTECTED_PREFIX`
```rust
pub const PROTECTED_PREFIX: &str = "__PROTECTED__";
```
Prefix for protected storage keys (TypeScript CLI compatible).

---

### Error Types

#### `ProtectedStorageError`
```rust
#[derive(Debug, Error)]
pub enum ProtectedStorageError {
    #[error("Invalid session key: {0}")]
    InvalidSessionKey(String),

    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),

    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),

    #[error("Invalid base64 encoding: {0}")]
    InvalidBase64(String),

    #[error("Invalid key format: {0}")]
    InvalidKeyFormat(String),
}
```

---

### Functions

#### `make_protected_key`
```rust
pub fn make_protected_key(key: &str) -> String
```
Create a protected storage key with the standard prefix.

**Arguments:**
- `key` - The base key name (without prefix)

**Returns:**
- Key name with `__PROTECTED__` prefix

**Example:**
```rust
let key = make_protected_key("abc123_user_auto");
assert_eq!(key, "__PROTECTED__abc123_user_auto");
```

---

#### `user_key_protected_storage_key`
```rust
pub fn user_key_protected_storage_key(user_id: &str) -> String
```
Generate the protected storage key suffix for a user's key.

**Arguments:**
- `user_id` - The user's UUID

**Returns:**
- Key suffix in format `{userId}_user_auto`

**Example:**
```rust
let key = user_key_protected_storage_key("abc-123-def");
assert_eq!(key, "abc-123-def_user_auto");
```

---

#### `generate_session_key`
```rust
pub fn generate_session_key() -> SymmetricCryptoKey
```
Generate a new random session key.

Uses the SDK's cryptographically secure key generation (AES-256-CBC-HMAC, 64 bytes).

**Returns:**
- New random `SymmetricCryptoKey`

**Example:**
```rust
let session_key = generate_session_key();
let formatted = format_session_key(&session_key);
// formatted is a base64 string suitable for BW_SESSION
```

---

#### `parse_session_key`
```rust
pub fn parse_session_key(session_str: &str) -> Result<SymmetricCryptoKey, ProtectedStorageError>
```
Parse a session key from base64 string (BW_SESSION format).

**Arguments:**
- `session_str` - Base64-encoded 64-byte session key

**Returns:**
- `Ok(SymmetricCryptoKey)` - Parsed key ready for encryption/decryption
- `Err(InvalidBase64)` - If decoding fails
- `Err(InvalidKeyFormat)` - If wrong length or invalid key bytes

**Example:**
```rust
let session_str = std::env::var("BW_SESSION")?;
let session_key = parse_session_key(&session_str)?;
```

---

#### `format_session_key`
```rust
pub fn format_session_key(key: &SymmetricCryptoKey) -> String
```
Format a session key as base64 for BW_SESSION export.

**Arguments:**
- `key` - The session key to format

**Returns:**
- Base64-encoded 64-byte key string

**Example:**
```rust
let key = generate_session_key();
let session_string = format_session_key(&key);
println!("export BW_SESSION=\"{}\"", session_string);
```

---

#### `encrypt_protected_string`
```rust
pub fn encrypt_protected_string(
    plain: &str,
    key: &SymmetricCryptoKey,
) -> Result<String, ProtectedStorageError>
```
Encrypt a string using a session key.

Uses EncArrayBuffer binary format (not string format) for TypeScript CLI compatibility.

**Arguments:**
- `plain` - Plaintext string to encrypt
- `key` - Session key for encryption

**Returns:**
- `Ok(String)` - Base64-encoded encrypted data
- `Err(EncryptionFailed)` - If SDK encryption fails

---

#### `decrypt_protected_string`
```rust
pub fn decrypt_protected_string(
    encrypted_b64: &str,
    key: &SymmetricCryptoKey,
) -> Result<String, ProtectedStorageError>
```
Decrypt a string using a session key.

Expects base64-encoded EncArrayBuffer binary format.

**Arguments:**
- `encrypted_b64` - Base64-encoded encrypted data
- `key` - Session key for decryption

**Returns:**
- `Ok(String)` - Decrypted plaintext string
- `Err(InvalidBase64)` - If decoding fails
- `Err(DecryptionFailed)` - If SDK decryption fails

---

#### `encrypt_user_key`
```rust
pub fn encrypt_user_key(
    user_key: &SymmetricCryptoKey,
    session_key: &SymmetricCryptoKey,
) -> Result<String, ProtectedStorageError>
```
Encrypt a user key for protected storage.

**Arguments:**
- `user_key` - The user's symmetric key to protect
- `session_key` - Session key for encryption

**Returns:**
- `Ok(String)` - Base64-encoded encrypted user key
- `Err(EncryptionFailed)` - If encryption fails

**Example:**
```rust
let session_key = generate_session_key();
let encrypted = encrypt_user_key(&user_key, &session_key)?;
storage.set(&make_protected_key(&user_key_protected_storage_key(&user_id)), &encrypted).await?;
```

---

#### `decrypt_user_key`
```rust
pub fn decrypt_user_key(
    encrypted_b64: &str,
    session_key: &SymmetricCryptoKey,
) -> Result<SymmetricCryptoKey, ProtectedStorageError>
```
Decrypt a user key from protected storage.

**Arguments:**
- `encrypted_b64` - Base64-encoded encrypted user key
- `session_key` - Session key for decryption

**Returns:**
- `Ok(SymmetricCryptoKey)` - Decrypted user key
- `Err(InvalidBase64)` - If decoding fails
- `Err(DecryptionFailed)` - If decryption fails
- `Err(InvalidKeyFormat)` - If key reconstruction fails

**Example:**
```rust
let session_key = parse_session_key(&session_str)?;
let encrypted = storage.get(&protected_key)?;
let user_key = decrypt_user_key(&encrypted, &session_key)?;
```

---

## Module: `bw_core::services::key_service`

Key service for user key management.

### Error Types

#### `KeyServiceError`
```rust
#[derive(Debug, Error)]
pub enum KeyServiceError {
    #[error("No active user. Run 'bw login' first.")]
    NoActiveUser,

    #[error("User key not found. Run 'bw unlock' first.")]
    UserKeyNotFound,

    #[error("Invalid session key: {0}")]
    InvalidSessionKey(String),

    #[error("Failed to decrypt user key: {0}")]
    DecryptionFailed(String),

    #[error("Storage error: {0}")]
    StorageError(String),
}
```

---

### Struct: `KeyService`

```rust
pub struct KeyService {
    storage: Arc<Mutex<JsonFileStorage>>,
    account_manager: Arc<AccountManager>,
}
```

Service for managing user encryption keys.

---

#### `KeyService::new`
```rust
pub fn new(
    storage: Arc<Mutex<JsonFileStorage>>,
    account_manager: Arc<AccountManager>,
) -> Self
```
Create a new key service.

**Arguments:**
- `storage` - Storage service for reading/writing protected keys
- `account_manager` - Account manager for getting active user

**Example:**
```rust
let storage = Arc::new(Mutex::new(JsonFileStorage::new(None)?));
let account_manager = Arc::new(AccountManager::new(Arc::clone(&storage)));
let key_service = KeyService::new(storage, account_manager);
```

---

#### `KeyService::get_user_key`
```rust
pub async fn get_user_key(
    &self,
    session_str: &str,
) -> Result<SymmetricCryptoKey, KeyServiceError>
```
Get user key from protected storage using session key.

**Arguments:**
- `session_str` - Base64-encoded session key (from BW_SESSION or --session)

**Returns:**
- `Ok(SymmetricCryptoKey)` - User key ready for vault decryption
- `Err(NoActiveUser)` - If not logged in
- `Err(InvalidSessionKey)` - If session format is wrong
- `Err(UserKeyNotFound)` - If protected key not stored
- `Err(DecryptionFailed)` - If decryption fails

**Example:**
```rust
let session = std::env::var("BW_SESSION")?;
let user_key = key_service.get_user_key(&session).await?;
// user_key is now ready for vault decryption
```

---

#### `KeyService::store_user_key`
```rust
pub async fn store_user_key(
    &self,
    user_id: &str,
    user_key: &SymmetricCryptoKey,
    session_key: &SymmetricCryptoKey,
) -> Result<(), KeyServiceError>
```
Store user key in protected storage.

Called during login/unlock after decrypting user key with master key.

**Arguments:**
- `user_id` - User's UUID
- `user_key` - User's symmetric key to store
- `session_key` - Session key to encrypt with

**Returns:**
- `Ok(())` - Key stored successfully
- `Err(EncryptionFailed)` - If encryption fails
- `Err(StorageError)` - If storage write fails

**Example:**
```rust
// After successful login/unlock:
let session_key = generate_session_key();
key_service.store_user_key(&user_id, &user_key, &session_key).await?;
let session_str = format_session_key(&session_key);
println!("export BW_SESSION=\"{}\"", session_str);
```

---

#### `KeyService::has_user_key`
```rust
pub async fn has_user_key(&self) -> Result<bool, KeyServiceError>
```
Check if user key is stored for the active user.

**Returns:**
- `Ok(true)` - Protected key exists for active user
- `Ok(false)` - No protected key or no active user
- `Err(StorageError)` - If storage read fails

**Example:**
```rust
if key_service.has_user_key().await? {
    println!("Vault is unlocked");
} else {
    println!("Vault is locked, run 'bw unlock'");
}
```

---

#### `KeyService::clear_user_key`
```rust
pub async fn clear_user_key(&self, user_id: &str) -> Result<(), KeyServiceError>
```
Clear the user key from protected storage.

Called during lock/logout operations.

**Arguments:**
- `user_id` - User's UUID

**Returns:**
- `Ok(())` - Key cleared successfully
- `Err(StorageError)` - If storage delete fails

**Example:**
```rust
// During lock:
key_service.clear_user_key(&user_id).await?;
println!("Vault locked");
```

---

## Module: `bw_core::services::vault::cipher_service`

Updated cipher service with real SDK decryption.

### Updated Methods

#### `CipherService::decrypt_cipher`
```rust
pub async fn decrypt_cipher(
    &self,
    cipher: &Cipher,
    user_key: &SymmetricCryptoKey,
) -> Result<CipherView, VaultError>
```
Decrypt a single cipher.

**Arguments:**
- `cipher` - Encrypted cipher from storage
- `user_key` - User's symmetric key for decryption

**Returns:**
- `Ok(CipherView)` - Decrypted cipher view
- `Err(DecryptionError)` - If decryption fails

---

#### `CipherService::decrypt_ciphers`
```rust
pub async fn decrypt_ciphers(
    &self,
    ciphers: &[Cipher],
    user_key: &SymmetricCryptoKey,
) -> Result<Vec<CipherView>, VaultError>
```
Decrypt multiple ciphers.

Continues on individual failures (logs warning, skips item).

**Arguments:**
- `ciphers` - List of encrypted ciphers
- `user_key` - User's symmetric key for decryption

**Returns:**
- `Ok(Vec<CipherView>)` - List of successfully decrypted cipher views

---

#### `CipherService::decrypt_folders`
```rust
pub async fn decrypt_folders(
    &self,
    folders: &[Folder],
    user_key: &SymmetricCryptoKey,
) -> Result<Vec<FolderView>, VaultError>
```
Decrypt folders.

**Arguments:**
- `folders` - List of encrypted folders
- `user_key` - User's symmetric key for decryption

**Returns:**
- `Ok(Vec<FolderView>)` - List of decrypted folder views

---

#### `CipherService::decrypt_collections`
```rust
pub async fn decrypt_collections(
    &self,
    collections: &[Collection],
    user_key: &SymmetricCryptoKey,
) -> Result<Vec<CollectionView>, VaultError>
```
Decrypt collections.

**Arguments:**
- `collections` - List of encrypted collections
- `user_key` - User's symmetric key for decryption

**Returns:**
- `Ok(Vec<CollectionView>)` - List of decrypted collection views

---

## Module: `bw_core::services::vault`

Updated vault service with session-based decryption.

### Updated Methods

All vault operations now require a `session` parameter:

#### `VaultService::list_items`
```rust
pub async fn list_items(
    &self,
    filters: &ItemFilters,
    session: &str,
) -> Result<Vec<CipherView>, VaultError>
```
List all items with optional filters.

**Arguments:**
- `filters` - Optional filters (org, collection, folder, search, etc.)
- `session` - Session key string (from BW_SESSION or --session)

**Returns:**
- `Ok(Vec<CipherView>)` - List of decrypted cipher views matching filters
- `Err(DecryptionError)` - If user key retrieval or decryption fails

---

#### `VaultService::get_item`
```rust
pub async fn get_item(
    &self,
    id_or_search: &str,
    session: &str,
) -> Result<CipherView, VaultError>
```
Get specific item by ID or search term.

**Arguments:**
- `id_or_search` - UUID or search string
- `session` - Session key string

**Returns:**
- `Ok(CipherView)` - Decrypted cipher view
- `Err(ItemNotFound)` - If item not found
- `Err(DecryptionError)` - If decryption fails

---

#### `VaultService::get_field`
```rust
pub async fn get_field(
    &self,
    id_or_search: &str,
    field: FieldType,
    session: &str,
) -> Result<String, VaultError>
```
Get specific field from item.

**Arguments:**
- `id_or_search` - UUID or search string
- `field` - Field type (Username, Password, Uri, Notes)
- `session` - Session key string

**Returns:**
- `Ok(String)` - Decrypted field value
- `Err(ItemNotFound)` - If item not found
- `Err(DecryptionError)` - If decryption fails

---

#### `VaultService::get_totp`
```rust
pub async fn get_totp(
    &self,
    id_or_search: &str,
    session: &str,
) -> Result<String, VaultError>
```
Generate TOTP code for item.

**Arguments:**
- `id_or_search` - UUID or search string
- `session` - Session key string

**Returns:**
- `Ok(String)` - 6-digit TOTP code
- `Err(ItemNotFound)` - If item not found
- `Err(NoTotp)` - If item has no TOTP secret
- `Err(DecryptionError)` - If decryption fails

---

#### `VaultService::list_folders`
```rust
pub async fn list_folders(
    &self,
    search: Option<&str>,
    session: &str,
) -> Result<Vec<FolderView>, VaultError>
```
List all folders.

**Arguments:**
- `search` - Optional search filter
- `session` - Session key string

**Returns:**
- `Ok(Vec<FolderView>)` - List of decrypted folder views

---

#### `VaultService::list_collections`
```rust
pub async fn list_collections(
    &self,
    organization_id: Option<&str>,
    search: Option<&str>,
    session: &str,
) -> Result<Vec<CollectionView>, VaultError>
```
List all collections.

**Arguments:**
- `organization_id` - Optional org filter
- `search` - Optional search filter
- `session` - Session key string

**Returns:**
- `Ok(Vec<CollectionView>)` - List of decrypted collection views

---

## Storage Format Specifications

### Protected Storage Key Format
```
__PROTECTED__{userId}_user_auto

Example:
__PROTECTED__a1b2c3d4-e5f6-7890-abcd-ef1234567890_user_auto
```

### Session Key Format (BW_SESSION)
```
Base64-encoded 64 bytes:
- Bytes 0-31: AES-256 encryption key
- Bytes 32-63: HMAC-SHA256 MAC key
```

### EncArrayBuffer Binary Format
```
[1 byte: type][16 bytes: IV][32 bytes: MAC][variable: ciphertext]

Type 2 = AES-256-CBC with HMAC-SHA256
```

### EncString String Format (vault fields)
```
{type}.{base64_iv}|{base64_ciphertext}|{base64_mac}

Example:
2.abc123==|def456==|ghi789==
```
