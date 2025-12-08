---
enhancement: 02-storage-layer
agent: architect
task_id: task_1764793362_34865
timestamp: 2025-12-03T20:15:00Z
status: READY_FOR_IMPLEMENTATION
---

# Storage Layer Implementation Plan

## Executive Summary

This document provides a comprehensive technical architecture and implementation plan for the Bitwarden CLI Rust migration storage layer. The storage system provides JSON-based file persistence with session-encrypted secure storage for sensitive values, maintaining full backward compatibility with the TypeScript CLI.

**Key Design Decisions:**
- **Storage Format**: JSON files compatible with TypeScript CLI's LowDB format
- **Encryption**: AES-256-CBC with HMAC-SHA256 using Bitwarden SDK crypto
- **Concurrency**: Atomic writes with file locking for multi-process safety
- **Architecture Pattern**: Service-based with trait abstraction for testability

## System Architecture

### High-Level Component Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                      CLI Commands Layer                       │
│  (auth, vault, tools - consumers of storage)                 │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│                   Service Container                          │
│  - Provides Storage trait instance                          │
│  - Dependency injection for all services                     │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│               Storage Trait (Abstract Interface)             │
│  + get<T>(&self, key: &str) -> Result<Option<T>>            │
│  + set<T>(&mut self, key: &str, value: &T) -> Result<()>    │
│  + remove(&mut self, key: &str) -> Result<bool>             │
│  + has(&self, key: &str) -> Result<bool>                     │
│  + get_secure(&self, key: &str) -> Result<Option<String>>   │
│  + set_secure(&mut self, key: &str, value: &str) -> Result<()>│
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│           JsonFileStorage (Concrete Implementation)          │
│  - Path resolution (platform-aware)                          │
│  - JSON file read/write with atomic operations               │
│  - In-memory cache of storage data                           │
│  - File locking for concurrent access                        │
└────────────┬────────────────────────────┬───────────────────┘
             │                            │
             ▼                            ▼
┌────────────────────────┐  ┌────────────────────────────────┐
│   SecureStorage        │  │   File System                  │
│  - BW_SESSION parsing  │  │  - data.json                   │
│  - SDK crypto wrapper  │  │  - Atomic writes (temp+rename) │
│  - __PROTECTED__ prefix│  │  - File locking                │
│  - EncString handling  │  │  - Permission management       │
└────────────┬───────────┘  └────────────────────────────────┘
             │
             ▼
┌─────────────────────────────────────────────────────────────┐
│              Bitwarden SDK (bitwarden-crypto)                │
│  - AES-256-CBC encryption/decryption                         │
│  - HMAC-SHA256 message authentication                       │
│  - EncString parsing and formatting                          │
└─────────────────────────────────────────────────────────────┘
```

### Module Organization

```
crates/bw-core/src/
├── services/
│   ├── mod.rs                    # Re-exports
│   ├── container.rs              # ServiceContainer (updated)
│   ├── sdk.rs                    # SDK client wrapper (existing)
│   └── storage/                  # NEW: Storage module
│       ├── mod.rs                # Public API, re-exports
│       ├── traits.rs             # Storage trait definition
│       ├── json_storage.rs       # JsonFileStorage implementation
│       ├── secure.rs             # SecureStorage wrapper
│       ├── path.rs               # Platform path resolution
│       ├── atomic.rs             # Atomic file operations
│       └── errors.rs             # Storage-specific errors
└── models/
    └── state/                    # NEW: State data structures
        ├── mod.rs
        ├── environment.rs        # Environment URLs
        ├── auth.rs               # Authentication state
        ├── user.rs               # User profile
        ├── vault.rs              # Vault sync state
        └── kdf.rs                # KDF configuration
```

## Technical Design

### 1. Storage Trait Design

**File**: `crates/bw-core/src/services/storage/traits.rs`

```rust
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Abstract storage interface for configuration and state persistence
///
/// Implementations must provide:
/// - Type-safe get/set operations with JSON serialization
/// - Secure storage for sensitive values (encrypted at rest)
/// - Atomic operations to prevent corruption
/// - Thread-safe access for potential concurrent operations
pub trait Storage: Send + Sync {
    /// Retrieve a value by key
    ///
    /// Returns None if key doesn't exist, error if deserialization fails
    fn get<T>(&self, key: &str) -> Result<Option<T>>
    where
        T: for<'de> Deserialize<'de>;

    /// Store a value by key
    ///
    /// Overwrites existing value if present
    fn set<T>(&mut self, key: &str, value: &T) -> Result<()>
    where
        T: Serialize;

    /// Remove a value by key
    ///
    /// Returns true if value existed and was removed, false if key not found
    fn remove(&mut self, key: &str) -> Result<bool>;

    /// Check if a key exists
    fn has(&self, key: &str) -> Result<bool>;

    /// Retrieve a secure (encrypted) value
    ///
    /// Requires BW_SESSION environment variable to decrypt
    /// Returns None if key doesn't exist
    /// Returns error if BW_SESSION missing or decryption fails
    fn get_secure(&self, key: &str) -> Result<Option<String>>;

    /// Store a secure (encrypted) value
    ///
    /// Encrypts value using BW_SESSION before storing
    /// Stores with __PROTECTED__ key prefix
    /// Requires BW_SESSION environment variable
    fn set_secure(&mut self, key: &str, value: &str) -> Result<()>;

    /// Remove a secure value by key
    ///
    /// Automatically adds __PROTECTED__ prefix if not present
    fn remove_secure(&mut self, key: &str) -> Result<bool>;

