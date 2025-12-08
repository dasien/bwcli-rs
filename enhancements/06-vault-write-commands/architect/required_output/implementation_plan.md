---
enhancement: 06-vault-write-commands
agent: architect
task_id: task_1764951370_98945
timestamp: 2025-12-05T08:52:00Z
status: READY_FOR_IMPLEMENTATION
---

# Vault Write Commands - Implementation Plan

## Executive Summary

This document provides the technical architecture and implementation plan for vault write operations in the Bitwarden CLI Rust migration. The design follows established patterns from enhancements 1-5 and leverages the existing service container, API client, storage layer, and SDK integration infrastructure.

**Scope**: Create, edit, delete, restore, move vault items and folders
**Integration Points**: Storage layer, API client, SDK encryption, authentication
**Risk Level**: High (destructive operations, data integrity, encryption complexity)
**Estimated Effort**: 10-14 days MVP, 15-21 days complete

## Architecture Overview

### High-Level Component Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     CLI Layer (bw-cli)                      │
│  ┌──────────────┐  ┌──────────────┐  ┌─────────────────┐  │
│  │   Create     │  │     Edit     │  │  Delete/Restore │  │
│  │   Commands   │  │   Commands   │  │    Commands     │  │
│  └──────┬───────┘  └──────┬───────┘  └────────┬────────┘  │
└─────────┼──────────────────┼────────────────────┼───────────┘
          │                  │                    │
          ▼                  ▼                    ▼
┌─────────────────────────────────────────────────────────────┐
│                   Core Layer (bw-core)                       │
│  ┌──────────────────────────────────────────────────────┐  │
│  │              WriteService (NEW)                      │  │
│  │  • Coordinates CRUD operations                       │  │
│  │  • Orchestrates validation → encryption → API        │  │
│  │  • Manages cache updates                             │  │
│  │  • Confirmation prompts for destructive operations   │  │
│  └────┬─────────┬──────────┬─────────────┬──────────────┘  │
│       │         │          │             │                  │
│  ┌────▼──────┐ ┌▼─────────▼───┐  ┌──────▼──────┐          │
│  │Validation │ │CipherService │  │Confirmation │          │
│  │Service    │ │• encrypt()   │  │Service      │          │
│  │(NEW)      │ │• decrypt()   │  │(NEW)        │          │
│  └───────────┘ └──────────────┘  └─────────────┘          │
│                                                              │
│  ┌─────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │ API Client  │  │   Storage    │  │ SDK Client   │     │
│  │ (existing)  │  │  (existing)  │  │  (existing)  │     │
│  └─────────────┘  └──────────────┘  └──────────────┘     │
└─────────────────────────────────────────────────────────────┘
          │                  │                    │
          ▼                  ▼                    ▼
    Bitwarden API      Local JSON Cache      SDK Encryption
```

### Design Principles Applied

1. **Consistency with Existing Patterns**: Follow established service-oriented architecture from enhancements 1-5
2. **Separation of Concerns**: Validation → Encryption → API → Cache as distinct phases
3. **Fail-Fast Validation**: Validate inputs before expensive encryption or API calls
4. **Pessimistic Cache Updates**: Only update cache after confirmed API success
5. **SDK-Only Cryptography**: All encryption through Bitwarden SDK (no custom crypto)
6. **User-Friendly Errors**: Domain-specific error types with actionable messages
7. **Atomic Operations**: Cache updates use atomic file writes for consistency
8. **Safety First**: Confirmation prompts for destructive operations

## Module Organization

### New Files to Create

```
crates/bw-core/src/
├── services/vault/
│   ├── write_service.rs              # NEW: CRUD orchestration
│   ├── validation_service.rs         # NEW: Input validation
│   └── confirmation_service.rs       # NEW: Confirmation prompts
├── models/vault/
│   ├── cipher_request.rs             # NEW: API request models
│   └── validation_error.rs           # NEW: Validation error types

crates/bw-cli/src/
├── commands/vault/
│   ├── create.rs                     # NEW: Create commands
│   ├── edit.rs                       # NEW: Edit commands
│   ├── delete.rs                     # NEW: Delete commands
│   └── restore.rs                    # NEW: Restore/move commands
```

### Files to Modify

```
crates/bw-core/src/
├── services/vault/
│   ├── cipher_service.rs             # EXTEND: Add encrypt_cipher()
│   ├── mod.rs                        # EXTEND: Export new services
│   └── errors.rs                     # EXTEND: Add write operation errors
├── models/vault/
│   ├── cipher.rs                     # EXTEND: Add builder patterns
│   └── mod.rs                        # EXTEND: Export new models

crates/bw-cli/src/
├── commands/vault.rs                 # EXTEND: Add write subcommands
└── main.rs                           # EXTEND: Wire up new commands
```

## Core Service Design

### 1. WriteService - CRUD Operations Coordinator

**Purpose**: Orchestrate create, update, delete operations with validation, encryption, and cache management.

**Location**: `crates/bw-core/src/services/vault/write_service.rs`

**Dependencies**:
- `ApiClient` - API operations
- `Storage` - Cache updates
- `CipherService` - Encryption/decryption
- `ValidationService` - Input validation
- `ConfirmationService` - User confirmations

**Interface Design**:

```rust
pub struct WriteService {
    api_client: Arc<dyn ApiClient>,
    storage: Arc<Mutex<dyn Storage>>,
    cipher_service: Arc<CipherService>,
    validation_service: Arc<ValidationService>,
    confirmation_service: Arc<ConfirmationService>,
}

impl WriteService {
    /// Create new cipher (item)
    pub async fn create_cipher(
        &self,
        cipher_view: CipherView,
    ) -> Result<Cipher, VaultError> {
        // 1. Validate input structure
        self.validation_service.validate_cipher_create(&cipher_view)?;

        // 2. Encrypt using SDK
        let encrypted = self.cipher_service.encrypt_cipher(&cipher_view).await?;

        // 3. Send to API
        let created: Cipher = self.api_client
            .post_with_auth("/api/ciphers", &encrypted)
            .await?;

        // 4. Update local cache (atomic)
        self.add_cipher_to_cache(&created).await?;

        Ok(created)
    }

    /// Update existing cipher
    pub async fn update_cipher(
        &self,
        id: &str,
        cipher_view: CipherView,
    ) -> Result<Cipher, VaultError> {
        // 1. Validate item exists
        self.validate_cipher_exists(id).await?;

        // 2. Validate update structure
        self.validation_service.validate_cipher_update(&cipher_view)?;

        // 3. Encrypt using SDK
        let encrypted = self.cipher_service.encrypt_cipher(&cipher_view).await?;

        // 4. Send to API
        let updated: Cipher = self.api_client
            .put_with_auth(&format!("/api/ciphers/{}", id), &encrypted)
            .await?;

        // 5. Update cache atomically
        self.update_cipher_in_cache(&updated).await?;

        Ok(updated)
    }

    /// Delete cipher (soft or permanent)
    pub async fn delete_cipher(
        &self,
        id: &str,
        permanent: bool,
        no_interaction: bool,
    ) -> Result<(), VaultError> {
        // 1. Validate item exists
        self.validate_cipher_exists(id).await?;

        // 2. Confirm if permanent
        if permanent && !no_interaction {
            if !self.confirmation_service.confirm_permanent_delete()? {
                return Err(VaultError::OperationCancelled);
            }
        }

        // 3. Send delete to API
        let endpoint = if permanent {
            format!("/api/ciphers/{}/delete", id)
        } else {
            format!("/api/ciphers/{}", id)
        };

        self.api_client.delete_with_auth(&endpoint).await?;

        // 4. Update cache
        if permanent {
            self.remove_cipher_from_cache(id).await?;
        } else {
            // Soft delete - mark with deletedDate
            self.mark_cipher_deleted(id).await?;
        }

        Ok(())
    }

