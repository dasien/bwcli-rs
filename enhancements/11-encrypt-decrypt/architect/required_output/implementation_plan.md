---
enhancement: 11-encrypt-decrypt
agent: architect
task_id: task_1765335052_98506
timestamp: 2025-12-09T12:30:00Z
status: READY_FOR_IMPLEMENTATION
---

# Implementation Plan: Vault Encryption/Decryption

## Executive Summary

This enhancement implements the critical decryption capability for the Rust CLI, enabling users to view their vault contents (passwords, usernames, notes) instead of encrypted EncString data. The implementation follows the TypeScript CLI's security model using protected storage with session keys.

---

## 1. Architecture Overview

### 1.1 Security Model

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Security Architecture                         │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  User                      CLI                        Storage        │
│  ────                      ───                        ───────        │
│                                                                      │
│  Master Password ─────────► Derive Master Key                        │
│                                     │                                │
│                                     ▼                                │
│                             Decrypt User Key                         │
│                             (from server)                            │
│                                     │                                │
│                                     ▼                                │
│  BW_SESSION ◄──────── Generate Session Key                           │
│  (env var)                          │                                │
│                                     ▼                                │
│                    ┌────────────────────────────────┐                │
│                    │ Encrypt User Key with          │                │
│                    │ Session Key                    │                │
│                    └────────────────────────────────┘                │
│                                     │                                │
│                                     ▼                                │
│                    ┌────────────────────────────────┐                │
│                    │ Store as:                      │                │
│                    │ __PROTECTED__{userId}_user_auto │──────► data.json
│                    └────────────────────────────────┘                │
│                                                                      │
│  ════════════════════════════════════════════════════════════════   │
│                         VAULT OPERATIONS                             │
│  ════════════════════════════════════════════════════════════════   │
│                                                                      │
│  BW_SESSION ─────────────► Parse Session Key                         │
│                                     │                                │
│                                     ▼                                │
│                    ┌────────────────────────────────┐                │
│  data.json ───────►│ Read __PROTECTED__...          │                │
│                    │ Decrypt with Session Key       │                │
│                    └────────────────────────────────┘                │
│                                     │                                │
│                                     ▼                                │
│                              User Key                                │
│                                     │                                │
│                                     ▼                                │
│                    ┌────────────────────────────────┐                │
│                    │ Decrypt Vault Items            │                │
│                    │ (EncString → plaintext)        │                │
│                    └────────────────────────────────┘                │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### 1.2 Component Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│                           CLI Commands                               │
│                    (vault.rs, auth/login.rs)                         │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│                         VaultService                                 │
│                    (vault/mod.rs)                                    │
│                                                                      │
│   ┌───────────────────┐  ┌───────────────────┐                      │
│   │   CipherService   │  │    KeyService     │ ◄── NEW              │
│   │  (decrypt/encrypt)│  │  (user key mgmt)  │                      │
│   └─────────┬─────────┘  └─────────┬─────────┘                      │
│             │                      │                                 │
│             ▼                      ▼                                 │
│   ┌─────────────────────────────────────────┐                       │
│   │         Protected Storage Module         │ ◄── NEW              │
│   │    (protected_storage.rs)                │                       │
│   └─────────────────────────────────────────┘                       │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│                      bitwarden-crypto SDK                            │
│                                                                      │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐     │
│  │ SymmetricCrypto │  │   EncString     │  │ OctetStreamBytes│     │
│  │      Key        │  │                 │  │                 │     │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘     │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 2. Detailed Component Design

### 2.1 Protected Storage Module

**File:** `crates/bw-core/src/services/storage/protected_storage.rs`

**Purpose:** Provides encryption/decryption for sensitive data stored locally using the session key.

**Public API:**

```rust
// Constants
pub const PROTECTED_PREFIX: &str = "__PROTECTED__";

// Key formatting
pub fn make_protected_key(key: &str) -> String;
pub fn user_key_protected_storage_key(user_id: &str) -> String;

// Session key operations
pub fn parse_session_key(session_str: &str) -> Result<SymmetricCryptoKey, ProtectedStorageError>;
pub fn format_session_key(key: &SymmetricCryptoKey) -> String;
pub fn generate_session_key() -> SymmetricCryptoKey;

// Protected data operations
pub fn encrypt_protected_bytes(plain: &[u8], key: &SymmetricCryptoKey) -> Result<String, ProtectedStorageError>;
pub fn decrypt_protected_bytes(encrypted_b64: &str, key: &SymmetricCryptoKey) -> Result<Vec<u8>, ProtectedStorageError>;

// User key operations
pub fn encrypt_user_key(user_key: &SymmetricCryptoKey, session_key: &SymmetricCryptoKey) -> Result<String, ProtectedStorageError>;
pub fn decrypt_user_key(encrypted_b64: &str, session_key: &SymmetricCryptoKey) -> Result<SymmetricCryptoKey, ProtectedStorageError>;
```