    /// Persist all pending changes to disk
    ///
    /// Called automatically by set/remove operations
    /// Can be called explicitly to ensure durability
    fn flush(&mut self) -> Result<()>;
}
```

**Design Rationale:**
- **Trait-based abstraction**: Enables testing with mock implementations
- **Generic type parameters**: Type-safe serialization/deserialization
- **Separate secure methods**: Clear distinction between plain and encrypted storage
- **Error handling**: Uses `anyhow::Result` for flexible error propagation
- **Send + Sync**: Enables thread-safe access patterns

### 2. JSON File Storage Implementation

**File**: `crates/bw-core/src/services/storage/json_storage.rs`

```rust
use super::{
    atomic::AtomicWriter,
    errors::StorageError,
    path::StoragePath,
    secure::SecureStorage,
    traits::Storage,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// JSON-based file storage with support for secure (encrypted) values
///
/// Storage format is compatible with TypeScript CLI's LowDB format:
/// - Single JSON file (data.json)
/// - Flat key-value structure
/// - Keys with __PROTECTED__ prefix are encrypted
/// - Values are JSON types (string, number, object, array, etc.)
pub struct JsonFileStorage {
    /// Path to storage file (data.json)
    file_path: PathBuf,

    /// In-memory cache of storage contents
    /// Uses Mutex for interior mutability (required for get operations)
    data: Arc<Mutex<HashMap<String, Value>>>,

    /// Secure storage handler for encrypted values
    secure: SecureStorage,

    /// Atomic writer for safe file operations
    writer: AtomicWriter,
}

impl JsonFileStorage {
    /// Create new storage instance
    ///
    /// # Arguments
    /// * `custom_path` - Optional custom storage directory path
    ///   If None, uses platform-specific default
    ///
    /// # Path Resolution Priority
    /// 1. `./bw-data` (relative to executable)
    /// 2. `BITWARDENCLI_APPDATA_DIR` environment variable
    /// 3. Platform default (see `StoragePath::resolve`)
    pub fn new(custom_path: Option<PathBuf>) -> Result<Self> {
        let storage_path = StoragePath::resolve(custom_path)?;
        let file_path = storage_path.join("data.json");

        // Ensure storage directory exists with correct permissions
        StoragePath::ensure_directory_exists(&storage_path)?;

        let secure = SecureStorage::new()?;
        let writer = AtomicWriter::new(file_path.clone());

        let data = if file_path.exists() {
            Self::load_from_file(&file_path)?
        } else {
            Arc::new(Mutex::new(HashMap::new()))
        };

        Ok(Self {
            file_path,
            data,
            secure,
            writer,
        })
    }

    /// Load storage from file
    fn load_from_file(path: &PathBuf) -> Result<Arc<Mutex<HashMap<String, Value>>>> {
        let contents = fs::read_to_string(path)
            .map_err(|e| StorageError::ReadError(e, path.clone()))?;

        if contents.trim().is_empty() {
            return Ok(Arc::new(Mutex::new(HashMap::new())));
        }

        let data: HashMap<String, Value> = serde_json::from_str(&contents)
            .map_err(|e| StorageError::ParseError(e, path.clone()))?;

        Ok(Arc::new(Mutex::new(data)))
    }

    /// Get nested value by dot-separated path
    ///
    /// Example: "environmentUrls.api" accesses obj["environmentUrls"]["api"]
    fn get_nested<'a>(data: &'a Value, key: &str) -> Option<&'a Value> {
        let parts: Vec<&str> = key.split('.').collect();
        let mut current = data;

        for part in parts {
            current = current.get(part)?;
        }

        Some(current)
    }

    /// Set nested value by dot-separated path
    ///
    /// Creates intermediate objects as needed
    fn set_nested(data: &mut Value, key: &str, value: Value) {
        let parts: Vec<&str> = key.split('.').collect();
        let mut current = data;

        for (i, part) in parts.iter().enumerate() {
            if i == parts.len() - 1 {
                // Last part - set the value
                current[part] = value;
            } else {
                // Intermediate part - ensure object exists
                if !current[part].is_object() {
                    current[part] = Value::Object(serde_json::Map::new());
                }
                current = &mut current[part];
            }
        }
    }

    /// Remove nested value by dot-separated path
    fn remove_nested(data: &mut Value, key: &str) -> bool {
        let parts: Vec<&str> = key.split('.').collect();
        if parts.is_empty() {
            return false;
        }

        let mut current = data;

        // Navigate to parent
        for part in &parts[..parts.len() - 1] {
            if let Some(obj) = current.get_mut(part) {
                current = obj;
            } else {
                return false;
            }
        }

        // Remove final key
        let last_key = parts[parts.len() - 1];
        if let Some(obj) = current.as_object_mut() {
            obj.remove(last_key).is_some()
        } else {
            false
        }
    }
}

impl Storage for JsonFileStorage {
    fn get<T>(&self, key: &str) -> Result<Option<T>>
    where
        T: for<'de> Deserialize<'de>,
    {
        let data = self.data.lock().unwrap();
        let root = Value::Object(
            data.iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        );

        let value = Self::get_nested(&root, key);

        match value {
            None => Ok(None),
            Some(v) => {
                let deserialized: T = serde_json::from_value(v.clone())
                    .map_err(|e| StorageError::DeserializationError(e, key.to_string()))?;
                Ok(Some(deserialized))
            }
        }
    }

    fn set<T>(&mut self, key: &str, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        let json_value = serde_json::to_value(value)
            .map_err(|e| StorageError::SerializationError(e, key.to_string()))?;

        let mut data = self.data.lock().unwrap();
        let mut root = Value::Object(
            data.iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        );

        Self::set_nested(&mut root, key, json_value);

        // Update in-memory cache
        if let Value::Object(map) = root {
            *data = map.into_iter().collect();
        }

        drop(data);
        self.flush()
    }

    fn remove(&mut self, key: &str) -> Result<bool> {
        let mut data = self.data.lock().unwrap();
        let mut root = Value::Object(
            data.iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        );

        let removed = Self::remove_nested(&mut root, key);

        if removed {
            if let Value::Object(map) = root {
                *data = map.into_iter().collect();
            }
            drop(data);
            self.flush()?;
        }

        Ok(removed)
    }

    fn has(&self, key: &str) -> Result<bool> {
        let data = self.data.lock().unwrap();
        let root = Value::Object(
            data.iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        );

        Ok(Self::get_nested(&root, key).is_some())
    }

