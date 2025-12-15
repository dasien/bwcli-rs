# SDK Vault Bridge Developer Guide

## Introduction

The `SdkVaultBridge` provides a bridge layer between the CLI's vault operations and the Bitwarden SDK's `VaultClient`. This guide explains how to use the bridge for vault encryption and decryption operations.

## Prerequisites

Before using the SDK Vault Bridge, ensure you have:

1. A valid user session with decrypted user key
2. User's KDF configuration (from account state)
3. User's email address
4. Optionally, the encrypted private key (for organization operations)

## Basic Usage

### 1. Creating a Bridge Instance

```rust
use bitwarden_core::Client;
use std::sync::Arc;
use bw_core::services::vault::sdk_bridge::SdkVaultBridge;

// Create SDK client
let client = Arc::new(Client::new(None));

// Create bridge
let bridge = SdkVaultBridge::new(client);
```

### 2. Initializing Crypto

Before any encrypt/decrypt operations, initialize the SDK crypto state:

```rust
use bw_core::models::state::KdfConfig;

// Get these from your session/storage
let user_key: SymmetricCryptoKey = /* ... */;
let email = "user@example.com";
let kdf_config = KdfConfig {
    kdf_type: KdfType::PBKDF2SHA256,
    iterations: Some(600_000),
    memory: None,
    parallelism: None,
};
let private_key = Some("2.encrypted|private|key");

// Initialize crypto
bridge.initialize_crypto(&user_key, email, &kdf_config, private_key).await?;

// Verify initialization
assert!(bridge.is_crypto_initialized());
```

### 3. Decrypting Ciphers

#### Single Cipher

```rust
// Get encrypted cipher from storage
let cipher: Cipher = storage.get_cipher(cipher_id)?;

// Decrypt
let cipher_view: CipherView = bridge.decrypt_cipher(&cipher)?;

// Access decrypted fields
println!("Name: {}", cipher_view.name);
if let Some(login) = &cipher_view.login {
    println!("Username: {:?}", login.username);
}
```

#### Multiple Ciphers

```rust
// Get all encrypted ciphers
let ciphers: Vec<Cipher> = storage.get_all_ciphers()?;

// Decrypt all (continues on individual failures)
let decrypted: Vec<CipherView> = bridge.decrypt_ciphers(&ciphers)?;
```

### 4. Encrypting Ciphers

```rust
// Create or modify a cipher view
let cipher_view = CipherView {
    id: "".to_string(), // Empty for new ciphers
    cipher_type: CipherType::Login,
    name: "New Login".to_string(),
    login: Some(CipherLoginView {
        username: Some("newuser".to_string()),
        password: Some("newpass".to_string()),
        uris: vec![],
        totp: None,
    }),
    // ... other fields
};

// Encrypt
let encrypted_cipher: Cipher = bridge.encrypt_cipher(&cipher_view)?;

// Save to storage or send to API
storage.save_cipher(&encrypted_cipher)?;
```

### 5. Working with Folders

```rust
// Decrypt a folder
let folder: Folder = storage.get_folder(folder_id)?;
let folder_view: FolderView = bridge.decrypt_folder(&folder)?;
println!("Folder name: {}", folder_view.name);

// Encrypt a new folder name
let encrypted_name: String = bridge.encrypt_folder_name("My New Folder")?;
```

### 6. Working with Collections

```rust
// Decrypt collections (read-only, no encryption method)
let collections: Vec<Collection> = storage.get_collections()?;
let decrypted: Vec<CollectionView> = bridge.decrypt_collections(&collections)?;

for collection in decrypted {
    println!("Collection: {} (org: {})", collection.name, collection.organization_id);
}
```

## Type Conversion Details

### JSON Serialization Approach

Encrypted types (`Cipher`, `Folder`, `Collection`) use JSON serialization for conversion because SDK internal types are not publicly exported:

```rust
// CLI types and SDK types share camelCase JSON format
let json = serde_json::to_value(&cli_cipher)?;
let sdk_cipher: bitwarden_vault::Cipher = serde_json::from_value(json)?;
```

### Direct Field Mapping

Decrypted view types use direct field mapping for type safety:

```rust
// CipherView fields are mapped individually
let sdk_view = bitwarden_vault::CipherView {
    name: cli_view.name.clone(),
    notes: cli_view.notes.clone(),
    r#type: cli_cipher_type_to_sdk(cli_view.cipher_type),
    // ... more fields
};
```

## Error Handling

### Common Errors

| Error | Cause | Solution |
|-------|-------|----------|
| `CryptoInitFailed` | Invalid user key or private key | Verify key format and re-authenticate if needed |
| `DecryptionError` | Invalid EncString format or wrong key | Check that crypto is initialized with correct user key |
| `EncryptionError` | Crypto not initialized | Call `initialize_crypto()` before encryption |

### Error Handling Pattern

```rust
match bridge.decrypt_cipher(&cipher) {
    Ok(view) => {
        // Use decrypted view
    }
    Err(VaultError::DecryptionError(msg)) => {
        // Log and potentially skip this cipher
        tracing::warn!("Failed to decrypt cipher {}: {}", cipher.id, msg);
    }
    Err(e) => {
        // Handle other errors
        return Err(e.into());
    }
}
```

## KDF Configuration

The bridge supports both KDF types used by Bitwarden:

### PBKDF2-SHA256

```rust
let kdf_config = KdfConfig {
    kdf_type: KdfType::PBKDF2SHA256,
    iterations: Some(600_000), // Default if None
    memory: None,
    parallelism: None,
};
```

### Argon2id

```rust
let kdf_config = KdfConfig {
    kdf_type: KdfType::Argon2id,
    iterations: Some(3),      // Default if None
    memory: Some(64),         // MB, converted to KiB internally
    parallelism: Some(4),     // Default if None
};
```

## Supported Cipher Types

| Type | Encrypt | Decrypt | Notes |
|------|---------|---------|-------|
| Login | Yes | Yes | Full support including URIs and TOTP |
| SecureNote | Yes | Yes | Full support |
| Card | Yes | Yes | Full support |
| Identity | Yes | Yes | Full support |
| SshKey | Limited | Limited | Type conversion only, SSH key fields not fully mapped |

## Supported Field Types

| Type | Code | Description |
|------|------|-------------|
| Text | 0 | Plain text field |
| Hidden | 1 | Hidden/password field |
| Boolean | 2 | True/false field |
| Linked | 3 | Linked to cipher field |

Unknown field types default to Text (0).

## Best Practices

1. **Initialize Once**: Initialize crypto once per session, not per operation
2. **Handle Failures Gracefully**: Use `decrypt_ciphers()` for bulk operations as it continues on individual failures
3. **Check Initialization**: Call `is_crypto_initialized()` before operations if unsure
4. **Preserve Fields**: When encrypting modified ciphers, ensure all original fields are preserved
5. **Use Appropriate Types**: Use `Cipher` for storage, `CipherView` for display/editing

## Future Integration

This bridge module is designed for gradual integration. Future steps include:

1. Integrating with `CipherService` for transparent SDK usage
2. Integrating with `VaultService` for automatic crypto initialization
3. Integrating with `WriteService` for API operations

## Testing

Run the bridge tests:

```bash
cargo test -p bw-core --lib services::vault::sdk_bridge::tests
```

Test coverage includes:
- KDF conversion (4 tests)
- Type conversions (15+ tests)
- JSON serialization (4 tests)
- Error handling (1 test)