**Error Type:**

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

**Implementation Notes:**

1. Use `SymmetricCryptoKey::make_aes256_cbc_hmac_key()` for session key generation (replaces `SessionKey::generate()`)
2. Use `OctetStreamBytes` wrapper for encrypting raw bytes
3. Use `EncString::to_buffer()` for binary format (EncArrayBuffer)
4. Use `EncString::from_buffer()` for parsing binary format
5. Use `BitwardenLegacyKeyBytes` for key encoding/decoding
6. Output is base64-encoded EncArrayBuffer (binary format, not string format)

**SDK Integration Pattern:**

```rust
// Encryption
let enc_string: EncString = OctetStreamBytes::from(plain_bytes.to_vec())
    .encrypt_with_key(&session_key)?;
let buffer = enc_string.to_buffer()?;
let result = base64::encode(&buffer);

// Decryption
let buffer = base64::decode(encrypted_b64)?;
let enc_string = EncString::from_buffer(&buffer)?;
let decrypted: Vec<u8> = enc_string.decrypt_with_key(&session_key)?;
```

---

### 2.2 Key Service

**File:** `crates/bw-core/src/services/key_service.rs`

**Purpose:** Retrieves and manages the user key for vault decryption operations.

**Public API:**

```rust
pub struct KeyService {
    storage: Arc<Mutex<JsonFileStorage>>,
    account_manager: Arc<AccountManager>,
}

impl KeyService {
    pub fn new(
        storage: Arc<Mutex<JsonFileStorage>>,
        account_manager: Arc<AccountManager>,
    ) -> Self;

    /// Get user key from protected storage using session key
    ///
    /// # Arguments
    /// * `session_str` - Base64-encoded session key (from BW_SESSION or --session)
    ///
    /// # Returns
    /// User symmetric key ready for vault decryption
    pub async fn get_user_key(&self, session_str: &str) -> Result<SymmetricCryptoKey, KeyServiceError>;

    /// Store user key in protected storage
    ///
    /// Called during login/unlock after decrypting user key with master key
    pub async fn store_user_key(
        &self,
        user_id: &str,
        user_key: &SymmetricCryptoKey,
        session_key: &SymmetricCryptoKey,
    ) -> Result<(), KeyServiceError>;

    /// Check if user key is stored for the active user
    pub async fn has_user_key(&self) -> Result<bool, KeyServiceError>;
}
```

**Error Type:**

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

### 2.3 CipherService Updates

**File:** `crates/bw-core/src/services/vault/cipher_service.rs`

**Changes Required:**

1. Add `user_key` parameter to decryption methods
2. Replace placeholder `decrypt_string` with real SDK decryption
3. Use `EncString::from_str()` to parse encrypted fields
4. Use `KeyDecryptable` trait for decryption

**Updated API:**

```rust
impl CipherService {
    // Existing - add user_key parameter
    pub async fn decrypt_cipher(
        &self,
        cipher: &Cipher,
        user_key: &SymmetricCryptoKey,
    ) -> Result<CipherView, VaultError>;

    pub async fn decrypt_ciphers(
        &self,
        ciphers: &[Cipher],
        user_key: &SymmetricCryptoKey,
    ) -> Result<Vec<CipherView>, VaultError>;

    pub async fn decrypt_folders(
        &self,
        folders: &[Folder],
        user_key: &SymmetricCryptoKey,
    ) -> Result<Vec<FolderView>, VaultError>;

    pub async fn decrypt_collections(
        &self,
        collections: &[Collection],
        user_key: &SymmetricCryptoKey,
    ) -> Result<Vec<CollectionView>, VaultError>;

    // New private helper
    fn decrypt_string(
        &self,
        enc_string: &str,
        key: &SymmetricCryptoKey,
    ) -> Result<String, VaultError>;
}
```