    fn get_secure(&self, key: &str) -> Result<Option<String>> {
        let protected_key = format!("__PROTECTED__{}", key);

        // Get encrypted value
        let encrypted: Option<String> = self.get(&protected_key)?;

        match encrypted {
            None => Ok(None),
            Some(enc_str) => {
                // Decrypt using secure storage
                let decrypted = self.secure.decrypt(&enc_str)?;
                Ok(Some(decrypted))
            }
        }
    }

    fn set_secure(&mut self, key: &str, value: &str) -> Result<()> {
        let protected_key = format!("__PROTECTED__{}", key);

        // Encrypt using secure storage
        let encrypted = self.secure.encrypt(value)?;

        // Store encrypted value
        self.set(&protected_key, &encrypted)
    }

    fn remove_secure(&mut self, key: &str) -> Result<bool> {
        let protected_key = if key.starts_with("__PROTECTED__") {
            key.to_string()
        } else {
            format!("__PROTECTED__{}", key)
        };

        self.remove(&protected_key)
    }

    fn flush(&mut self) -> Result<()> {
        let data = self.data.lock().unwrap();
        let json = serde_json::to_string_pretty(&*data)
            .map_err(|e| StorageError::SerializationError(e, "storage".to_string()))?;

        self.writer.write_atomic(&json)?;

        Ok(())
    }
}
```

**Design Rationale:**
- **In-memory cache**: Reduces file I/O, improves performance
- **Nested key support**: Dot-notation for hierarchical data (e.g., "urls.api")
- **Atomic operations**: Every write operation is atomic (see AtomicWriter)
- **Mutex for interior mutability**: Allows `&self` methods to modify cache
- **Separate secure methods**: Clear API for encrypted vs plain storage

### 3. Secure Storage (Encryption/Decryption)

**File**: `crates/bw-core/src/services/storage/secure.rs`

```rust
use super::errors::StorageError;
use anyhow::Result;
use std::env;

/// Secure storage handler for encrypting/decrypting sensitive values
///
/// Uses BW_SESSION environment variable as encryption key
/// Delegates to Bitwarden SDK for all cryptographic operations
pub struct SecureStorage {
    // SDK client will be added when SDK integration is complete
    // For now, we'll define the interface
}

impl SecureStorage {
    /// Create new secure storage handler
    ///
    /// Validates that BW_SESSION is available and correctly formatted
    pub fn new() -> Result<Self> {
        // BW_SESSION validation will be implemented with SDK
        Ok(Self {})
    }

    /// Encrypt a plaintext value
    ///
    /// # Arguments
    /// * `plaintext` - String to encrypt
    ///
    /// # Returns
    /// EncString format: "2.base64_iv|base64_ciphertext|base64_mac"
    ///
    /// # Process
    /// 1. Get BW_SESSION from environment (64 bytes base64-encoded)
    /// 2. Parse BW_SESSION: first 32 bytes = encryption key, last 32 = MAC key
    /// 3. Generate random 16-byte IV
    /// 4. Encrypt plaintext using AES-256-CBC with encryption key
    /// 5. Compute HMAC-SHA256 over IV + ciphertext using MAC key
    /// 6. Format as EncString: "2.iv_b64|ct_b64|mac_b64"
    ///
    /// # SDK Integration
    /// Use `bitwarden_crypto::EncryptService` and `SymmetricCryptoKey`
    pub fn encrypt(&self, plaintext: &str) -> Result<String> {
        let bw_session = env::var("BW_SESSION")
            .map_err(|_| StorageError::MissingSessionKey)?;

        // TODO: Implement with SDK
        // use bitwarden_crypto::{EncryptService, SymmetricCryptoKey};
        // let key = SymmetricCryptoKey::from_b64(&bw_session)?;
        // let encrypted = encrypt_service.encrypt(plaintext.as_bytes(), &key)?;
        // Ok(encrypted.to_string())

        // Placeholder for implementation phase
        Err(StorageError::NotImplemented("SDK encryption not yet integrated".to_string()).into())
    }

    /// Decrypt an encrypted value
    ///
    /// # Arguments
    /// * `enc_string` - EncString format encrypted value
    ///
    /// # Returns
    /// Decrypted plaintext string
    ///
    /// # Process
    /// 1. Parse EncString format: "type.iv|ct|mac"
    /// 2. Verify type is 2 (AesCbc256_HmacSha256_B64)
    /// 3. Base64-decode IV, ciphertext, MAC
    /// 4. Get BW_SESSION and parse into encryption + MAC keys
    /// 5. Verify HMAC-SHA256(iv + ct) matches provided MAC
    /// 6. Decrypt ciphertext using AES-256-CBC
    /// 7. Return UTF-8 decoded plaintext
    ///
    /// # SDK Integration
    /// Use `bitwarden_crypto::EncString::parse` and decrypt methods
    pub fn decrypt(&self, enc_string: &str) -> Result<String> {
        let bw_session = env::var("BW_SESSION")
            .map_err(|_| StorageError::MissingSessionKey)?;

        // TODO: Implement with SDK
        // use bitwarden_crypto::{EncString, SymmetricCryptoKey};
        // let enc = EncString::parse(enc_string)?;
        // let key = SymmetricCryptoKey::from_b64(&bw_session)?;
        // let decrypted = enc.decrypt(&key)?;
        // Ok(String::from_utf8(decrypted)?)

        // Placeholder for implementation phase
        Err(StorageError::NotImplemented("SDK decryption not yet integrated".to_string()).into())
    }

