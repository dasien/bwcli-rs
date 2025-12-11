# Vault Decryption Research Document

## Overview

This document captures the research findings for implementing vault item decryption in the Rust CLI, enabling it to decrypt and display vault contents (passwords, usernames, notes, etc.) rather than returning encrypted data.

## Current State

The Rust CLI currently:
- Can login (password and API key)
- Can sync vault data from server
- Can list items and get individual items
- **But returns encrypted data** - fields like `name`, `username`, `password` are still in EncString format (e.g., `"2.iv|data|mac"`)

## Architecture Understanding

### TypeScript CLI Architecture

The TypeScript CLI uses a layered security model:

1. **BW_SESSION** - A random 64-byte key generated on login/unlock
2. **Protected Storage** - Sensitive data (like the user key) is encrypted with BW_SESSION and stored with `__PROTECTED__` prefix
3. **User Key** - The actual symmetric key used to decrypt vault items

### Key Flow

```
Login/Unlock:
1. User provides master password
2. Derive master key from password + email + KDF params
3. Decrypt encrypted user key (from server/storage) with master key
4. Generate random 64-byte session key
5. Encrypt user key with session key
6. Store encrypted user key as `__PROTECTED__{userId}_user_auto`
7. Return session key as BW_SESSION

Vault Operations:
1. Parse BW_SESSION to get session key
2. Read `__PROTECTED__{userId}_user_auto` from storage
3. Decrypt to get user key
4. Use user key to decrypt vault item fields (EncString → plaintext)
```

### Storage Keys

TypeScript CLI storage format (in `data.json`):
- `user_{userId}_kdfConfig_kdfConfig` - KDF configuration
- `user_{userId}_masterPassword_masterKeyEncryptedUserKey` - Encrypted user key
- `user_{userId}_ciphers_ciphers` - Encrypted vault items
- `__PROTECTED__{userId}_user_auto` - User key encrypted with session key

## SDK Crypto Functions

The `bitwarden-crypto` crate provides all necessary primitives:

### Key Types
- `SymmetricCryptoKey` - Symmetric encryption key (user key, session key)
- `BitwardenLegacyKeyBytes` - For converting between `Vec<u8>` and `SymmetricCryptoKey`
- `OctetStreamBytes` - For encrypting raw bytes

### EncString Format
Two representations:
1. **String format**: `"2.iv_base64|data_base64|mac_base64"` - Used for vault item fields
2. **Binary format (EncArrayBuffer)**: `[encType][IV][MAC][data]` - Used for protected storage

### Key Methods

```rust
// Encrypt raw bytes
let enc_string: EncString = OctetStreamBytes::from(plain_bytes)
    .encrypt_with_key(&key)?;

// Convert to binary format (EncArrayBuffer)
let buffer: Vec<u8> = enc_string.to_buffer()?;

// Parse binary format
let enc_string = EncString::from_buffer(&buffer)?;

// Decrypt to bytes
let decrypted: Vec<u8> = enc_string.decrypt_with_key(&key)?;

// Decrypt to string
let decrypted: String = enc_string.decrypt_with_key(&key)?;

// Key encoding/decoding
let encoded = key.to_encoded();  // BitwardenLegacyKeyBytes
let key = SymmetricCryptoKey::try_from(&BitwardenLegacyKeyBytes::from(bytes))?;

// Generate new key
let key = SymmetricCryptoKey::make_aes256_cbc_hmac_key();
```

### Traits
- `KeyEncryptable<SymmetricCryptoKey, EncString>` - Implemented for `String`, `&str`, `&Bytes<T>`
- `KeyDecryptable<SymmetricCryptoKey, String>` - Implemented for `EncString`
- `KeyDecryptable<SymmetricCryptoKey, Vec<u8>>` - Implemented for `EncString`

## Implementation Plan

### Step 1: Protected Storage Module

Create `crates/bw-core/src/services/storage/protected_storage.rs`:

```rust
use bitwarden_crypto::{
    BitwardenLegacyKeyBytes, EncString, KeyDecryptable, KeyEncryptable,
    OctetStreamBytes, SymmetricCryptoKey,
};

pub const PROTECTED_PREFIX: &str = "__PROTECTED__";

pub fn make_protected_key(key: &str) -> String {
    format!("{}{}", PROTECTED_PREFIX, key)
}

pub fn user_key_protected_storage_key(user_id: &str) -> String {
    format!("{}_user_auto", user_id)
}

pub fn parse_session_key(session_str: &str) -> Result<SymmetricCryptoKey, Error> {
    let key_bytes = base64::decode(session_str)?;
    let legacy_bytes = BitwardenLegacyKeyBytes::from(key_bytes);
    SymmetricCryptoKey::try_from(&legacy_bytes)
}

pub fn encrypt_protected_bytes(plain: &[u8], key: &SymmetricCryptoKey) -> Result<String, Error> {
    let enc_string: EncString = OctetStreamBytes::from(plain.to_vec())
        .encrypt_with_key(key)?;
    let buffer = enc_string.to_buffer()?;
    Ok(base64::encode(&buffer))
}

pub fn decrypt_protected_bytes(encrypted_b64: &str, key: &SymmetricCryptoKey) -> Result<Vec<u8>, Error> {
    let buffer = base64::decode(encrypted_b64)?;
    let enc_string = EncString::from_buffer(&buffer)?;
    enc_string.decrypt_with_key(key)
}

pub fn encrypt_user_key(user_key: &SymmetricCryptoKey, session_key: &SymmetricCryptoKey) -> Result<String, Error> {
    let encoded = user_key.to_encoded();
    encrypt_protected_bytes(encoded.as_ref(), session_key)
}

pub fn decrypt_user_key(encrypted_b64: &str, session_key: &SymmetricCryptoKey) -> Result<SymmetricCryptoKey, Error> {
    let bytes = decrypt_protected_bytes(encrypted_b64, session_key)?;
    let legacy_bytes = BitwardenLegacyKeyBytes::from(bytes);
    SymmetricCryptoKey::try_from(&legacy_bytes)
}
```

### Step 2: Update Login/Unlock Flow

In `auth_service.rs`, modify `login_with_password` and `unlock`:

```rust
// After decrypting user key with master key:
let user_key = self.decrypt_user_key(encrypted_key, &master_key).await?;

// Generate random session key
let session_key = SymmetricCryptoKey::make_aes256_cbc_hmac_key();

// Encrypt user key with session key
let encrypted_user_key = protected_storage::encrypt_user_key(&user_key, &session_key)?;

// Store in protected storage
let storage_key = protected_storage::make_protected_key(
    &protected_storage::user_key_protected_storage_key(&user_id)
);
self.storage.set(&storage_key, &encrypted_user_key).await?;

// Return session key as BW_SESSION
let session_key_str = session_key.to_base64().to_string();
```

### Step 3: User Key Retrieval Service

Create a service to retrieve the user key for vault operations:

```rust
pub struct KeyService {
    storage: Arc<dyn StorageService>,
}

impl KeyService {
    pub async fn get_user_key(&self, session_str: &str) -> Result<SymmetricCryptoKey, Error> {
        // Parse session key from BW_SESSION
        let session_key = protected_storage::parse_session_key(session_str)?;

        // Get active user ID
        let user_id = self.storage.get_active_user_id().await?;

        // Read encrypted user key from protected storage
        let storage_key = protected_storage::make_protected_key(
            &protected_storage::user_key_protected_storage_key(&user_id)
        );
        let encrypted_user_key = self.storage.get(&storage_key).await?
            .ok_or(Error::NotLoggedIn)?;

        // Decrypt and return user key
        protected_storage::decrypt_user_key(&encrypted_user_key, &session_key)
    }
}
```

### Step 4: Update CipherService for Decryption

In `cipher_service.rs`:

```rust
impl CipherService {
    pub fn decrypt_cipher(&self, cipher: &Cipher, user_key: &SymmetricCryptoKey) -> Result<DecryptedCipher, Error> {
        Ok(DecryptedCipher {
            id: cipher.id.clone(),
            name: self.decrypt_string(&cipher.name, user_key)?,
            login: cipher.login.as_ref().map(|l| self.decrypt_login(l, user_key)).transpose()?,
            // ... other fields
        })
    }

    fn decrypt_string(&self, enc_str: &Option<String>, key: &SymmetricCryptoKey) -> Result<Option<String>, Error> {
        match enc_str {
            Some(s) if !s.is_empty() => {
                let enc_string = EncString::from_str(s)?;
                Ok(Some(enc_string.decrypt_with_key(key)?))
            }
            _ => Ok(None)
        }
    }

    fn decrypt_login(&self, login: &CipherLogin, key: &SymmetricCryptoKey) -> Result<DecryptedLogin, Error> {
        Ok(DecryptedLogin {
            username: self.decrypt_string(&login.username, key)?,
            password: self.decrypt_string(&login.password, key)?,
            totp: self.decrypt_string(&login.totp, key)?,
            uris: login.uris.as_ref().map(|uris| {
                uris.iter().map(|u| self.decrypt_uri(u, key)).collect()
            }).transpose()?,
        })
    }
}
```

### Step 5: Update CLI Commands

In vault commands (list, get):

```rust
pub async fn execute_list_items(cmd: ListItemsCommand, global_args: &GlobalArgs) -> Result<Response> {
    // Get session from BW_SESSION env var or --session flag
    let session = get_session(global_args)?;

    // Get services
    let container = ServiceContainer::new(...)?;
    let key_service = container.key_service();
    let cipher_service = container.cipher_service();

    // Get user key
    let user_key = key_service.get_user_key(&session).await?;

    // Get ciphers
    let ciphers = cipher_service.get_all_ciphers().await?;

    // Decrypt ciphers
    let decrypted: Vec<DecryptedCipher> = ciphers
        .iter()
        .map(|c| cipher_service.decrypt_cipher(c, &user_key))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Response::json(&decrypted))
}
```

## Files to Create/Modify

### New Files
1. `crates/bw-core/src/services/storage/protected_storage.rs` - Protected storage encryption/decryption
2. `crates/bw-core/src/services/key_service.rs` - User key retrieval service
3. `crates/bw-core/src/models/vault/decrypted.rs` - Decrypted vault item models

### Modified Files
1. `crates/bw-core/src/services/storage/mod.rs` - Export protected_storage module
2. `crates/bw-core/src/services/auth/auth_service.rs` - Store encrypted user key on login/unlock
3. `crates/bw-core/src/services/vault/cipher_service.rs` - Add decryption methods
4. `crates/bw-cli/src/commands/vault.rs` - Use decryption in list/get commands

## Testing Strategy

1. **Unit tests for protected storage**
   - Encrypt/decrypt roundtrip
   - User key encrypt/decrypt roundtrip
   - Session key parsing
   - Wrong key fails decryption

2. **Integration tests**
   - Login → list items shows decrypted data
   - Unlock → get item shows decrypted password

3. **Manual testing**
   - Login with real account
   - `bw list items` shows readable names
   - `bw get item <id>` shows decrypted login credentials

## TypeScript CLI Reference Files

Key files in the TypeScript CLI for reference:
- `apps/cli/src/platform/services/node-env-secure-storage.service.ts` - Protected storage implementation
- `libs/common/src/key-management/crypto/services/encrypt.service.implementation.ts` - Encryption service
- `libs/key-management/src/key.service.ts` - Key management
- `libs/common/src/platform/services/key-state/user-key.state.ts` - User key state

## Dependencies

The implementation uses these crates from the SDK:
- `bitwarden_crypto` - Core crypto primitives
- `base64` - Encoding/decoding (already in use)

No new external dependencies needed.