**Decryption Pattern:**

```rust
fn decrypt_string(&self, enc_string: &str, key: &SymmetricCryptoKey) -> Result<String, VaultError> {
    if enc_string.is_empty() {
        return Ok(String::new());
    }

    let enc: EncString = enc_string.parse()
        .map_err(|e| VaultError::DecryptionError(format!("Invalid EncString: {}", e)))?;

    enc.decrypt_with_key(key)
        .map_err(|e| VaultError::DecryptionError(format!("Decryption failed: {}", e)))
}
```

---

### 2.4 AuthService Updates

**File:** `crates/bw-core/src/services/auth/auth_service.rs`

**Changes Required:**

1. Store encrypted user key in protected storage after login/unlock
2. Generate session key using SDK instead of custom `SessionKey` type
3. Return session key in format compatible with TypeScript CLI

**Modified Flow - Login:**

```rust
pub async fn login_with_password(...) -> Result<LoginResult, AuthError> {
    // ... existing steps 1-5 (KDF, master key, authentication) ...

    // Step 6: Decrypt user key (EXISTING)
    let user_key = self.decrypt_user_key(&encrypted_key, &master_key).await?;

    // Step 7: Generate session key (MODIFIED - use SDK)
    let session_key = SymmetricCryptoKey::make_aes256_cbc_hmac_key();

    // Step 8: Store encrypted user key in protected storage (NEW)
    let encrypted_user_key = protected_storage::encrypt_user_key(&user_key, &session_key)?;
    let protected_key = protected_storage::make_protected_key(
        &protected_storage::user_key_protected_storage_key(&profile.id)
    );
    storage.set(&protected_key, &encrypted_user_key).await?;

    // Step 9: Format session key for export (MODIFIED)
    let session_key_str = protected_storage::format_session_key(&session_key);

    // ... persist auth state ...

    Ok(LoginResult {
        user_id: profile.id,
        email: profile.email,
        session_key: session_key_str,
    })
}
```

**Modified Flow - Unlock:**

```rust
pub async fn unlock(&self, password: Secret<String>) -> Result<UnlockResult, AuthError> {
    // ... existing validation and master key derivation ...

    // Decrypt user key (EXISTING)
    let user_key = self.decrypt_user_key(&encrypted_user_key, &master_key).await?;

    // Generate new session key (MODIFIED - use SDK)
    let session_key = SymmetricCryptoKey::make_aes256_cbc_hmac_key();

    // Store encrypted user key in protected storage (NEW)
    let encrypted_user_key = protected_storage::encrypt_user_key(&user_key, &session_key)?;
    let protected_key = protected_storage::make_protected_key(
        &protected_storage::user_key_protected_storage_key(&user_id)
    );
    storage.set(&protected_key, &encrypted_user_key).await?;

    // Format session key for export (MODIFIED)
    let session_key_str = protected_storage::format_session_key(&session_key);

    Ok(UnlockResult {
        session_key: session_key_str,
    })
}
```

---

### 2.5 VaultService Updates

**File:** `crates/bw-core/src/services/vault/mod.rs`

**Changes Required:**

1. Add `KeyService` dependency
2. Pass user key to decryption methods

**Updated Implementation:**

```rust
pub struct VaultService {
    sync_service: SyncService,
    cipher_service: CipherService,
    search_service: SearchService,
    totp_service: TotpService,
    key_service: KeyService,  // NEW
    storage: Arc<Mutex<JsonFileStorage>>,
}

impl VaultService {
    pub fn new(
        api_client: Arc<BitwardenApiClient>,
        storage: Arc<Mutex<JsonFileStorage>>,
        sdk_client: Arc<Client>,
        account_manager: Arc<AccountManager>,  // NEW
    ) -> Self {
        // ... existing services ...
        let key_service = KeyService::new(Arc::clone(&storage), account_manager);
        // ...
    }

    /// List all items with optional filters
    ///
    /// # Arguments
    /// * `filters` - Optional filters to apply
    /// * `session` - Session key string for decryption
    pub async fn list_items(
        &self,
        filters: &ItemFilters,
        session: &str,  // NEW
    ) -> Result<Vec<CipherView>, VaultError> {
        let user_key = self.key_service.get_user_key(session).await
            .map_err(|e| VaultError::DecryptionError(e.to_string()))?;

        let vault_data = self.get_vault_data().await?;
        let filtered = self.search_service.filter_ciphers(&vault_data.ciphers, filters);
        self.cipher_service.decrypt_ciphers(&filtered, &user_key).await
    }

    // Similar updates for get_item, list_folders, list_collections, etc.
}
```