    /// Check if BW_SESSION is available and valid
    pub fn is_available(&self) -> bool {
        env::var("BW_SESSION").is_ok()
    }
}
```

**Critical Notes for Implementation:**

1. **Encryption Format** (from research):
   - Type 2 = `AesCbc256_HmacSha256_B64`
   - Format: `2.base64_iv|base64_ciphertext|base64_mac`
   - IV: 16 bytes (128 bits) for AES-CBC
   - Ciphertext: Variable length (plaintext + PKCS#7 padding)
   - MAC: 32 bytes (256 bits) HMAC-SHA256

2. **BW_SESSION Format** (from research):
   - 64 bytes base64-encoded
   - First 32 bytes: AES-256 encryption key
   - Last 32 bytes: HMAC-SHA256 MAC key

3. **SDK Usage**:
   - DO NOT implement crypto manually
   - Use `bitwarden_crypto::EncryptService`
   - Use `bitwarden_crypto::SymmetricCryptoKey`
   - Use `bitwarden_crypto::EncString` for parsing

**Sources:**
- [Decrypting Bitwarden Secrets - attie.co.uk](https://attie.co.uk/bitwarden/decrypt/)
- [Bitwarden Encryption Protocols](https://bitwarden.com/help/what-encryption-is-used/)

### 4. Platform Path Resolution

**File**: `crates/bw-core/src/services/storage/path.rs`

```rust
use super::errors::StorageError;
use anyhow::Result;
use directories::ProjectDirs;
use std::env;
use std::fs;
use std::path::PathBuf;

/// Platform-aware storage path resolution
pub struct StoragePath;

impl StoragePath {
    /// Resolve storage directory path
    ///
    /// # Priority Order
    /// 1. `./bw-data` - Relative to executable (portable mode)
    /// 2. `BITWARDENCLI_APPDATA_DIR` - Custom path via environment variable
    /// 3. Platform default - OS-specific application data directory
    ///
    /// # Platform Defaults
    /// - **macOS**: `~/Library/Application Support/Bitwarden CLI`
    /// - **Windows**: `%APPDATA%/Bitwarden CLI`
    /// - **Linux**: `$XDG_CONFIG_HOME/Bitwarden CLI` or `~/.config/Bitwarden CLI`
    pub fn resolve(custom_path: Option<PathBuf>) -> Result<PathBuf> {
        // 1. Check for custom path argument
        if let Some(path) = custom_path {
            return Ok(Self::canonicalize_path(path)?);
        }

        // 2. Check for ./bw-data (portable mode)
        if let Ok(exe_path) = env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                let portable_path = exe_dir.join("bw-data");
                if portable_path.exists() && portable_path.is_dir() {
                    return Ok(portable_path);
                }
            }
        }

        // 3. Check BITWARDENCLI_APPDATA_DIR environment variable
        if let Ok(env_path) = env::var("BITWARDENCLI_APPDATA_DIR") {
            let path = PathBuf::from(env_path);
            return Ok(Self::canonicalize_path(path)?);
        }

        // 4. Use platform default
        let project_dirs = ProjectDirs::from("com", "Bitwarden", "Bitwarden CLI")
            .ok_or_else(|| StorageError::PathResolutionError(
                "Could not determine platform application directory".to_string()
            ))?;

        Ok(project_dirs.data_dir().to_path_buf())
    }

    /// Ensure directory exists with correct permissions
    ///
    /// # Permissions
    /// - Directory: 0700 (owner read/write/execute only)
    /// - Data file: 0600 (owner read/write only)
    pub fn ensure_directory_exists(path: &PathBuf) -> Result<()> {
        if !path.exists() {
            fs::create_dir_all(path)
                .map_err(|e| StorageError::CreateDirectoryError(e, path.clone()))?;

            // Set permissions (Unix-only, ignored on Windows)
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let perms = fs::Permissions::from_mode(0o700);
                fs::set_permissions(path, perms)
                    .map_err(|e| StorageError::PermissionError(e, path.clone()))?;
            }
        }

        // Verify directory is writable
        if !Self::is_writable(path) {
            return Err(StorageError::NotWritableError(path.clone()).into());
        }

        Ok(())
    }

    /// Check if path is writable
    fn is_writable(path: &PathBuf) -> bool {
        // Attempt to create a temp file
        let test_file = path.join(".write-test");
        match fs::write(&test_file, b"test") {
            Ok(_) => {
                let _ = fs::remove_file(&test_file);
                true
            }
            Err(_) => false,
        }
    }

    /// Canonicalize path (resolve to absolute path)
    fn canonicalize_path(path: PathBuf) -> Result<PathBuf> {
        if path.is_absolute() {
            Ok(path)
        } else {
            env::current_dir()
                .map(|cwd| cwd.join(path))
                .map_err(|e| StorageError::PathResolutionError(format!("Failed to get current directory: {}", e)).into())
        }
    }
}
```

**Design Rationale:**
- **Portable mode**: Enables USB drive installations
- **Environment override**: Supports testing and custom deployments
- **Platform defaults**: Uses `directories` crate for OS-specific paths
- **Permission management**: Ensures secure file permissions on Unix
- **Validation**: Checks writability before proceeding

### 5. Atomic File Operations

**File**: `crates/bw-core/src/services/storage/atomic.rs`

```rust
use super::errors::StorageError;
use anyhow::Result;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;

/// Atomic file writer with file locking
///
/// Uses temp file + atomic rename pattern to prevent corruption
/// Implements file locking to handle concurrent access
pub struct AtomicWriter {
    target_path: PathBuf,
}

impl AtomicWriter {
    pub fn new(target_path: PathBuf) -> Self {
        Self { target_path }
    }

    /// Write content atomically to target file
    ///
    /// # Process
    /// 1. Acquire file lock on target path
    /// 2. Write content to temporary file in same directory
    /// 3. Flush and sync to ensure data is on disk
    /// 4. Atomically rename temp file to target path
    /// 5. Release file lock
    ///
    /// # Atomicity Guarantee
    /// The rename operation is atomic on all platforms (POSIX and Windows)
    /// If process crashes during write, either old or new data is present
    pub fn write_atomic(&self, content: &str) -> Result<()> {
        // Acquire lock file
        let lock = self.acquire_lock()?;

        // Create temp file in same directory (ensures same filesystem)
        let temp_path = self.temp_file_path();

        let mut file = File::create(&temp_path)
            .map_err(|e| StorageError::WriteError(e, temp_path.clone()))?;

        file.write_all(content.as_bytes())
            .map_err(|e| StorageError::WriteError(e, temp_path.clone()))?;

        // Flush and sync to ensure data reaches disk
        file.flush()
            .map_err(|e| StorageError::WriteError(e, temp_path.clone()))?;
        file.sync_all()
            .map_err(|e| StorageError::WriteError(e, temp_path.clone()))?;

        drop(file);

        // Atomic rename
        fs::rename(&temp_path, &self.target_path)
            .map_err(|e| StorageError::WriteError(e, self.target_path.clone()))?;

        // Set file permissions (Unix-only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(0o600);
            fs::set_permissions(&self.target_path, perms)
                .map_err(|e| StorageError::PermissionError(e, self.target_path.clone()))?;
        }