    /// Restore cipher from trash
    pub async fn restore_cipher(&self, id: &str) -> Result<Cipher, VaultError> {
        // 1. Validate item exists and is deleted
        self.validate_cipher_deleted(id).await?;

        // 2. Send restore to API
        let restored: Cipher = self.api_client
            .put_with_auth(&format!("/api/ciphers/{}/restore", id), &json!({}))
            .await?;

        // 3. Update cache
        self.update_cipher_in_cache(&restored).await?;

        Ok(restored)
    }

    /// Move cipher to different folder
    pub async fn move_cipher(
        &self,
        cipher_id: &str,
        folder_id: Option<&str>,
    ) -> Result<Cipher, VaultError> {
        // 1. Validate cipher exists
        self.validate_cipher_exists(cipher_id).await?;

        // 2. Validate folder exists if specified
        if let Some(fid) = folder_id {
            self.validate_folder_exists(fid).await?;
        }

        // 3. Get current cipher
        let mut cipher = self.get_cipher(cipher_id).await?;

        // 4. Update folder assignment
        cipher.folder_id = folder_id.map(String::from);

        // 5. Send update to API
        let updated = self.update_cipher(cipher_id, cipher).await?;

        Ok(updated)
    }

    /// Create folder
    pub async fn create_folder(&self, name: String) -> Result<Folder, VaultError> {
        // 1. Validate name
        self.validation_service.validate_folder_name(&name)?;

        // 2. Encrypt folder name using SDK
        let encrypted_name = self.cipher_service.encrypt_string(&name).await?;

        // 3. Send to API
        let folder_request = FolderRequest { name: encrypted_name };
        let created: Folder = self.api_client
            .post_with_auth("/api/folders", &folder_request)
            .await?;

        // 4. Update cache
        self.add_folder_to_cache(&created).await?;

        Ok(created)
    }

    /// Update folder name
    pub async fn update_folder(
        &self,
        id: &str,
        name: String,
    ) -> Result<Folder, VaultError> {
        // Similar pattern: validate -> encrypt -> API -> cache
        // Implementation details...
    }

    /// Delete folder
    pub async fn delete_folder(&self, id: &str) -> Result<(), VaultError> {
        // Similar pattern to cipher delete
        // Note: Items in folder become unfoldered
        // Implementation details...
    }

    // Private helper methods

    async fn add_cipher_to_cache(&self, cipher: &Cipher) -> Result<(), VaultError> {
        let mut storage = self.storage.lock().await;

        // Get current vault data
        let mut vault_data: VaultData = storage
            .get("vaultData")?
            .ok_or(VaultError::NotSynced)?;

        // Add new cipher
        vault_data.ciphers.push(cipher.clone());

        // Update timestamp
        vault_data.last_sync = Some(Utc::now());

        // Write atomically
        storage.set("vaultData", &vault_data).await?;
        storage.flush().await?;

        Ok(())
    }

    async fn update_cipher_in_cache(&self, cipher: &Cipher) -> Result<(), VaultError> {
        let mut storage = self.storage.lock().await;

        let mut vault_data: VaultData = storage
            .get("vaultData")?
            .ok_or(VaultError::NotSynced)?;

        // Find and replace cipher
        if let Some(index) = vault_data.ciphers.iter().position(|c| c.id == cipher.id) {
            vault_data.ciphers[index] = cipher.clone();
        } else {
            return Err(VaultError::ItemNotFound);
        }

        // Write atomically
        storage.set("vaultData", &vault_data).await?;
        storage.flush().await?;

        Ok(())
    }

    async fn remove_cipher_from_cache(&self, id: &str) -> Result<(), VaultError> {
        let mut storage = self.storage.lock().await;

        let mut vault_data: VaultData = storage
            .get("vaultData")?
            .ok_or(VaultError::NotSynced)?;

        // Remove cipher
        vault_data.ciphers.retain(|c| c.id != id);

        // Write atomically
        storage.set("vaultData", &vault_data).await?;
        storage.flush().await?;

        Ok(())
    }

    async fn validate_cipher_exists(&self, id: &str) -> Result<(), VaultError> {
        let storage = self.storage.lock().await;
        let vault_data: VaultData = storage
            .get("vaultData")?
            .ok_or(VaultError::NotSynced)?;

        vault_data.ciphers.iter()
            .find(|c| c.id == id)
            .ok_or(VaultError::ItemNotFound)?;

        Ok(())
    }
}
```

**Error Handling Strategy**:
- Validate early (fail fast before encryption/API)
- API errors mapped to domain errors
- Cache updates only after API success
- Atomic cache writes prevent corruption
- Clear error messages with resolution hints

### 2. ValidationService - Input Validation

**Purpose**: Validate cipher structure, field types, and constraints before encryption/submission.

**Location**: `crates/bw-core/src/services/vault/validation_service.rs`

**Interface Design**:

```rust
pub struct ValidationService;

impl ValidationService {
    pub fn new() -> Self {
        Self
    }

    /// Validate cipher for creation
    pub fn validate_cipher_create(&self, cipher: &CipherView) -> Result<(), ValidationError> {
        // Required fields
        self.validate_required_fields(cipher)?;

        // Type-specific validation
        match cipher.r#type {
            CipherType::Login => self.validate_login(cipher)?,
            CipherType::SecureNote => self.validate_secure_note(cipher)?,
            CipherType::Card => self.validate_card(cipher)?,
            CipherType::Identity => self.validate_identity(cipher)?,
        }

        // Field constraints
        self.validate_field_lengths(cipher)?;

        // UUID validation
        self.validate_uuids(cipher)?;