---

### 2.6 CLI Command Updates

**File:** `crates/bw-cli/src/commands/vault.rs`

**Changes Required:**

1. Get session from BW_SESSION environment variable or `--session` flag
2. Pass session to VaultService methods

**Session Retrieval Pattern:**

```rust
fn get_session(global_args: &GlobalArgs) -> Result<String, anyhow::Error> {
    // Priority: --session flag > BW_SESSION env var
    if let Some(session) = &global_args.session {
        return Ok(session.clone());
    }

    std::env::var("BW_SESSION")
        .map_err(|_| anyhow::anyhow!("Session key required. Export BW_SESSION or use --session flag."))
}
```

**Updated Command:**

```rust
pub async fn execute_list(cmd: ListCommands, global_args: &GlobalArgs) -> anyhow::Result<Response> {
    let session = get_session(global_args)?;  // NEW
    let vault_service = create_vault_service(global_args)?;

    match cmd {
        ListCommands::Items(item_cmd) => {
            let filters = ItemFilters { /* ... */ };
            match vault_service.list_items(&filters, &session).await {  // MODIFIED
                Ok(items) => Ok(Response::success(items)),
                Err(e) => Ok(Response::error(e.to_string())),
            }
        }
        // ... other commands
    }
}
```

---

### 2.7 Storage Keys Update

**File:** `crates/bw-core/src/services/storage/keys.rs`

**Changes Required:**

Add new storage key variant for protected user key:

```rust
pub enum StorageKey {
    // ... existing keys ...

    /// User key encrypted with session key (protected storage)
    /// Format: __PROTECTED__{userId}_user_auto
    UserProtectedKey,
}

impl StorageKey {
    pub fn format(&self, user_id: Option<&str>) -> String {
        match self {
            // ... existing matches ...

            Self::UserProtectedKey => {
                let uid = user_id.expect("UserProtectedKey requires user_id");
                format!("__PROTECTED__{}_user_auto", uid)
            }
        }
    }
}
```

---

## 3. Data Flow Diagrams

### 3.1 Login Flow (Updated)

```
User Input                     CLI                              Server
─────────                      ───                              ──────

email, password ──────────► Fetch KDF config ◄──────────────────► /prelogin
                                   │
                                   ▼
                            Derive Master Key
                            (PBKDF2/Argon2)
                                   │
                                   ▼
                            Hash for Auth
                                   │
                                   ▼
                            Authenticate ◄──────────────────────► /token
                                   │
                            ┌──────┴──────┐
                            │             │
                            ▼             ▼
                    Login Response    Encrypted
                    (tokens)          User Key
                                         │
                                         ▼
                                  Decrypt User Key
                                  (with master key)
                                         │
                                         ▼
                              ┌──────────────────────┐
                              │ Generate Session Key │
                              │ (SDK: make_aes256... │
                              └──────────────────────┘
                                         │
                              ┌──────────┴──────────┐
                              │                     │
                              ▼                     ▼
                       Encrypt User Key      Store Tokens
                       (with session key)    & KDF config
                              │
                              ▼
                       Store Protected
                       __PROTECTED__{id}_user_auto
                              │
                              ▼
◄─────────────────────  BW_SESSION
                       (session key b64)
```

### 3.2 Vault Operation Flow

```
BW_SESSION env var                CLI                           Storage
─────────────────                 ───                           ───────

BW_SESSION ─────────────────► Parse Session Key
                                    │
                                    ▼
                              Get Active User ID ◄────────────── data.json
                                    │
                                    ▼
                              Read Protected Key ◄────────────── __PROTECTED__...
                                    │
                                    ▼
                              Decrypt User Key
                              (session key)
                                    │
                                    ▼
                              Read Vault Data ◄─────────────────  vaultData
                                    │
                                    ▼
                              Decrypt Ciphers
                              (user key)
                                    │
                                    ▼
◄───────────────────────── JSON Output
                              (decrypted)
```

---

## 4. File Changes Summary

### 4.1 New Files

| File | Purpose |
|------|---------|
| `crates/bw-core/src/services/storage/protected_storage.rs` | Protected storage encryption/decryption |
| `crates/bw-core/src/services/key_service.rs` | User key retrieval and management |