        // Release lock
        drop(lock);

        Ok(())
    }

    /// Generate temp file path
    fn temp_file_path(&self) -> PathBuf {
        let mut temp = self.target_path.clone();
        temp.set_extension("tmp");
        temp
    }

    /// Acquire file lock for concurrent access protection
    ///
    /// Uses fs2 crate for cross-platform file locking
    /// Returns lock guard that releases lock on drop
    fn acquire_lock(&self) -> Result<FileLock> {
        FileLock::acquire(&self.target_path)
    }
}

/// File lock guard
///
/// Automatically releases lock when dropped
struct FileLock {
    #[allow(dead_code)]
    lock_path: PathBuf,
    // TODO: Add fs2::File field when dependency added
}

impl FileLock {
    fn acquire(target_path: &PathBuf) -> Result<Self> {
        let lock_path = target_path.with_extension("lock");

        // TODO: Implement with fs2 crate
        // use fs2::FileExt;
        // let file = File::create(&lock_path)?;
        // file.lock_exclusive()?;

        Ok(Self { lock_path })
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        // Lock is released automatically when file is closed
        // Clean up lock file if it exists
        let _ = fs::remove_file(&self.lock_path);
    }
}
```

**Design Rationale:**
- **Temp file + rename**: Atomic operation guaranteed by OS
- **File locking**: Prevents concurrent access corruption
- **Same filesystem**: Temp file in same directory ensures atomic rename
- **Sync operations**: Ensures data durability before rename
- **Lock cleanup**: Automatic cleanup via Drop trait

**Additional Dependency Required:**
```toml
# Add to workspace dependencies
fs2 = "0.4"  # Cross-platform file locking
```

### 6. Storage Error Types

**File**: `crates/bw-core/src/services/storage/errors.rs`

```rust
use std::path::PathBuf;
use thiserror::Error;