        Ok(())
    }

    /// Validate cipher for update
    pub fn validate_cipher_update(&self, cipher: &CipherView) -> Result<(), ValidationError> {
        // Similar to create, but ID must be present
        if cipher.id.is_none() {
            return Err(ValidationError::MissingField("id".to_string()));
        }

        self.validate_cipher_create(cipher)?;
        Ok(())
    }

    /// Validate folder name
    pub fn validate_folder_name(&self, name: &str) -> Result<(), ValidationError> {
        if name.is_empty() {
            return Err(ValidationError::EmptyField("name".to_string()));
        }

        if name.len() > 1000 {
            return Err(ValidationError::FieldTooLong {
                field: "name".to_string(),
                max: 1000,
                actual: name.len(),
            });
        }

        Ok(())
    }

    // Private validation helpers

    fn validate_required_fields(&self, cipher: &CipherView) -> Result<(), ValidationError> {
        if cipher.name.is_empty() {
            return Err(ValidationError::MissingField("name".to_string()));
        }
        Ok(())
    }

    fn validate_login(&self, cipher: &CipherView) -> Result<(), ValidationError> {
        if cipher.login.is_none() {
            return Err(ValidationError::MissingField("login".to_string()));
        }

        let login = cipher.login.as_ref().unwrap();

        // Validate URIs
        for uri in &login.uris {
            if let Some(uri_str) = &uri.uri {
                if uri_str.len() > 10000 {
                    return Err(ValidationError::FieldTooLong {
                        field: "uri".to_string(),
                        max: 10000,
                        actual: uri_str.len(),
                    });
                }
            }
        }

        // Validate TOTP format if present
        if let Some(totp) = &login.totp {
            self.validate_totp_format(totp)?;
        }

        Ok(())
    }

    fn validate_field_lengths(&self, cipher: &CipherView) -> Result<(), ValidationError> {
        // Name: max 1000 chars
        if cipher.name.len() > 1000 {
            return Err(ValidationError::FieldTooLong {
                field: "name".to_string(),
                max: 1000,
                actual: cipher.name.len(),
            });
        }

        // Notes: max 10000 chars
        if let Some(notes) = &cipher.notes {
            if notes.len() > 10000 {
                return Err(ValidationError::FieldTooLong {
                    field: "notes".to_string(),
                    max: 10000,
                    actual: notes.len(),
                });
            }
        }

        // Additional length checks for type-specific fields...

        Ok(())
    }

    fn validate_uuids(&self, cipher: &CipherView) -> Result<(), ValidationError> {
        // Validate folder_id format if present
        if let Some(folder_id) = &cipher.folder_id {
            if !Self::is_valid_uuid(folder_id) {
                return Err(ValidationError::InvalidUuid {
                    field: "folderId".to_string(),
                    value: folder_id.clone(),
                });
            }
        }

        // Validate organization_id format if present
        if let Some(org_id) = &cipher.organization_id {
            if !Self::is_valid_uuid(org_id) {
                return Err(ValidationError::InvalidUuid {
                    field: "organizationId".to_string(),
                    value: org_id.clone(),
                });
            }
        }

        Ok(())
    }

    fn is_valid_uuid(s: &str) -> bool {
        // UUID v4 format: xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx
        let uuid_regex = regex::Regex::new(
            r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$"
        ).unwrap();
        uuid_regex.is_match(s)
    }

    fn validate_totp_format(&self, totp: &str) -> Result<(), ValidationError> {
        if !totp.starts_with("otpauth://") {
            return Err(ValidationError::InvalidFormat {
                field: "totp".to_string(),
                expected: "otpauth:// URI".to_string(),
                actual: totp.to_string(),
            });
        }
        Ok(())
    }
}
```

**Validation Rules Summary**:

| Field | Constraint | Error Message |
|-------|-----------|---------------|
| `name` | Required, max 1000 chars | "Required field 'name' missing" / "Field 'name' too long (max 1000)" |
| `notes` | Optional, max 10000 chars | "Field 'notes' too long (max 10000)" |
| `type` | Required, 1-4 | "Invalid cipher type: {type}" |
| `login` | Required if type=1 | "Required field 'login' missing for login type" |
| `folderId` | Valid UUID if present | "Invalid UUID format for 'folderId'" |
| `totp` | Valid otpauth:// URI | "Invalid TOTP format, expected otpauth:// URI" |
| `uris` | Valid URI structure | "Invalid URI format" |

### 3. ConfirmationService - User Confirmation Prompts

**Purpose**: Handle confirmation prompts for destructive operations with `--nointeraction` support.

**Location**: `crates/bw-core/src/services/vault/confirmation_service.rs`

**Interface Design**:

```rust
pub struct ConfirmationService {
    no_interaction: bool,
}

impl ConfirmationService {
    pub fn new(no_interaction: bool) -> Self {
        Self { no_interaction }
    }

    /// Confirm permanent delete operation
    pub fn confirm_permanent_delete(&self) -> Result<bool, VaultError> {
        if self.no_interaction {
            return Ok(true);  // Auto-confirm in non-interactive mode
        }

        self.prompt_yes_no(
            "Are you sure you want to permanently delete this item? [y/N]: "
        )
    }

    /// Generic yes/no prompt
    fn prompt_yes_no(&self, message: &str) -> Result<bool, VaultError> {
        use std::io::{self, Write};

        print!("{}", message);
        io::stdout().flush()
            .map_err(|e| VaultError::IoError(e.to_string()))?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)
            .map_err(|e| VaultError::IoError(e.to_string()))?;

        let response = input.trim().to_lowercase();
        Ok(response == "y" || response == "yes")
    }
}
```

**Confirmation Behavior**:
- `--nointeraction` flag: Auto-confirm all prompts (for scripts)
- Interactive mode: Show prompt, wait for y/n response
- Default to "no" if user presses Enter without input
- Only confirm for truly destructive operations (permanent delete)

### 4. CipherService Extensions - Encryption Methods

**Purpose**: Extend existing CipherService with encryption methods for write operations.

**Location**: `crates/bw-core/src/services/vault/cipher_service.rs` (EXTEND)

**New Methods to Add**:

```rust
impl CipherService {
    /// Encrypt cipher view to cipher (for API submission)
    pub async fn encrypt_cipher(
        &self,
        cipher_view: &CipherView,
    ) -> Result<Cipher, VaultError> {
        // Use SDK to encrypt all fields
        let encrypted_name = self.encrypt_string(&cipher_view.name).await?;

        let encrypted_notes = if let Some(notes) = &cipher_view.notes {
            Some(self.encrypt_string(notes).await?)
        } else {
            None
        };

        let encrypted_login = if let Some(login) = &cipher_view.login {
            Some(self.encrypt_login(login).await?)
        } else {
            None
        };

        // Build encrypted cipher
        Ok(Cipher {
            id: cipher_view.id.clone().unwrap_or_default(),
            r#type: cipher_view.r#type,
            name: encrypted_name,
            notes: encrypted_notes,
            login: encrypted_login,
            folder_id: cipher_view.folder_id.clone(),
            organization_id: cipher_view.organization_id.clone(),
            favorite: cipher_view.favorite,
            // ... other fields
        })
    }

    /// Encrypt a single string field
    async fn encrypt_string(&self, plain_text: &str) -> Result<String, VaultError> {
        // TODO: Use SDK client to encrypt
        // self.sdk_client.encrypt_string(plain_text).await
        //     .map_err(|e| VaultError::EncryptionError(e.to_string()))

        // Placeholder for MVP (SDK integration)
        Ok(format!("2.encrypted_{}", plain_text))
    }

    /// Encrypt login data
    async fn encrypt_login(
        &self,
        login: &CipherLoginView,
    ) -> Result<CipherLogin, VaultError> {
        let encrypted_username = if let Some(username) = &login.username {
            Some(self.encrypt_string(username).await?)
        } else {
            None
        };

        let encrypted_password = if let Some(password) = &login.password {
            Some(self.encrypt_string(password).await?)
        } else {
            None
        };

        let encrypted_totp = if let Some(totp) = &login.totp {
            Some(self.encrypt_string(totp).await?)
        } else {
            None
        };

        let encrypted_uris = self.encrypt_uris(&login.uris).await?;

        Ok(CipherLogin {
            username: encrypted_username,
            password: encrypted_password,
            totp: encrypted_totp,
            uris: encrypted_uris,
        })
    }