### 4.2 Modified Files

| File | Changes |
|------|---------|
| `crates/bw-core/src/services/storage/mod.rs` | Export protected_storage module |
| `crates/bw-core/src/services/storage/keys.rs` | Add UserProtectedKey variant |
| `crates/bw-core/src/services/auth/auth_service.rs` | Store encrypted user key on login/unlock |
| `crates/bw-core/src/services/vault/cipher_service.rs` | Real decryption implementation |
| `crates/bw-core/src/services/vault/mod.rs` | Add KeyService, pass user key to decryption |
| `crates/bw-core/src/services/mod.rs` | Export key_service |
| `crates/bw-core/src/services/container.rs` | Add AccountManager to VaultService |
| `crates/bw-cli/src/commands/vault.rs` | Get session, pass to services |

---

## 5. SDK Integration Details

### 5.1 Required SDK Types

From `bitwarden-crypto`:

| Type | Purpose |
|------|---------|
| `SymmetricCryptoKey` | Session key and user key representation |
| `EncString` | Encrypted string container with type/iv/ct/mac |
| `OctetStreamBytes` | Wrapper for encrypting raw bytes |
| `BitwardenLegacyKeyBytes` | Key encoding/decoding for storage |
| `KeyEncryptable` trait | For encrypting data with a key |
| `KeyDecryptable` trait | For decrypting data with a key |

### 5.2 Key Generation

```rust
// Generate new 64-byte AES-256-CBC-HMAC key
let session_key = SymmetricCryptoKey::make_aes256_cbc_hmac_key();
```

### 5.3 Key Encoding/Decoding

```rust
// Encoding key to bytes for storage
let encoded: BitwardenLegacyKeyBytes = user_key.to_encoded();
let bytes: Vec<u8> = encoded.as_ref().to_vec();

// Decoding key from bytes
let legacy_bytes = BitwardenLegacyKeyBytes::from(bytes);
let key = SymmetricCryptoKey::try_from(&legacy_bytes)?;
```

### 5.4 EncString Formats

**String Format (vault fields):**
```
"2.base64_iv|base64_ciphertext|base64_mac"

Type 2 = AES-256-CBC with HMAC-SHA256
```

**Binary Format (protected storage):**
```
[1 byte: type][16 bytes: IV][32 bytes: MAC][variable: ciphertext]
```

---

## 6. Error Handling Strategy

### 6.1 Error Propagation

```
CLI Command
    │
    ├──► Session parsing error → "Session key required. Export BW_SESSION..."
    │
    ├──► KeyService error → "Failed to retrieve user key: {details}"
    │
    └──► VaultError::DecryptionError → "Decryption failed: {details}"
```

### 6.2 Graceful Degradation

For batch operations (list_items, decrypt_ciphers):
- Log warning for individual item decryption failures
- Continue processing remaining items
- Return successfully decrypted items

```rust
for cipher in ciphers {
    match self.decrypt_cipher(cipher, user_key).await {
        Ok(decrypted) => results.push(decrypted),
        Err(e) => {
            tracing::warn!("Failed to decrypt cipher {}: {}", cipher.id, e);
            // Continue with next cipher
        }
    }
}
```

---

## 7. Testing Strategy

### 7.1 Unit Tests

**protected_storage.rs:**
- `test_encrypt_decrypt_roundtrip` - Basic encrypt/decrypt cycle
- `test_user_key_roundtrip` - User key specific handling
- `test_session_key_parsing` - Valid base64 parsing
- `test_invalid_session_key` - Error handling for invalid keys
- `test_wrong_key_fails` - Decryption with wrong key fails cleanly
- `test_storage_key_format` - Key names match TypeScript format

**key_service.rs:**
- `test_get_user_key_success` - Happy path retrieval
- `test_no_active_user` - Error when not logged in
- `test_missing_protected_key` - Error when key not stored
- `test_invalid_session` - Error for malformed session

**cipher_service.rs:**
- `test_decrypt_login_cipher` - Login type decryption
- `test_decrypt_secure_note` - Secure note decryption
- `test_decrypt_card` - Card type decryption
- `test_decrypt_identity` - Identity type decryption
- `test_decrypt_with_fields` - Custom fields decryption
- `test_decrypt_empty_fields` - Handle null/empty fields