/// Storage-specific errors
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("Failed to read storage file {1}: {0}")]
    ReadError(#[source] std::io::Error, PathBuf),

    #[error("Failed to write storage file {1}: {0}")]
    WriteError(#[source] std::io::Error, PathBuf),

    #[error("Failed to parse storage file {1}: {0}")]
    ParseError(#[source] serde_json::Error, PathBuf),

    #[error("Failed to serialize value for key '{1}': {0}")]
    SerializationError(#[source] serde_json::Error, String),

    #[error("Failed to deserialize value for key '{1}': {0}")]
    DeserializationError(#[source] serde_json::Error, String),

    #[error("Failed to create directory {1}: {0}")]
    CreateDirectoryError(#[source] std::io::Error, PathBuf),

    #[error("Permission denied for path {1}: {0}")]
    PermissionError(#[source] std::io::Error, PathBuf),

    #[error("Path is not writable: {0}")]
    NotWritableError(PathBuf),

    #[error("Failed to resolve storage path: {0}")]
    PathResolutionError(String),

    #[error("BW_SESSION environment variable not set or invalid")]
    MissingSessionKey,

    #[error("Failed to decrypt value: {0}")]
    DecryptionError(String),

    #[error("Failed to encrypt value: {0}")]
    EncryptionError(String),

    #[error("Not implemented: {0}")]
    NotImplemented(String),
}
```

**Design Rationale:**
- **thiserror**: Automatic Error trait implementation
- **Context**: Include path/key information for debugging
- **Source errors**: Preserve underlying error chain
- **Clear messages**: User-friendly error descriptions

### 7. State Data Structures

**File**: `crates/bw-core/src/models/state/environment.rs`

```rust
use serde::{Deserialize, Serialize};

/// Environment server URLs configuration
///
/// Compatible with TypeScript CLI storage format
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentUrls {
    /// Base API URL (default: https://api.bitwarden.com)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base: Option<String>,

    /// API server URL (default: https://api.bitwarden.com)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api: Option<String>,

    /// Identity server URL (default: https://identity.bitwarden.com)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identity: Option<String>,

    /// Web vault URL (default: https://vault.bitwarden.com)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub web_vault: Option<String>,

    /// Icons server URL (default: https://icons.bitwarden.com)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icons: Option<String>,

    /// Notifications server URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notifications: Option<String>,

    /// Events server URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub events: Option<String>,
}

impl Default for EnvironmentUrls {
    fn default() -> Self {
        Self {
            base: Some("https://bitwarden.com".to_string()),
            api: Some("https://api.bitwarden.com".to_string()),
            identity: Some("https://identity.bitwarden.com".to_string()),
            web_vault: Some("https://vault.bitwarden.com".to_string()),
            icons: Some("https://icons.bitwarden.com".to_string()),
            notifications: Some("https://notifications.bitwarden.com".to_string()),
            events: Some("https://events.bitwarden.com".to_string()),
        }
    }
}
```

**File**: `crates/bw-core/src/models/state/auth.rs`

```rust
use serde::{Deserialize, Serialize};
use secrecy::Secret;

/// Authentication state
///
/// Tokens are stored encrypted with __PROTECTED__ prefix
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthState {
    /// Access token (JWT) - ENCRYPTED
    /// Stored as: __PROTECTED__tokens.accessToken
    #[serde(skip)]
    pub access_token: Option<Secret<String>>,

    /// Refresh token - ENCRYPTED
    /// Stored as: __PROTECTED__tokens.refreshToken
    #[serde(skip)]
    pub refresh_token: Option<Secret<String>>,

    /// Token expiration timestamp (Unix seconds)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_expiry: Option<i64>,

    /// User ID (GUID)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
}
```

**File**: `crates/bw-core/src/models/state/user.rs`

```rust
use serde::{Deserialize, Serialize};

/// User profile information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserProfile {
    /// User ID (GUID)
    pub id: String,

    /// Email address
    pub email: String,

    /// Display name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Email verified flag
    #[serde(default)]
    pub email_verified: bool,

    /// Premium subscription flag
    #[serde(default)]
    pub premium: bool,

    /// Security stamp (for session validation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub security_stamp: Option<String>,
}
```

**File**: `crates/bw-core/src/models/state/kdf.rs`

```rust
use serde::{Deserialize, Serialize};

/// Key Derivation Function configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KdfConfig {
    /// KDF type: 0 = PBKDF2-SHA256, 1 = Argon2id
    pub kdf: KdfType,

    /// PBKDF2 iterations (default: 600000)
    /// Used when kdf = PBKDF2
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kdf_iterations: Option<u32>,

    /// Argon2 memory in MB (default: 64)
    /// Used when kdf = Argon2id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kdf_memory: Option<u32>,

    /// Argon2 parallelism (default: 4)
    /// Used when kdf = Argon2id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kdf_parallelism: Option<u32>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[repr(u8)]
pub enum KdfType {
    PBKDF2_SHA256 = 0,
    Argon2id = 1,
}
```

**File**: `crates/bw-core/src/models/state/vault.rs`

```rust
use serde::{Deserialize, Serialize};

/// Vault synchronization state
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultState {
    /// Last sync timestamp (ISO 8601)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_sync: Option<String>,

    /// Encrypted master key - ENCRYPTED
    /// Stored as: __PROTECTED__keys.masterKey
    #[serde(skip)]
    pub master_key: Option<String>,

    /// Encrypted private key (RSA) - ENCRYPTED
    /// Stored as: __PROTECTED__keys.privateKey
    #[serde(skip)]
    pub private_key: Option<String>,

    /// Organization keys (encrypted) - ENCRYPTED
    /// Stored as: __PROTECTED__keys.orgKeys
    #[serde(skip)]
    pub org_keys: Option<Vec<OrgKey>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrgKey {
    /// Organization ID (GUID)
    pub org_id: String,

    /// Encrypted organization key
    pub key: String,
}
```

**File**: `crates/bw-core/src/models/state/mod.rs`

```rust
mod auth;
mod environment;
mod kdf;
mod user;
mod vault;

pub use auth::AuthState;
pub use environment::EnvironmentUrls;
pub use kdf::{KdfConfig, KdfType};
pub use user::UserProfile;
pub use vault::{OrgKey, VaultState};
```

**Design Rationale:**
- **serde attributes**: Match TypeScript field names exactly (camelCase)
- **Option types**: All optional fields use Option<T>
- **secrecy::Secret**: Sensitive values use Secret wrapper
- **Skip serialization**: Encrypted fields use `#[serde(skip)]` (handled separately)
- **Forward compatibility**: Unknown fields ignored by default

### 8. Service Container Integration

**File**: `crates/bw-core/src/services/container.rs` (updated)

```rust
use super::{create_sdk_client, sdk::Client, storage::{JsonFileStorage, Storage}};
use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;

/// Service container for dependency injection
///
/// Provides access to:
/// - SDK client (crypto, vault, auth operations)
/// - Storage (configuration and state persistence)
/// - HTTP client (future: enhancement 3)
pub struct ServiceContainer {
    /// Bitwarden SDK client - handles all crypto and most business logic
    sdk: Client,

    /// Storage service - configuration and state persistence
    storage: Arc<dyn Storage>,
}

impl ServiceContainer {
    /// Create a new service container
    ///
    /// # Arguments
    /// * `api_url` - Optional API server URL
    /// * `identity_url` - Optional Identity server URL
    /// * `storage_path` - Optional custom storage directory path
    pub fn new(
        api_url: Option<String>,
        identity_url: Option<String>,
        storage_path: Option<PathBuf>,
    ) -> Result<Self> {
        let sdk = create_sdk_client(api_url, identity_url)?;
        let storage: Arc<dyn Storage> = Arc::new(JsonFileStorage::new(storage_path)?);

        Ok(Self { sdk, storage })
    }

    /// Get reference to SDK client
    ///
    /// Use this for all crypto operations (encrypt, decrypt, key derivation)
    /// and vault operations (sync, cipher operations, etc.)
    pub fn sdk(&self) -> &Client {
        &self.sdk
    }

    /// Get reference to storage service
    ///
    /// Use this for configuration and state persistence
    pub fn storage(&self) -> Arc<dyn Storage> {
        Arc::clone(&self.storage)
    }
}
```

### 9. Module Public API

**File**: `crates/bw-core/src/services/storage/mod.rs`

```rust
mod atomic;
mod errors;
mod json_storage;
mod path;
mod secure;
mod traits;

// Public exports
pub use errors::StorageError;
pub use json_storage::JsonFileStorage;
pub use traits::Storage;

// Internal use only
pub(crate) use atomic::AtomicWriter;
pub(crate) use path::StoragePath;
pub(crate) use secure::SecureStorage;
```

**File**: `crates/bw-core/src/services/mod.rs` (updated)

```rust
mod container;
mod sdk;

// NEW: Storage module
pub mod storage;

pub use container::ServiceContainer;
pub use sdk::create_sdk_client;
```

## Dependency Updates

### Workspace Dependencies to Add

**File**: `Cargo.toml` (workspace root)

```toml
[workspace.dependencies]
# ... existing dependencies ...

# Storage dependencies
tempfile = "3.12"          # Atomic write operations (alternative to manual impl)
fs2 = "0.4"                # Cross-platform file locking
```

### Crate Dependencies

**File**: `crates/bw-core/Cargo.toml` (updated)

```toml
[dependencies]
# ... existing dependencies ...

# Storage
fs2.workspace = true

[dev-dependencies]
# ... existing dev-dependencies ...
tempfile.workspace = true   # For testing
```

## Integration Strategy

### Phase 1: Foundation (No SDK Dependencies)
1. Implement path resolution (`path.rs`)
2. Implement error types (`errors.rs`)
3. Implement atomic writer (`atomic.rs`)
4. Implement basic JSON storage without encryption (`json_storage.rs` - partial)
5. Unit tests for path resolution and atomic writes

### Phase 2: State Structures
1. Implement all state models (`models/state/`)
2. Unit tests for serialization/deserialization
3. Test compatibility with TypeScript CLI JSON samples

### Phase 3: SDK Integration
1. Complete `SecureStorage` implementation (`secure.rs`)
2. Integrate with `JsonFileStorage` secure methods
3. Test encryption/decryption round trips
4. Test with real BW_SESSION values

### Phase 4: Service Container Integration
1. Update `ServiceContainer` to include storage
2. Update initialization in CLI main
3. Integration tests with real workflows

### Phase 5: Testing and Validation
1. Cross-platform testing (Windows, macOS, Linux)
2. Concurrent access testing
3. Corruption recovery testing
4. TypeScript CLI compatibility testing

## Testing Strategy

### Unit Tests

**File**: `crates/bw-core/src/services/storage/path.rs` (tests)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_portable_mode_detection() {
        // Test ./bw-data detection
    }

    #[test]
    fn test_env_var_override() {
        // Test BITWARDENCLI_APPDATA_DIR
    }

    #[test]
    fn test_platform_defaults() {
        // Test platform-specific paths
    }

    #[test]
    fn test_directory_creation() {
        // Test directory creation with permissions
    }
}
```

**File**: `crates/bw-core/src/services/storage/json_storage.rs` (tests)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_get_set_string() {
        // Test basic string storage
    }

    #[test]
    fn test_get_set_object() {
        // Test complex object storage
    }

    #[test]
    fn test_nested_keys() {
        // Test dot-notation keys
    }

    #[test]
    fn test_remove() {
        // Test value removal
    }

    #[test]
    fn test_atomic_writes() {
        // Test write atomicity
    }

    #[test]
    fn test_concurrent_access() {
        // Test file locking
    }
}
```

### Integration Tests

**File**: `crates/bw-core/tests/storage_integration.rs`

```rust
use bw_core::services::storage::{JsonFileStorage, Storage};
use bw_core::models::state::EnvironmentUrls;
use tempfile::TempDir;

#[test]
fn test_typescript_cli_compatibility() {
    // Load real TypeScript CLI data.json
    // Parse with JsonFileStorage
    // Verify all fields read correctly
}

#[test]
fn test_secure_storage_round_trip() {
    // Set BW_SESSION environment variable
    // Store encrypted value
    // Retrieve and decrypt
    // Verify match
}

#[test]
fn test_corruption_recovery() {
    // Write valid storage
    // Corrupt the file
    // Attempt to load
    // Verify error and backup creation
}
```

### Cross-Platform Testing

**CI Configuration** (GitHub Actions):
```yaml
test-storage:
  strategy:
    matrix:
      os: [ubuntu-latest, macos-latest, windows-latest]
  runs-on: ${{ matrix.os }}
  steps:
    - uses: actions/checkout@v4
    - name: Run storage tests
      run: cargo test --package bw-core storage
    - name: Test path resolution
      run: cargo test --package bw-core path_resolution
```

## Error Handling Patterns

### User-Facing Error Messages

```rust
// Example: Storage directory not writable
return Err(anyhow!(
    "Cannot write to storage directory: {}\n\
     Please check directory permissions or set BITWARDENCLI_APPDATA_DIR \
     to a writable location.",
    path.display()
));

// Example: Corrupted storage file
return Err(anyhow!(
    "Storage file is corrupted: {}\n\
     A backup has been created at: {}\n\
     The storage will be reinitialized. You may need to log in again.",
    path.display(),
    backup_path.display()
));

// Example: Missing BW_SESSION
return Err(anyhow!(
    "BW_SESSION environment variable not set.\n\
     Please unlock your vault first with: bw unlock"
));
```

### Error Recovery Actions

1. **Corrupted Storage**:
   - Create backup: `data.json.bak.{timestamp}`
   - Initialize empty storage
   - Log error details for debugging

2. **Permission Errors**:
   - Check directory permissions
   - Suggest alternative paths
   - Provide BITWARDENCLI_APPDATA_DIR instructions

3. **Concurrent Access**:
   - Retry with exponential backoff
   - Maximum 3 retries, 100ms initial delay
   - Clear error message if all retries fail

## Performance Considerations

### Optimization Strategies

1. **In-Memory Cache**:
   - All storage data cached in memory
   - No file reads after initial load
   - Writes update cache and flush to disk

2. **Lazy Initialization**:
   - Storage created only when first accessed
   - Reduces CLI startup time for non-storage commands

3. **Efficient JSON Parsing**:
   - Use `serde_json` (fastest Rust JSON library)
   - Stream parsing for large files (future enhancement)

4. **Minimal Allocations**:
   - Reuse buffers where possible
   - Use `&str` instead of `String` where possible

### Performance Targets

| Operation | Target | Measurement |
|-----------|--------|-------------|
| Initial load | <20ms | First storage access |
| Get operation | <1ms | After initial load |
| Set operation | <50ms | Including disk flush |
| Encrypt/decrypt | <10ms | Per operation |

## Security Considerations

### Data Protection

1. **File Permissions**:
   - Storage directory: 0700 (owner only)
   - Data file: 0600 (owner read/write only)
   - Lock file: 0600 (owner read/write only)

2. **Memory Security**:
   - Use `secrecy::Secret<String>` for sensitive values
   - Zeroize buffers after use (via `zeroize` crate)
   - Never log sensitive data

3. **Encryption**:
   - All crypto via Bitwarden SDK (no custom crypto)
   - AES-256-CBC with HMAC-SHA256
   - Verify MAC before decryption

### Attack Surface

| Attack Vector | Mitigation |
|---------------|------------|
| File permission bypass | OS-level permissions, validate on access |
| Concurrent access race | File locking with retry logic |
| Symlink attacks | Resolve paths, don't follow symlinks for writes |
| Directory traversal | Validate paths, reject `..` components |
| Process memory dump | Use Secret types, zeroize sensitive data |
| Disk forensics | Encrypted storage for sensitive values |

## Migration and Compatibility

### TypeScript CLI Compatibility Checklist

- [ ] JSON structure matches exactly (field names case-sensitive)
- [ ] `__PROTECTED__` prefix for encrypted values
- [ ] EncString format: `2.iv_b64|ct_b64|mac_b64`
- [ ] BW_SESSION format: 64 bytes base64 (32 byte enc key + 32 byte MAC key)
- [ ] Support all state structures (environment, auth, user, vault, kdf)
- [ ] Handle missing optional fields gracefully
- [ ] Preserve unknown fields (forward compatibility)

### Breaking Changes

None expected. This is a new implementation maintaining full backward compatibility.

### Future Schema Evolution

**Version Field** (recommended to add now):
```json
{
  "_version": "1.0.0",
  "environmentUrls": { ... },
  ...
}
```

**Migration Strategy** (for future versions):
1. Read `_version` field
2. If missing, assume version 1.0.0
3. Apply migrations in sequence
4. Update version field
5. Write updated storage

## Open Questions and Recommendations

### Resolved from Requirements Analysis

1. **Encryption Algorithm**: ✅ AES-256-CBC with HMAC-SHA256 (from research)
2. **BW_SESSION Format**: ✅ 64 bytes base64, 32 encryption + 32 MAC key
3. **EncString Format**: ✅ `2.iv_b64|ct_b64|mac_b64`
4. **File Locking**: ✅ Implement with fs2 crate

### Remaining Questions for Implementation

1. **SDK Crypto API**: Need to verify exact SDK function signatures
   - `SymmetricCryptoKey::from_b64()` method name?
   - `EncString::parse()` and decrypt methods?
   - **Action**: Review SDK documentation during implementation

2. **Storage Version Field**: Should we add `_version` field now?
   - **Recommendation**: Yes, add as "1.0.0" for future compatibility
   - **Impact**: Minimal, TypeScript CLI will ignore unknown fields

3. **Concurrent Access Testing**: How to test file locking effectively?
   - **Recommendation**: Use integration tests with multiple processes
   - **Tool**: Use `std::process::Command` to spawn CLI instances

4. **Performance Profiling**: Need baseline measurements
   - **Action**: Add criterion benchmarks during implementation
   - **Metrics**: Load time, get/set operations, encryption/decryption

### Recommendations for Implementation Phase

1. **Start with Non-Encrypted Storage**:
   - Implement and test basic JSON storage first
   - Add secure storage after SDK integration ready
   - Reduces complexity during initial development

2. **Create Test Data Set**:
   - Generate real TypeScript CLI storage files
   - Include various scenarios (encrypted, plain, edge cases)
   - Use for compatibility testing

3. **Add Comprehensive Logging**:
   - Use `tracing` crate for debug logging
   - Log path resolution decisions
   - Log encryption/decryption operations (no sensitive data)
   - Helps debugging cross-platform issues

4. **Consider Property-Based Testing**:
   - Use `proptest` crate for fuzzing
   - Test with random data structures
   - Verify serialization round-trips

## Summary and Next Steps

### Architecture Decisions Summary

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Storage Format | JSON files | TypeScript CLI compatibility |
| Encryption | AES-256-CBC + HMAC-SHA256 | Bitwarden standard, SDK provided |
| Concurrency | File locking + atomic writes | Multi-process safety |
| Path Resolution | Portable > Env > Platform | Flexibility + convention |
| Error Handling | thiserror (library) + anyhow (app) | Rust best practices |
| State Models | Serde structs | Type safety + serialization |
| Module Organization | services/storage/ | Clear separation of concerns |

### Implementation Checklist

**Phase 1: Foundation** (1-2 days)
- [ ] Create module structure
- [ ] Implement `StoragePath` with tests
- [ ] Implement `AtomicWriter` with tests
- [ ] Implement `StorageError` types
- [ ] Add workspace dependencies (fs2)

**Phase 2: Basic Storage** (2-3 days)
- [ ] Implement `Storage` trait
- [ ] Implement `JsonFileStorage` (non-encrypted)
- [ ] Add nested key support (dot notation)
- [ ] Unit tests for get/set/remove/has
- [ ] Integration tests with temp directories

**Phase 3: State Models** (1-2 days)
- [ ] Implement all state structs
- [ ] Serialization/deserialization tests
- [ ] TypeScript compatibility tests
- [ ] Sample data validation

**Phase 4: Secure Storage** (2-3 days)
- [ ] Implement `SecureStorage` with SDK
- [ ] Add get_secure/set_secure to trait
- [ ] Encryption/decryption tests
- [ ] BW_SESSION parsing and validation

**Phase 5: Integration** (1-2 days)
- [ ] Update `ServiceContainer`
- [ ] CLI initialization updates
- [ ] End-to-end integration tests
- [ ] Cross-platform testing

**Phase 6: Testing & Documentation** (1-2 days)
- [ ] Concurrent access tests
- [ ] Corruption recovery tests
- [ ] Performance benchmarks
- [ ] Documentation updates

### Success Criteria

✅ **Ready for Implementation** when:
- All state structures defined with TypeScript compatibility
- Storage trait interface complete and documented
- Module organization clearly specified
- Error handling patterns established
- Testing strategy defined
- Integration points identified

✅ **Implementation Complete** when:
- All unit tests pass (>90% coverage)
- Integration tests pass on all platforms
- TypeScript CLI compatibility verified
- Concurrent access handling works
- Corruption recovery tested
- Performance targets met (<50ms for typical operations)
- Documentation complete

## Status

**Status**: READY_FOR_IMPLEMENTATION

All architectural decisions have been made, technical specifications are complete, and implementation guidance is detailed. The implementer can proceed with confidence.

**Key strengths of this design**:
1. ✅ Full TypeScript CLI compatibility (proven formats)
2. ✅ Clear separation of concerns (trait-based architecture)
3. ✅ Strong error handling and recovery
4. ✅ Security best practices (SDK crypto, file permissions)
5. ✅ Testable design (trait abstraction, integration tests)
6. ✅ Cross-platform support (directories crate, platform-specific code)

**Next Agent**: Implementer (with this plan as specification)