    /// Encrypt URI list
    async fn encrypt_uris(
        &self,
        uris: &[LoginUriView],
    ) -> Result<Vec<LoginUri>, VaultError> {
        let mut encrypted_uris = Vec::new();

        for uri_view in uris {
            let encrypted_uri = if let Some(uri) = &uri_view.uri {
                Some(self.encrypt_string(uri).await?)
            } else {
                None
            };

            encrypted_uris.push(LoginUri {
                uri: encrypted_uri,
                r#match: uri_view.r#match,
            });
        }

        Ok(encrypted_uris)
    }
}
```

**Encryption Strategy**:
- All sensitive fields encrypted via SDK `encrypt_string()` method
- Result format: `{type}.{iv}.{ciphertext}.{mac}` (EncString format)
- Type prefix: `2` for AES-256-CBC with HMAC-SHA256
- SDK handles all cryptographic operations (key derivation, IV generation, encryption, MAC)
- Error handling: Map SDK errors to VaultError::EncryptionError

## Data Models

### Validation Error Model

**Location**: `crates/bw-core/src/models/vault/validation_error.rs` (NEW)

```rust
#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("Required field '{0}' is missing")]
    MissingField(String),

    #[error("Field '{0}' is empty")]
    EmptyField(String),

    #[error("Field '{field}' is too long (max {max}, actual {actual})")]
    FieldTooLong {
        field: String,
        max: usize,
        actual: usize,
    },

    #[error("Invalid UUID format for '{field}': {value}")]
    InvalidUuid {
        field: String,
        value: String,
    },

    #[error("Invalid format for '{field}': expected {expected}, got {actual}")]
    InvalidFormat {
        field: String,
        expected: String,
        actual: String,
    },

    #[error("Invalid cipher type: {0}")]
    InvalidCipherType(i32),

    #[error("Type mismatch: cipher type {cipher_type} requires {field}")]
    TypeMismatch {
        cipher_type: String,
        field: String,
    },
}
```

### API Request Models

**Location**: `crates/bw-core/src/models/vault/cipher_request.rs` (NEW)

```rust
/// Request body for creating/updating ciphers
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CipherRequest {
    pub r#type: CipherType,
    pub name: String,                    // Encrypted
    pub notes: Option<String>,           // Encrypted
    pub favorite: bool,
    pub folder_id: Option<String>,
    pub organization_id: Option<String>,
    pub login: Option<CipherLoginRequest>,
    pub secure_note: Option<SecureNoteRequest>,
    pub card: Option<CardRequest>,
    pub identity: Option<IdentityRequest>,
    pub fields: Vec<FieldRequest>,
    pub password_history: Vec<PasswordHistoryRequest>,
}

impl From<Cipher> for CipherRequest {
    fn from(cipher: Cipher) -> Self {
        // Convert Cipher to request format
        Self {
            r#type: cipher.r#type,
            name: cipher.name,
            notes: cipher.notes,
            favorite: cipher.favorite,
            folder_id: cipher.folder_id,
            organization_id: cipher.organization_id,
            login: cipher.login.map(Into::into),
            // ... other fields
            fields: Vec::new(),
            password_history: Vec::new(),
        }
    }
}

/// Request body for folder operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderRequest {
    pub name: String,  // Encrypted
}
```

### Extended Error Types

**Location**: `crates/bw-core/src/services/vault/errors.rs` (EXTEND)

```rust
#[derive(Debug, Error)]
pub enum VaultError {
    // Existing errors...
    #[error("Not authenticated. Run 'bw login' first.")]
    NotAuthenticated,

    #[error("Vault not synced. Run 'bw sync' first.")]
    NotSynced,

    #[error("Item not found")]
    ItemNotFound,

    // New write operation errors
    #[error("Validation error: {0}")]
    ValidationError(#[from] ValidationError),

    #[error("Encryption error: {0}")]
    EncryptionError(String),

    #[error("Decryption error: {0}")]
    DecryptionError(String),

    #[error("Operation cancelled by user")]
    OperationCancelled,

    #[error("Folder not found")]
    FolderNotFound,

    #[error("Item is not in trash")]
    ItemNotDeleted,

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("API error: {0}")]
    ApiError(String),

    #[error("IO error: {0}")]
    IoError(String),
}
```

## Command Layer Design

### Command Structure

**Location**: `crates/bw-cli/src/commands/vault/` (NEW modules)

### Create Commands

**File**: `crates/bw-cli/src/commands/vault/create.rs`

```rust
use clap::{Args, Subcommand};
use anyhow::Result;
use bw_core::models::vault::{CipherView, CipherType};
use bw_core::services::vault::WriteService;
use crate::output::Response;

#[derive(Subcommand)]
pub enum CreateCommands {
    /// Create a new item
    Item(CreateItemArgs),

    /// Create a new folder
    Folder(CreateFolderArgs),

    /// Create an attachment
    #[cfg(feature = "attachments")]
    Attachment(CreateAttachmentArgs),
}

#[derive(Args)]
pub struct CreateItemArgs {
    /// Base64-encoded JSON or raw JSON (use stdin or bw encode)
    #[arg(value_name = "JSON")]
    pub item_json: String,
}

#[derive(Args)]
pub struct CreateFolderArgs {
    /// Folder name
    #[arg(value_name = "NAME")]
    pub name: String,
}

pub async fn execute_create(
    cmd: CreateCommands,
    write_service: &WriteService,
    global_args: &GlobalArgs,
) -> Result<Response> {
    match cmd {
        CreateCommands::Item(args) => execute_create_item(args, write_service).await,
        CreateCommands::Folder(args) => execute_create_folder(args, write_service).await,
        #[cfg(feature = "attachments")]
        CreateCommands::Attachment(args) => execute_create_attachment(args, write_service).await,
    }
}

async fn execute_create_item(
    args: CreateItemArgs,
    write_service: &WriteService,
) -> Result<Response> {
    // 1. Decode input (base64 or raw JSON)
    let json_str = if is_base64(&args.item_json) {
        decode_base64(&args.item_json)?
    } else {
        args.item_json.clone()
    };

    // 2. Parse JSON to CipherView
    let cipher_view: CipherView = serde_json::from_str(&json_str)
        .map_err(|e| anyhow::anyhow!("Invalid JSON: {}", e))?;

    // 3. Call service to create
    match write_service.create_cipher(cipher_view).await {
        Ok(cipher) => {
            // Decrypt for display
            let cipher_view = write_service.decrypt_cipher(&cipher).await?;
            Ok(Response::success(cipher_view))
        }
        Err(e) => Ok(Response::error(format!("Failed to create item: {}", e))),
    }
}

async fn execute_create_folder(
    args: CreateFolderArgs,
    write_service: &WriteService,
) -> Result<Response> {
    match write_service.create_folder(args.name).await {
        Ok(folder) => {
            // Decrypt for display
            let folder_view = write_service.decrypt_folder(&folder).await?;
            Ok(Response::success(folder_view))
        }
        Err(e) => Ok(Response::error(format!("Failed to create folder: {}", e))),
    }
}

fn is_base64(s: &str) -> bool {
    // Simple heuristic: base64 doesn't contain spaces or newlines
    !s.contains('\n') && !s.contains(' ') && s.len() % 4 == 0
}

fn decode_base64(s: &str) -> Result<String> {
    use base64::{Engine as _, engine::general_purpose};
    let bytes = general_purpose::STANDARD.decode(s)?;
    Ok(String::from_utf8(bytes)?)
}
```

### Edit Commands

**File**: `crates/bw-cli/src/commands/vault/edit.rs`

```rust
#[derive(Subcommand)]
pub enum EditCommands {
    /// Edit an existing item
    Item(EditItemArgs),

    /// Edit a folder
    Folder(EditFolderArgs),
}

#[derive(Args)]
pub struct EditItemArgs {
    /// Item ID
    #[arg(value_name = "ID")]
    pub id: String,

    /// Base64-encoded JSON or raw JSON (use stdin or bw encode)
    #[arg(value_name = "JSON")]
    pub item_json: String,
}

#[derive(Args)]
pub struct EditFolderArgs {
    /// Folder ID
    #[arg(value_name = "ID")]
    pub id: String,

    /// New folder name
    #[arg(value_name = "NAME")]
    pub name: String,
}

pub async fn execute_edit(
    cmd: EditCommands,
    write_service: &WriteService,
) -> Result<Response> {
    match cmd {
        EditCommands::Item(args) => execute_edit_item(args, write_service).await,
        EditCommands::Folder(args) => execute_edit_folder(args, write_service).await,
    }
}