### 7.2 Integration Tests

- `test_login_stores_protected_key` - Verify key stored after login
- `test_unlock_stores_protected_key` - Verify key stored after unlock
- `test_full_flow_login_list` - Login → list items → decrypted output
- `test_full_flow_unlock_get` - Unlock → get item → decrypted output

### 7.3 Manual Testing Checklist

1. Fresh login with password → `bw list items` shows readable names
2. `bw get password <id>` returns actual password
3. `bw get username <id>` returns actual username
4. `bw get totp <id>` returns valid 6-digit code
5. Lock → unlock → commands still work
6. Wrong session key → clear error message
7. Compare output with TypeScript CLI for format compatibility

---

## 8. Implementation Order

### Phase 1: Protected Storage Foundation (Estimated: Core infrastructure)
1. Create `protected_storage.rs` with encryption/decryption functions
2. Add error types
3. Write unit tests for roundtrip operations
4. Update storage/mod.rs exports

### Phase 2: Key Service (Estimated: Service layer)
1. Create `key_service.rs`
2. Implement `get_user_key` and `store_user_key`
3. Add error handling
4. Write unit tests

### Phase 3: Auth Flow Integration (Estimated: Integration)
1. Update `auth_service.rs` login flow to store protected key
2. Update `auth_service.rs` unlock flow to store protected key
3. Replace `SessionKey` usage with SDK `SymmetricCryptoKey`
4. Write integration tests

### Phase 4: Decryption Implementation (Estimated: Core feature)
1. Update `cipher_service.rs` with real SDK decryption
2. Add user key parameter to all decryption methods
3. Handle error cases gracefully
4. Write unit tests for each cipher type

### Phase 5: Command Integration (Estimated: CLI layer)
1. Update `vault.rs` commands to get session
2. Update `VaultService` to accept and use session
3. Pass user key through the decryption chain
4. End-to-end testing

### Phase 6: Cleanup and Verification (Estimated: Polish)
1. Run full test suite
2. Cross-compatibility testing with TypeScript CLI
3. Documentation updates
4. Code review and cleanup

---

## 9. Security Considerations

### 9.1 Sensitive Data Handling

- User key should never be logged or printed
- Use `zeroize` on sensitive byte arrays when done
- Session key transmitted only via BW_SESSION (user responsibility)
- Clear decrypted data from memory after use where practical

### 9.2 Error Messages

- Never expose cryptographic details in error messages
- Use generic "decryption failed" messages
- Log detailed errors at debug level only

### 9.3 Storage Security

- Protected keys use same encryption as TypeScript CLI
- Session key required to access user key
- No plaintext sensitive data in storage

---

## 10. Compatibility Considerations

### 10.1 TypeScript CLI Compatibility

**Must match exactly:**
- Storage key format: `__PROTECTED__{userId}_user_auto`
- Session key format: 64-byte base64-encoded
- Protected storage format: base64-encoded EncArrayBuffer

**Verification:**
- Login with TypeScript CLI, use same session with Rust CLI
- Login with Rust CLI, use same session with TypeScript CLI

### 10.2 Backward Compatibility

- Existing storage (tokens, vault data) unchanged
- New protected storage keys added alongside existing
- No migration needed for existing data

---

## 11. Dependencies

### 11.1 SDK Crates (Already in Cargo.toml)

```toml
bitwarden-crypto = { path = "../sdk-internal/crates/bitwarden-crypto" }
```

### 11.2 Other Crates (Already in Cargo.toml)

```toml
base64 = "0.22"
zeroize = { version = "1.8", features = ["derive"] }
thiserror = "1.0"
```

---

## 12. Open Questions

None - the research phase has addressed all architectural questions. The implementation approach using SDK primitives is clear and well-documented.

---

## 13. References

- Enhancement specification: `enhancements/11-encrypt-decrypt/11-encrypt-decrypt.md`
- Research document: `docs/research/vault_decryption_research.md`
- Requirements analysis: `enhancements/11-encrypt-decrypt/requirements-analyst/required_output/analysis_summary.md`
- TypeScript CLI reference: `apps/cli/src/platform/services/node-env-secure-storage.service.ts`
- SDK EncString: `crates/bitwarden-crypto/src/enc_string/symmetric.rs`
- SDK SymmetricCryptoKey: `crates/bitwarden-crypto/src/keys/symmetric_crypto_key.rs`