async fn execute_edit_item(
    args: EditItemArgs,
    write_service: &WriteService,
) -> Result<Response> {
    // 1. Decode input
    let json_str = if is_base64(&args.item_json) {
        decode_base64(&args.item_json)?
    } else {
        args.item_json.clone()
    };

    // 2. Parse JSON
    let mut cipher_view: CipherView = serde_json::from_str(&json_str)
        .map_err(|e| anyhow::anyhow!("Invalid JSON: {}", e))?;

    // 3. Ensure ID matches
    cipher_view.id = Some(args.id.clone());

    // 4. Call service to update
    match write_service.update_cipher(&args.id, cipher_view).await {
        Ok(cipher) => {
            let cipher_view = write_service.decrypt_cipher(&cipher).await?;
            Ok(Response::success(cipher_view))
        }
        Err(e) => Ok(Response::error(format!("Failed to edit item: {}", e))),
    }
}
```

### Delete Commands

**File**: `crates/bw-cli/src/commands/vault/delete.rs`

```rust
#[derive(Subcommand)]
pub enum DeleteCommands {
    /// Delete an item
    Item(DeleteItemArgs),

    /// Delete a folder
    Folder(DeleteFolderArgs),
}

#[derive(Args)]
pub struct DeleteItemArgs {
    /// Item ID
    #[arg(value_name = "ID")]
    pub id: String,

    /// Permanently delete (cannot be restored)
    #[arg(long)]
    pub permanent: bool,
}

#[derive(Args)]
pub struct DeleteFolderArgs {
    /// Folder ID
    #[arg(value_name = "ID")]
    pub id: String,
}

pub async fn execute_delete(
    cmd: DeleteCommands,
    write_service: &WriteService,
    no_interaction: bool,
) -> Result<Response> {
    match cmd {
        DeleteCommands::Item(args) => {
            execute_delete_item(args, write_service, no_interaction).await
        }
        DeleteCommands::Folder(args) => {
            execute_delete_folder(args, write_service).await
        }
    }
}

async fn execute_delete_item(
    args: DeleteItemArgs,
    write_service: &WriteService,
    no_interaction: bool,
) -> Result<Response> {
    match write_service.delete_cipher(&args.id, args.permanent, no_interaction).await {
        Ok(_) => {
            let message = if args.permanent {
                format!("Item {} permanently deleted", args.id)
            } else {
                format!("Item {} moved to trash", args.id)
            };
            Ok(Response::success_message(message))
        }
        Err(e) => Ok(Response::error(format!("Failed to delete item: {}", e))),
    }
}
```

### Restore Commands

**File**: `crates/bw-cli/src/commands/vault/restore.rs`

```rust
#[derive(Args)]
pub struct RestoreArgs {
    /// Item ID to restore
    #[arg(value_name = "ID")]
    pub id: String,
}

pub async fn execute_restore(
    args: RestoreArgs,
    write_service: &WriteService,
) -> Result<Response> {
    match write_service.restore_cipher(&args.id).await {
        Ok(cipher) => {
            let cipher_view = write_service.decrypt_cipher(&cipher).await?;
            Ok(Response::success(cipher_view))
        }
        Err(e) => Ok(Response::error(format!("Failed to restore item: {}", e))),
    }
}
```

### Move Command

**File**: `crates/bw-cli/src/commands/vault/restore.rs` (same file)

```rust
#[derive(Args)]
pub struct MoveArgs {
    /// Item ID to move
    #[arg(value_name = "ITEM_ID")]
    pub item_id: String,

    /// Target folder ID (use "null" for root)
    #[arg(value_name = "FOLDER_ID")]
    pub folder_id: String,
}

pub async fn execute_move(
    args: MoveArgs,
    write_service: &WriteService,
) -> Result<Response> {
    let folder_id = if args.folder_id == "null" {
        None
    } else {
        Some(args.folder_id.as_str())
    };

    match write_service.move_cipher(&args.item_id, folder_id).await {
        Ok(cipher) => {
            let cipher_view = write_service.decrypt_cipher(&cipher).await?;
            Ok(Response::success(cipher_view))
        }
        Err(e) => Ok(Response::error(format!("Failed to move item: {}", e))),
    }
}
```

### Command Registration

**File**: `crates/bw-cli/src/commands/vault.rs` (EXTEND)

```rust
#[derive(Subcommand)]
pub enum VaultCommands {
    // Existing read commands...
    List(ListCommand),
    Get(GetCommand),

    // New write commands
    Create(CreateCommands),
    Edit(EditCommands),
    Delete(DeleteCommands),
    Restore(RestoreArgs),
    Move(MoveArgs),
}

pub async fn execute_vault_command(
    cmd: VaultCommands,
    global_args: &GlobalArgs,
) -> Result<Response> {
    // Create services
    let container = ServiceContainer::new(/* ... */)?;
    let write_service = WriteService::new(
        container.api_client(),
        container.storage(),
        Arc::new(CipherService::new(container.sdk())),
        Arc::new(ValidationService::new()),
        Arc::new(ConfirmationService::new(global_args.nointeraction)),
    );

    match cmd {
        // Existing commands...
        VaultCommands::List(args) => { /* ... */ }
        VaultCommands::Get(args) => { /* ... */ }

        // New write commands
        VaultCommands::Create(create_cmd) => {
            create::execute_create(create_cmd, &write_service, global_args).await
        }
        VaultCommands::Edit(edit_cmd) => {
            edit::execute_edit(edit_cmd, &write_service).await
        }
        VaultCommands::Delete(delete_cmd) => {
            delete::execute_delete(delete_cmd, &write_service, global_args.nointeraction).await
        }
        VaultCommands::Restore(restore_args) => {
            restore::execute_restore(restore_args, &write_service).await
        }
        VaultCommands::Move(move_args) => {
            restore::execute_move(move_args, &write_service).await
        }
    }
}
```

## API Endpoints

### Cipher Endpoints

| Operation | Method | Endpoint | Request Body | Response |
|-----------|--------|----------|--------------|----------|
| Create Item | POST | `/api/ciphers` | CipherRequest | Cipher |
| Update Item | PUT | `/api/ciphers/{id}` | CipherRequest | Cipher |
| Delete Item (soft) | DELETE | `/api/ciphers/{id}` | - | - |
| Delete Item (hard) | DELETE | `/api/ciphers/{id}/delete` | - | - |
| Restore Item | PUT | `/api/ciphers/{id}/restore` | {} | Cipher |

### Folder Endpoints

| Operation | Method | Endpoint | Request Body | Response |
|-----------|--------|----------|--------------|----------|
| Create Folder | POST | `/api/folders` | FolderRequest | Folder |
| Update Folder | PUT | `/api/folders/{id}` | FolderRequest | Folder |
| Delete Folder | DELETE | `/api/folders/{id}` | - | - |

### Error Responses

API error responses follow this format:

```json
{
  "error": "Validation failed",
  "message": "The Name field is required.",
  "statusCode": 400
}
```

**Error Mapping**:
- `400 Bad Request` → ValidationError
- `401 Unauthorized` → NotAuthenticated (handled by API client)
- `403 Forbidden` → PermissionDenied
- `404 Not Found` → ItemNotFound / FolderNotFound
- `500 Internal Server Error` → ApiError

## Testing Strategy

### Unit Tests

**Validation Service Tests**:
```rust
#[cfg(test)]
mod validation_tests {
    use super::*;

    #[test]
    fn test_validate_cipher_create_success() {
        let validator = ValidationService::new();
        let cipher = create_valid_login_cipher();

        let result = validator.validate_cipher_create(&cipher);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_cipher_missing_name() {
        let validator = ValidationService::new();
        let mut cipher = create_valid_login_cipher();
        cipher.name = "".to_string();

        let result = validator.validate_cipher_create(&cipher);
        assert!(matches!(result, Err(ValidationError::EmptyField(_))));
    }

    #[test]
    fn test_validate_cipher_name_too_long() {
        let validator = ValidationService::new();
        let mut cipher = create_valid_login_cipher();
        cipher.name = "a".repeat(1001);

        let result = validator.validate_cipher_create(&cipher);
        assert!(matches!(result, Err(ValidationError::FieldTooLong { .. })));
    }

    #[test]
    fn test_validate_invalid_uuid() {
        let validator = ValidationService::new();
        let mut cipher = create_valid_login_cipher();
        cipher.folder_id = Some("not-a-uuid".to_string());

        let result = validator.validate_cipher_create(&cipher);
        assert!(matches!(result, Err(ValidationError::InvalidUuid { .. })));
    }
}
```

**Write Service Tests** (with mocks):
```rust
#[cfg(test)]
mod write_service_tests {
    use super::*;
    use mockall::mock;

    mock! {
        pub ApiClientMock {}

        #[async_trait]
        impl ApiClient for ApiClientMock {
            async fn post_with_auth<T, R>(&self, path: &str, body: &T) -> Result<R>;
            async fn put_with_auth<T, R>(&self, path: &str, body: &T) -> Result<R>;
            async fn delete_with_auth(&self, path: &str) -> Result<()>;
        }
    }

    #[tokio::test]
    async fn test_create_cipher_success() {
        let mut mock_api = MockApiClientMock::new();
        mock_api.expect_post_with_auth()
            .returning(|_, _| Ok(create_test_cipher()));

        let write_service = create_test_write_service(mock_api);
        let cipher_view = create_valid_cipher_view();

        let result = write_service.create_cipher(cipher_view).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_cipher_validation_error() {
        let mock_api = MockApiClientMock::new();
        let write_service = create_test_write_service(mock_api);

        let mut cipher_view = create_valid_cipher_view();
        cipher_view.name = "".to_string();  // Invalid

        let result = write_service.create_cipher(cipher_view).await;
        assert!(matches!(result, Err(VaultError::ValidationError(_))));
    }
}
```

### Integration Tests

**Create Flow Test**:
```rust
#[tokio::test]
#[ignore]  // Requires test vault
async fn test_create_edit_delete_flow() {
    let container = ServiceContainer::new_for_test()?;
    let write_service = WriteService::new(/* ... */);

    // 1. Create item
    let cipher_view = CipherView {
        id: None,
        r#type: CipherType::Login,
        name: "Integration Test Item".to_string(),
        login: Some(LoginView {
            username: Some("test@example.com".to_string()),
            password: Some("testpass123".to_string()),
            uris: vec![],
            totp: None,
        }),
        ..Default::default()
    };

    let created = write_service.create_cipher(cipher_view).await
        .expect("Failed to create cipher");

    let created_id = created.id.clone();

    // 2. Edit item
    let mut updated_view = write_service.decrypt_cipher(&created).await?;
    updated_view.name = "Updated Test Item".to_string();

    let updated = write_service.update_cipher(&created_id, updated_view).await
        .expect("Failed to update cipher");

    assert_eq!(updated.name, "Updated Test Item");

    // 3. Delete item (soft)
    write_service.delete_cipher(&created_id, false, true).await
        .expect("Failed to delete cipher");

    // 4. Restore item
    let restored = write_service.restore_cipher(&created_id).await
        .expect("Failed to restore cipher");

    // 5. Permanent delete
    write_service.delete_cipher(&created_id, true, true).await
        .expect("Failed to permanently delete cipher");

    // 6. Verify removed
    let result = write_service.get_cipher(&created_id).await;
    assert!(matches!(result, Err(VaultError::ItemNotFound)));
}
```

### Test Coverage Goals

- **Validation Service**: 100% (pure functions, easy to test)
- **Write Service**: >80% (unit tests with mocks)
- **Command Layer**: >70% (integration tests)
- **Error Handling**: 100% (test all error paths)
- **Cache Updates**: 100% (critical for data integrity)

## Security Considerations

### 1. Encryption Security

**Requirements**:
- All sensitive fields encrypted using SDK
- No custom cryptography implementations
- Use `secrecy::Secret<String>` for passwords in memory
- Zeroize sensitive data after use

**Implementation**:
```rust
use secrecy::{Secret, ExposeSecret};
use zeroize::Zeroize;

// When handling passwords in plain text
let password: Secret<String> = Secret::new(cipher_view.login.password.clone());

// Use password (expose only when necessary)
let encrypted = self.sdk_client
    .encrypt_string(password.expose_secret())
    .await?;

// Password automatically zeroized when dropped
```

### 2. Input Validation

**Requirements**:
- Validate all user input before encryption/API
- Prevent injection attacks (SQL, command injection)
- Validate UUIDs, URLs, TOTP format
- Enforce max lengths to prevent DoS

**Implementation**:
- ValidationService handles all validation
- Regex patterns for UUID/TOTP validation
- Length checks on all string fields
- Type validation for cipher types

### 3. Confirmation for Destructive Operations

**Requirements**:
- Confirm permanent delete (cannot be undone)
- No confirmation for soft delete (recoverable)
- Respect `--nointeraction` flag for scripts

**Implementation**:
- ConfirmationService manages all prompts
- Default to "no" if user just presses Enter
- Auto-confirm in `--nointeraction` mode

### 4. Audit Logging

**Requirements**:
- Log all write operations (create, edit, delete)
- Never log decrypted sensitive data
- Include operation type, item ID, timestamp

**Implementation**:
```rust
// Safe logging (no sensitive data)
log::info!("Creating cipher of type {:?}", cipher_view.r#type);
log::info!("Updated cipher {}", cipher_id);
log::warn!("Permanently deleting cipher {}", cipher_id);

// NEVER log decrypted passwords, notes, etc.
```

### 5. Memory Safety

**Requirements**:
- Clear sensitive data from memory after use
- Prevent sensitive data in error messages
- Use Rust's ownership for memory safety

**Implementation**:
- Sensitive strings wrapped in `Secret<T>`
- Automatic zeroization on drop
- No sensitive data in error display strings

## Performance Considerations

### 1. Encryption Performance

**Challenge**: SDK encryption may be slower than plain operations.

**Mitigation**:
- Validate inputs BEFORE encryption (fail fast)
- Parallel encryption for multiple fields if supported
- Cache encrypted results where possible
- Set realistic performance expectations (<2s for create/edit)

**Benchmarking**:
```rust
#[bench]
fn bench_encrypt_cipher(b: &mut Bencher) {
    let cipher_view = create_test_cipher_view();
    let cipher_service = create_test_cipher_service();

    b.iter(|| {
        cipher_service.encrypt_cipher(&cipher_view).await
    });
}
```

### 2. Cache Update Performance

**Challenge**: Atomic cache writes require reading/writing entire vault data.

**Current Approach**:
- Read entire vaultData from storage
- Modify cipher list
- Write entire vaultData atomically

**Optimization Opportunities**:
- Incremental cache updates (future enhancement)
- In-memory cache with periodic flush
- Dirty flag to skip unnecessary writes

**Acceptable for MVP**: Current approach is simple, correct, and adequate for typical vault sizes (<10,000 items).

### 3. API Performance

**Challenge**: Network latency for API calls.

**Mitigation**:
- Connection pooling (already in reqwest)
- Retry transient failures
- Batch operations (future enhancement)

## Error Handling

### Error Flow Diagram

```
User Input
    │
    ▼
Validation  ─────► ValidationError ─────┐
    │                                    │
    ▼                                    │
Encryption  ─────► EncryptionError ────┤
    │                                    │
    ▼                                    ▼
API Call    ─────► ApiError ──────────► Response::error()
    │                                    │
    ▼                                    │
Cache Update ────► StorageError ───────┘
    │
    ▼
Success ─────────────────────────────► Response::success()
```

### Error Handling Pattern

```rust
// Service layer: Propagate domain errors
pub async fn create_cipher(&self, cipher_view: CipherView) -> Result<Cipher, VaultError> {
    self.validation_service.validate_cipher_create(&cipher_view)?;  // Propagate ValidationError
    let encrypted = self.cipher_service.encrypt_cipher(&cipher_view).await?;  // Propagate EncryptionError
    let created = self.api_client.post_with_auth("/api/ciphers", &encrypted).await?;  // Propagate ApiError
    self.add_cipher_to_cache(&created).await?;  // Propagate StorageError
    Ok(created)
}

// Command layer: Convert to Response
match write_service.create_cipher(cipher_view).await {
    Ok(cipher) => Ok(Response::success(cipher)),
    Err(e) => Ok(Response::error(e.to_string())),
}
```

### User-Facing Error Messages

| Error Type | User Message | Resolution Hint |
|------------|--------------|-----------------|
| ValidationError::MissingField | "Required field '{field}' is missing" | "Check your JSON input" |
| VaultError::NotAuthenticated | "Not authenticated. Run 'bw login' first." | Clear action |
| VaultError::ItemNotFound | "Item not found" | "Verify item ID with 'bw list items'" |
| VaultError::OperationCancelled | "Operation cancelled" | User chose not to proceed |
| EncryptionError | "Encryption failed: {details}" | "Try again or file issue" |
| ApiError (400) | "Invalid request: {message}" | "Check input format" |
| ApiError (401) | "Session expired. Run 'bw unlock' or 'bw login'." | Clear action |
| ApiError (403) | "Permission denied" | "Check organization membership" |
| ApiError (500) | "Server error. Try again later." | Transient issue |

## Implementation Phases

### Phase 1: MVP Core (Must Have) - 10-14 days

**Goal**: Basic CRUD operations for items and folders.

#### Week 1: Foundation (5 days)

**Day 1-2: Service Layer Setup**
- [ ] Create `WriteService` with basic structure
- [ ] Create `ValidationService` with core validation
- [ ] Create `ConfirmationService`
- [ ] Extend `VaultError` with write operation errors
- [ ] Unit tests for validation service

**Day 3-4: Encryption Integration**
- [ ] Extend `CipherService` with `encrypt_cipher()` method
- [ ] Implement `encrypt_string()`, `encrypt_login()` helpers
- [ ] SDK integration for encryption
- [ ] Unit tests for encryption methods
- [ ] Handle encryption errors gracefully

**Day 5: Cache Update Mechanism**
- [ ] Implement `add_cipher_to_cache()`
- [ ] Implement `update_cipher_in_cache()`
- [ ] Implement `remove_cipher_from_cache()`
- [ ] Atomic write strategy with temp files
- [ ] Unit tests for cache operations

#### Week 2: Commands (5 days)

**Day 6-7: Create Commands**
- [ ] Command structure for `bw create item`
- [ ] Command structure for `bw create folder`
- [ ] JSON parsing and base64 decoding
- [ ] Wire up commands to WriteService
- [ ] Integration tests for create flow

**Day 8: Edit Commands**
- [ ] Command structure for `bw edit item`
- [ ] Command structure for `bw edit folder`
- [ ] Validate item exists before edit
- [ ] Integration tests for edit flow

**Day 9: Delete Commands**
- [ ] Command structure for `bw delete item` (soft/permanent)
- [ ] Command structure for `bw delete folder`
- [ ] Confirmation prompt for permanent delete
- [ ] Integration tests for delete flow

**Day 10: Restore & Move Commands**
- [ ] Command structure for `bw restore item`
- [ ] Command structure for `bw move`
- [ ] Validation for restore/move operations
- [ ] Integration tests

#### Week 3: Testing & Polish (4 days)

**Day 11-12: Comprehensive Testing**
- [ ] Unit tests for WriteService (with mocks)
- [ ] Integration tests for full CRUD flows
- [ ] Error handling tests (all error paths)
- [ ] Cache consistency tests
- [ ] Security tests (encryption, validation)

**Day 13: Validation Enhancement**
- [ ] Complete validation rules for all cipher types
- [ ] Validation for card, identity, secure note types
- [ ] UUID format validation
- [ ] TOTP format validation
- [ ] Max length validations

**Day 14: Documentation & Refinement**
- [ ] Code documentation (rustdoc)
- [ ] Error message improvements
- [ ] Command help text
- [ ] README updates

**Phase 1 Exit Criteria**:
- [ ] All MVP commands working (create, edit, delete, restore, move)
- [ ] Encryption via SDK operational
- [ ] Cache updates atomic and consistent
- [ ] Validation prevents invalid data submission
- [ ] Integration tests pass
- [ ] Can create/edit/delete items in real vault
- [ ] `cargo clippy` passes with no warnings
- [ ] `cargo test` passes all tests

### Phase 2: Extended Features (Should Have) - 5-7 days

**Goal**: Attachments, organization features, template generation.

**Day 15-16: Template Generation**
- [ ] `bw get template item.login`
- [ ] `bw get template item.note`
- [ ] `bw get template item.card`
- [ ] `bw get template item.identity`
- [ ] `bw get template folder`
- [ ] Complete templates with all fields

**Day 17-18: Attachment Commands**
- [ ] `bw create attachment --file <path> --itemid <id>`
- [ ] `bw delete attachment <id> --itemid <id>`
- [ ] File upload with progress indication
- [ ] Encryption of attachment data
- [ ] Integration tests

**Day 19-21: Organization Features**
- [ ] `bw create org-collection`
- [ ] `bw edit org-collection`
- [ ] `bw delete org-collection`
- [ ] `bw share <itemId> <orgId>`
- [ ] `bw confirm org-member`
- [ ] `bw edit item-collections`
- [ ] Permission validation
- [ ] Integration tests

**Phase 2 Exit Criteria**:
- [ ] All "Should Have" features implemented
- [ ] Templates generate correct JSON structures
- [ ] Attachment upload working with progress
- [ ] Organization features tested
- [ ] Permission validation operational

### Phase 3: Polish & Optimization (Nice to Have) - 2-3 days

**Goal**: UX improvements, performance tuning, final polish.

**Day 22: UX Improvements**
- [ ] Better error messages with resolution hints
- [ ] Success messages with helpful context
- [ ] Confirmation prompt polish
- [ ] Help text improvements

**Day 23: Performance Optimization**
- [ ] Benchmark encryption performance
- [ ] Optimize cache update frequency
- [ ] Profile critical paths
- [ ] Address any bottlenecks

**Day 24: Final Polish**
- [ ] Documentation review
- [ ] Code cleanup
- [ ] Final integration tests
- [ ] User acceptance testing

**Phase 3 Exit Criteria**:
- [ ] Performance meets targets (<2s for CRUD)
- [ ] Error messages clear and actionable
- [ ] Help text comprehensive
- [ ] Code review complete
- [ ] Ready for production use

## Migration Strategy

### From TypeScript CLI

**Compatibility Goals**:
1. Identical JSON input/output format
2. Same command syntax and flags
3. Compatible data encryption (interoperable)
4. Same validation rules

**Validation**:
- Test creating items in Rust CLI, viewing in TypeScript CLI
- Test creating items in TypeScript CLI, editing in Rust CLI
- Compare encrypted data format
- Verify cache file compatibility

**Migration Path**:
- Users can switch between CLIs seamlessly
- Shared data directory and cache
- Compatible session files

## Dependencies

### External Crates

```toml
# Cargo.toml additions

[dependencies]
# Existing dependencies...
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "1"
anyhow = "1"
clap = { version = "4", features = ["derive"] }
base64 = "0.21"

# New dependencies for vault write
secrecy = "0.8"       # Secret<T> wrapper for sensitive data
zeroize = "1"         # Secure memory clearing
regex = "1"           # UUID/TOTP validation
```

### Internal Dependencies

- **Enhancement 1**: SDK integration (complete)
- **Enhancement 2**: Storage layer (complete)
- **Enhancement 3**: API client (complete)
- **Enhancement 4**: Authentication (complete)
- **Enhancement 5**: Vault read commands (complete)

**Blockers**: None - all dependencies complete.

## Risks & Mitigation

### Risk 1: Data Loss from Destructive Operations
**Severity**: Critical
**Probability**: Medium

**Mitigation**:
- Confirmation prompts for permanent delete
- Soft delete as default (recoverable)
- Comprehensive validation before submission
- Clear error messages
- Integration tests for delete operations

**Contingency**: If data loss occurs, user must rely on server backup/revision history.

### Risk 2: SDK Encryption Failures
**Severity**: High
**Probability**: Low

**Mitigation**:
- Use SDK exclusively (no custom crypto)
- Comprehensive error handling
- Test with all cipher types
- Validate encrypted output format

**Contingency**: If SDK bugs found, work with SDK team. Create reproducible test cases.

### Risk 3: Cache Consistency Issues
**Severity**: High
**Probability**: Medium

**Mitigation**:
- Atomic cache updates (temp file + rename)
- Never update cache on failure
- Validation before cache operations
- Clear error messages

**Contingency**: User runs `bw sync --force` to rebuild cache.

### Risk 4: Validation Rule Drift
**Severity**: Medium
**Probability**: Medium

**Mitigation**:
- Reference TypeScript CLI validation directly
- Compatibility test suite
- Document validation rules explicitly
- Monitor TypeScript CLI changes

**Contingency**: Update Rust validation to match TypeScript CLI when divergence detected.

## Success Criteria

### Functional Success Criteria

- [ ] **Create Operations**: Can create items of all types (login, note, card, identity)
- [ ] **Edit Operations**: Can update existing items with field changes
- [ ] **Delete Operations**: Can soft delete (trash) and permanently delete items
- [ ] **Restore Operations**: Can restore items from trash
- [ ] **Move Operations**: Can move items between folders
- [ ] **Folder Operations**: Can create, edit, and delete folders
- [ ] **Validation**: Invalid inputs rejected with clear errors
- [ ] **Encryption**: All sensitive fields encrypted via SDK
- [ ] **Cache Consistency**: Local cache reflects server state after operations

### Non-Functional Success Criteria

- [ ] **Performance**: CRUD operations complete in <2 seconds
- [ ] **Compatibility**: JSON format matches TypeScript CLI exactly
- [ ] **Interoperability**: Can edit items created by TypeScript CLI
- [ ] **Security**: All encryption via SDK, no custom crypto
- [ ] **Reliability**: Operations atomic (success or failure, no partial state)
- [ ] **Usability**: Error messages clear with resolution hints
- [ ] **Code Quality**: `cargo clippy` passes, `cargo test` passes

### Test Coverage Criteria

- [ ] **Validation Service**: 100% coverage
- [ ] **Write Service**: >80% coverage
- [ ] **Command Layer**: >70% coverage
- [ ] **Error Paths**: 100% coverage
- [ ] **Integration Tests**: All major flows tested

## Open Questions & Decisions

### Resolved Decisions

**OQ-1: Item Conflict Handling During Edit**
- **Decision**: Last-write-wins (overwrite server version)
- **Rationale**: Matches TypeScript CLI behavior. API handles revision tracking.

**OQ-2: Item Data Validation Depth**
- **Decision**: Complete validation (required fields, types, formats, max lengths)
- **Rationale**: Prevent API round-trips with bad data. Better UX.

**OQ-3: Attachment Upload Efficiency**
- **Decision**: Single upload with progress bar (no resume)
- **Rationale**: Simpler for MVP. Most attachments are small. Can add chunking later if needed.

**OQ-4: Batch Operations Support**
- **Decision**: No batch support in MVP
- **Rationale**: Keep implementation simple. Users can script parallel execution if needed.

**OQ-5: Partial Update Failure Handling**
- **Decision**: Best-effort error handling (cache unchanged on API failure)
- **Rationale**: Simple, clear semantics. User can retry on failure.

**OQ-6: Delete Confirmation Behavior**
- **Decision**: Confirm only on `--permanent` delete
- **Rationale**: Soft delete is recoverable (trash safety net). Matches TypeScript CLI.

**OQ-7: Template Format**
- **Decision**: Complete template with all fields and defaults
- **Rationale**: Matches TypeScript CLI. Users can remove unneeded fields.

**OQ-8: Cache Update Atomicity**
- **Decision**: Atomic file write (temp file + rename)
- **Rationale**: Simple, reliable. Storage layer already supports atomic writes.

### Outstanding Questions

None - all architectural decisions resolved.

## Documentation Deliverables

### For Implementer

1. **This Implementation Plan** - Complete technical specification
2. **API Reference** - Endpoint documentation
3. **Data Model Reference** - Request/response structures
4. **Error Handling Guide** - Error types and handling patterns
5. **Testing Guide** - Unit and integration test patterns

### For End Users (Created by Documenter Agent)

1. **Command Reference** - Help text for all commands
2. **Item Creation Guide** - How to create different item types
3. **Validation Error Reference** - Common errors and solutions
4. **Destructive Operations Guide** - Safe delete practices
5. **Troubleshooting Guide** - Common issues and fixes

## Appendix: Code Examples

### Example: Complete Create Flow

```rust
// User input: JSON item data
let json_input = r#"{
  "type": 1,
  "name": "GitHub Login",
  "login": {
    "username": "user@example.com",
    "password": "secure_pass_123",
    "uris": [{"uri": "https://github.com"}]
  },
  "favorite": false
}"#;

// 1. Parse JSON to CipherView
let cipher_view: CipherView = serde_json::from_str(json_input)?;

// 2. Validate input
validation_service.validate_cipher_create(&cipher_view)?;

// 3. Encrypt using SDK
let encrypted_cipher = cipher_service.encrypt_cipher(&cipher_view).await?;

// 4. Send to API
let created: Cipher = api_client
    .post_with_auth("/api/ciphers", &encrypted_cipher)
    .await?;

// 5. Update cache atomically
storage.lock().await.add_cipher(&created).await?;

// 6. Return success
Ok(Response::success(created))
```

### Example: Template Generation

```rust
// Generate login item template
pub fn generate_login_template() -> CipherView {
    CipherView {
        id: None,
        r#type: CipherType::Login,
        name: String::new(),
        notes: None,
        favorite: false,
        folder_id: None,
        organization_id: None,
        login: Some(LoginView {
            username: None,
            password: None,
            uris: vec![],
            totp: None,
        }),
        fields: vec![],
        password_history: vec![],
        ..Default::default()
    }
}

// Output as JSON
let template = generate_login_template();
let json = serde_json::to_string_pretty(&template)?;
println!("{}", json);
```

---

## Summary

This implementation plan provides a comprehensive architecture for vault write operations, following established patterns from enhancements 1-5. The design prioritizes:

1. **Consistency** - Follows existing service-oriented architecture
2. **Safety** - Validation, confirmation, atomic operations
3. **Security** - SDK-only encryption, memory protection
4. **Reliability** - Pessimistic cache updates, error handling
5. **Usability** - Clear errors, TypeScript CLI compatibility

The phased approach allows for iterative development with clear exit criteria at each stage. The MVP (Phase 1) delivers core CRUD functionality in 10-14 days, with extended features and polish in subsequent phases.

**Recommended Next Step**: Begin implementation with Phase 1, Day 1-2 (Service Layer Setup).

---

**Implementation Plan Status**: ✅ COMPLETE
**Ready for Implementation**: YES
**Blockers**: None
