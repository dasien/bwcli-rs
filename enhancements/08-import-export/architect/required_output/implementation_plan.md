---
enhancement: 08-import-export
agent: architect
task_id: task_1764976164_8356
timestamp: 2025-12-05T00:00:00Z
status: READY_FOR_IMPLEMENTATION
---

# Implementation Plan: Import/Export Commands

## Executive Summary

This implementation plan details the architecture for the import/export feature, the final enhancement in the Bitwarden CLI Rust migration. The architecture prioritizes **data integrity**, **format compatibility**, and **extensibility** while maintaining security and performance.

**Key Architectural Principles:**
1. **Strategy Pattern** for format parsers (easy extensibility)
2. **Streaming Processing** for exports (memory efficiency)
3. **Fail-Fast Validation** for imports (data integrity)
4. **SDK Encryption** for all cryptographic operations
5. **Phased Implementation** (MVP first, then expansion)

## 1. System Architecture

### 1.1 High-Level Component Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     CLI Layer                                │
│  (commands/tools.rs: execute_export, execute_import)         │
└────────────────────┬────────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────────┐
│                  Service Coordinator                         │
│  ┌──────────────────────┐  ┌─────────────────────────────┐ │
│  │  ExportService       │  │  ImportService              │ │
│  │  - orchestrates      │  │  - orchestrates             │ │
│  │  - validates auth    │  │  - validates auth           │ │
│  │  - manages progress  │  │  - manages progress         │ │
│  └──────────────────────┘  └─────────────────────────────┘ │
└────────────────────┬────────────────────────────────────────┘
                     │
         ┌───────────┴───────────┐
         ▼                       ▼
┌──────────────────┐    ┌──────────────────┐
│ Export Pipeline  │    │ Import Pipeline  │
│                  │    │                  │
│ 1. Data Reader   │    │ 1. Parser        │
│ 2. Formatter     │    │ 2. Validator     │
│ 3. Encryptor     │    │ 3. Transformer   │
│ 4. Writer        │    │ 4. Encryptor     │
│                  │    │ 5. Writer        │
└──────────────────┘    └──────────────────┘
         │                       │
         ▼                       ▼
┌─────────────────────────────────────────────┐
│         Shared Infrastructure               │
│  ┌──────────┐  ┌──────────┐  ┌───────────┐│
│  │  Vault   │  │   SDK    │  │   API     ││
│  │ Service  │  │ Client   │  │  Client   ││
│  └──────────┘  └──────────┘  └───────────┘│
└─────────────────────────────────────────────┘
```

### 1.2 Module Organization

Create new modules in `crates/bw-core/src/services/`:

```
services/
├── import_export/
│   ├── mod.rs                    # Public API, re-exports
│   ├── export/
│   │   ├── mod.rs               # ExportService
│   │   ├── formatters/
│   │   │   ├── mod.rs           # Formatter trait + registry
│   │   │   ├── csv.rs           # CSV formatter
│   │   │   ├── json.rs          # JSON formatter
│   │   │   └── encrypted.rs     # Encrypted JSON formatter
│   │   ├── data_collector.rs   # Read vault data
│   │   └── writer.rs            # File/stdout writing
│   ├── import/
│   │   ├── mod.rs               # ImportService
│   │   ├── parsers/
│   │   │   ├── mod.rs           # Parser trait + registry
│   │   │   ├── bitwarden_csv.rs
│   │   │   ├── bitwarden_json.rs
│   │   │   ├── encrypted_json.rs
│   │   │   ├── lastpass.rs
│   │   │   ├── onepassword.rs
│   │   │   └── chrome.rs
│   │   ├── validator.rs         # Data validation
│   │   ├── transformer.rs       # Convert to CipherView
│   │   └── detector.rs          # Auto-detect format
│   └── errors.rs                # Shared error types
```

### 1.3 Data Flow Diagrams

#### Export Flow

```
┌─────────┐
│  User   │ bw export --format json --output backup.json
└────┬────┘
     │
     ▼
┌──────────────────────────────────────────────────┐
│ 1. Validate Authentication                       │
│    - Check session token exists                  │
│    - Verify not locked                           │
└────┬─────────────────────────────────────────────┘
     │
     ▼
┌──────────────────────────────────────────────────┐
│ 2. Data Collection (DataCollector)               │
│    - Read vault data from cache                  │
│    - Filter by organization if specified         │
│    - Decrypt ciphers (via CipherService)         │
│    - Stream processing for large vaults          │
└────┬─────────────────────────────────────────────┘
     │
     ▼
┌──────────────────────────────────────────────────┐
│ 3. Format Conversion (Formatter trait)           │
│    - Select formatter (CSV/JSON/Encrypted)       │
│    - Transform CipherView to export format       │
│    - Encrypt if needed (SDK)                     │
│    - Generate output string/bytes                │
└────┬─────────────────────────────────────────────┘
     │
     ▼
┌──────────────────────────────────────────────────┐
│ 4. Output Writing (Writer)                       │
│    - Write to file or stdout                     │
│    - Atomic file operations                      │
│    - Show warnings if unencrypted                │
└────┬─────────────────────────────────────────────┘
     │
     ▼
┌─────────┐
│ Success │ "Exported 150 items"
└─────────┘
```

#### Import Flow

```
┌─────────┐
│  User   │ bw import lastpass export.csv
└────┬────┘
     │
     ▼
┌──────────────────────────────────────────────────┐
│ 1. Validate Authentication                       │
│    - Check session token exists                  │
│    - Verify not locked                           │
└────┬─────────────────────────────────────────────┘
     │
     ▼
┌──────────────────────────────────────────────────┐
│ 2. Format Detection (Detector)                   │
│    - Auto-detect if format is "bitwarden*"       │
│    - Otherwise use specified format              │
│    - Select appropriate parser                   │
└────┬─────────────────────────────────────────────┘
     │
     ▼
┌──────────────────────────────────────────────────┐
│ 3. Parse Input (Parser trait)                    │
│    - Read file                                   │
│    - Parse according to format                   │
│    - Convert to intermediate ImportData struct   │
└────┬─────────────────────────────────────────────┘
     │
     ▼
┌──────────────────────────────────────────────────┐
│ 4. Validate Data (Validator)                     │
│    - Check all required fields present           │
│    - Validate data types                         │
│    - Check structure consistency                 │
│    - FAIL-FAST if validation errors              │
└────┬─────────────────────────────────────────────┘
     │
     ▼
┌──────────────────────────────────────────────────┐
│ 5. Transform to CipherView (Transformer)         │
│    - Convert ImportData to CipherView            │
│    - Create folders as needed                    │
│    - Map fields to Bitwarden structure           │
└────┬─────────────────────────────────────────────┘
     │
     ▼
┌──────────────────────────────────────────────────┐
│ 6. Import to Vault (WriteService)                │
│    - Create folders first (deduplicate)          │
│    - Batch cipher creation (100 per batch)       │
│    - Encrypt via SDK                             │
│    - API calls with retry logic                  │
│    - Update cache after successful import        │
└────┬─────────────────────────────────────────────┘
     │
     ▼
┌─────────┐
│ Success │ "Imported 150 items"
└─────────┘
```

## 2. Technical Specifications

### 2.1 Core Traits and Types

#### Export Formatter Trait

```rust
/// Trait for export format implementations
#[async_trait::async_trait]
pub trait ExportFormatter: Send + Sync {
    /// Format name (e.g., "csv", "json", "encrypted_json")
    fn format_name(&self) -> &str;

    /// File extension (e.g., "csv", "json")
    fn file_extension(&self) -> &str;

    /// Format vault data for export
    async fn format(
        &self,
        data: &ExportData,
        options: &ExportOptions,
    ) -> Result<Vec<u8>, ExportError>;

    /// Whether this format requires encryption password
    fn requires_password(&self) -> bool;

    /// Whether this is an encrypted format
    fn is_encrypted(&self) -> bool;
}

/// Export data structure (decrypted vault items)
pub struct ExportData {
    pub folders: Vec<FolderView>,
    pub ciphers: Vec<CipherView>,
    pub collections: Vec<CollectionView>,
}

/// Export options
pub struct ExportOptions {
    pub password: Option<String>,      // For encrypted exports
    pub organization_id: Option<String>,
}
```

#### Import Parser Trait

```rust
/// Trait for import format parsers
#[async_trait::async_trait]
pub trait ImportParser: Send + Sync {
    /// Format name (e.g., "bitwardenjson", "lastpass")
    fn format_name(&self) -> &str;

    /// Parse import file
    async fn parse(
        &self,
        data: &[u8],
        options: &ImportOptions,
    ) -> Result<ImportData, ImportError>;

    /// Check if this parser can handle the data (for auto-detection)
    fn can_parse(&self, data: &[u8]) -> bool;

    /// Whether this format requires decryption password
    fn requires_password(&self) -> bool;
}

/// Import data structure (intermediate format)
pub struct ImportData {
    pub folders: Vec<ImportFolder>,
    pub items: Vec<ImportItem>,
}

pub struct ImportFolder {
    pub name: String,
}

pub struct ImportItem {
    pub item_type: ImportItemType,
    pub folder_name: Option<String>,
    pub favorite: bool,
    pub name: String,
    pub notes: Option<String>,
    pub fields: Vec<ImportField>,
    pub login: Option<ImportLogin>,
    pub card: Option<ImportCard>,
    pub identity: Option<ImportIdentity>,
}

pub enum ImportItemType {
    Login,
    SecureNote,
    Card,
    Identity,
}

/// Import options
pub struct ImportOptions {
    pub password: Option<String>,      // For encrypted imports
    pub organization_id: Option<String>,
}
```

### 2.2 Service Interfaces

#### ExportService

```rust
/// Service for exporting vault data
pub struct ExportService {
    vault_service: Arc<VaultService>,
    cipher_service: Arc<CipherService>,
    storage: Arc<Mutex<JsonFileStorage>>,
    formatters: HashMap<String, Box<dyn ExportFormatter>>,
}

impl ExportService {
    pub fn new(/* dependencies */) -> Self {
        let mut formatters: HashMap<String, Box<dyn ExportFormatter>> = HashMap::new();
        formatters.insert("csv".to_string(), Box::new(CsvFormatter::new()));
        formatters.insert("json".to_string(), Box::new(JsonFormatter::new()));
        formatters.insert("encrypted_json".to_string(), Box::new(EncryptedJsonFormatter::new()));
        // ... register formatters

        Self { /* ... */ formatters }
    }

    /// Export vault to specified format
    pub async fn export(
        &self,
        format: &str,
        output_path: Option<&str>,
        options: ExportOptions,
    ) -> Result<ExportResult, ExportError>;

    /// List supported export formats
    pub fn supported_formats(&self) -> Vec<String>;
}

pub struct ExportResult {
    pub item_count: usize,
    pub format: String,
    pub output_path: Option<String>,
    pub encrypted: bool,
}
```

#### ImportService

```rust
/// Service for importing data into vault
pub struct ImportService {
    write_service: Arc<WriteService>,
    cipher_service: Arc<CipherService>,
    validator: Arc<ImportValidator>,
    transformer: Arc<ImportTransformer>,
    detector: Arc<FormatDetector>,
    parsers: HashMap<String, Box<dyn ImportParser>>,
}

impl ImportService {
    pub fn new(/* dependencies */) -> Self {
        let mut parsers: HashMap<String, Box<dyn ImportParser>> = HashMap::new();
        parsers.insert("bitwardencsv".to_string(), Box::new(BitwardenCsvParser::new()));
        parsers.insert("bitwardenjson".to_string(), Box::new(BitwardenJsonParser::new()));
        parsers.insert("encrypted_json".to_string(), Box::new(EncryptedJsonParser::new()));
        parsers.insert("lastpass".to_string(), Box::new(LastPassParser::new()));
        parsers.insert("1password".to_string(), Box::new(OnePasswordParser::new()));
        parsers.insert("chrome".to_string(), Box::new(ChromeParser::new()));
        // ... register parsers

        Self { /* ... */ parsers }
    }

    /// Import data from file
    pub async fn import(
        &self,
        format: &str,
        file_path: &str,
        options: ImportOptions,
    ) -> Result<ImportResult, ImportError>;

    /// List supported import formats
    pub fn supported_formats(&self) -> Vec<FormatInfo>;
}

pub struct ImportResult {
    pub items_created: usize,
    pub folders_created: usize,
    pub format: String,
}

pub struct FormatInfo {
    pub name: String,
    pub display_name: String,
    pub description: String,
}
```

### 2.3 Error Handling

```rust
/// Export-specific errors
#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("Not authenticated")]
    NotAuthenticated,

    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),

    #[error("Password required for encrypted export")]
    PasswordRequired,

    #[error("Failed to write output file: {0}")]
    FileWriteError(String),

    #[error("Failed to decrypt vault: {0}")]
    DecryptionError(String),

    #[error("Vault error: {0}")]
    VaultError(#[from] VaultError),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Import-specific errors
#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("Not authenticated")]
    NotAuthenticated,

    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),

    #[error("Failed to read import file: {0}")]
    FileReadError(String),

    #[error("Failed to parse import data: {0}")]
    ParseError(String),

    #[error("Validation failed: {errors:?}")]
    ValidationError {
        errors: Vec<ValidationError>,
    },

    #[error("Password required for encrypted import")]
    PasswordRequired,

    #[error("Failed to import items: {0}")]
    ImportFailed(String),

    #[error("Vault error: {0}")]
    VaultError(#[from] VaultError),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
}

pub struct ValidationError {
    pub line: Option<usize>,
    pub field: Option<String>,
    pub message: String,
}
```

## 3. Format Specifications

### 3.1 Export Formats

#### CSV Format

**Structure:** Match TypeScript CLI exactly

```csv
folder,favorite,type,name,notes,fields,reprompt,login_uri,login_username,login_password,login_totp
"Personal",1,"login","Example Site","My notes","custom_field_name: custom_field_value",0,"https://example.com","user@example.com","password123","otpauth://totp/..."
```

**Fields:**
- `folder`: Folder name (empty if no folder)
- `favorite`: 1 or 0
- `type`: login, note, card, identity
- `name`: Item name
- `notes`: Item notes
- `fields`: Custom fields as "name: value" (newline separated)
- `reprompt`: 1 or 0
- `login_uri`: Login URI (multiple URIs newline separated)
- `login_username`: Login username
- `login_password`: Login password
- `login_totp`: TOTP secret
- Card fields: `card_cardholderName`, `card_brand`, `card_number`, `card_expMonth`, `card_expYear`, `card_code`
- Identity fields: `identity_title`, `identity_firstName`, `identity_middleName`, `identity_lastName`, `identity_address1`, etc.

**Implementation Notes:**
- Use `csv` crate for serialization
- Quote all fields (even if no special characters)
- Handle newlines within notes/fields by quoting
- Match TypeScript CLI field order exactly

#### JSON Format

**Structure:** Direct serialization of decrypted vault

```json
{
  "encrypted": false,
  "folders": [
    {
      "id": "uuid",
      "name": "Personal"
    }
  ],
  "items": [
    {
      "id": "uuid",
      "organizationId": null,
      "folderId": "uuid",
      "type": 1,
      "reprompt": 0,
      "name": "Example Site",
      "notes": "My notes",
      "favorite": true,
      "login": {
        "username": "user@example.com",
        "password": "password123",
        "totp": "otpauth://...",
        "uris": [
          {
            "match": null,
            "uri": "https://example.com"
          }
        ]
      },
      "collectionIds": [],
      "revisionDate": "2025-12-05T00:00:00.000Z",
      "creationDate": "2025-12-05T00:00:00.000Z",
      "deletedDate": null
    }
  ]
}
```

**Implementation Notes:**
- Use `serde_json` for serialization
- Pretty-print JSON (indent 2 spaces)
- Include `encrypted: false` field
- Match TypeScript CLI structure exactly
- Serialize all timestamps as ISO 8601

#### Encrypted JSON Format

**Structure:** Encrypted version of JSON format

```json
{
  "encrypted": true,
  "encKeyValidation_DO_NOT_EDIT": "2.base64encstring...",
  "data": "2.base64encstring..."
}
```

**Encryption Process:**
1. Serialize vault to JSON (same as JSON format but without `encrypted` field)
2. Derive encryption key from password using PBKDF2-SHA256 (100,000 iterations)
3. Generate random 16-byte IV
4. Encrypt JSON with AES-256-CBC
5. Create EncString format: `type.iv|ciphertext|mac`
6. Add key validation EncString (encrypt known value)

**Implementation Notes:**
- Use Bitwarden SDK for all encryption operations
- Store KDF parameters in export (iterations, memory, parallelism)
- Validate password strength (warn if < 12 chars)
- Include `encKeyValidation_DO_NOT_EDIT` for password verification

### 3.2 Import Formats

#### Bitwarden CSV

Same structure as export CSV. Parser must handle:
- Quoted and unquoted fields
- Newlines within quoted fields
- Multiple URIs separated by newlines
- Custom fields in "name: value" format
- All cipher types (login, note, card, identity)

#### Bitwarden JSON

Same structure as export JSON. Parser must handle:
- `encrypted: false` format
- All cipher types
- Missing optional fields
- Collections and folders

#### Encrypted JSON

Must decrypt using password, then parse as Bitwarden JSON.

**Decryption Process:**
1. Parse JSON structure
2. Extract `encKeyValidation_DO_NOT_EDIT` and `data`
3. Prompt for password
4. Derive key from password (same KDF as encryption)
5. Decrypt validation string to verify password
6. Decrypt data
7. Parse decrypted JSON

#### LastPass Format

**CSV Structure:**
```csv
url,username,password,extra,name,grouping,fav
https://example.com,user@example.com,password123,"notes here","Example Site","Personal",0
```

**Mapping:**
- `url` → `login.uris[0].uri`
- `username` → `login.username`
- `password` → `login.password`
- `extra` → `notes`
- `name` → `name`
- `grouping` → folder name
- `fav` → `favorite` (1 = true)

#### 1Password Format

**CSV Structure:**
```csv
Title,Website,Username,Password,Notes,Type,Folder
"Example Site","https://example.com","user@example.com","password123","notes","Login","Personal"
```

**Mapping:**
- `Title` → `name`
- `Website` → `login.uris[0].uri`
- `Username` → `login.username`
- `Password` → `login.password`
- `Notes` → `notes`
- `Type` → map to CipherType (Login, Secure Note, etc.)
- `Folder` → folder name

#### Chrome Passwords Format

**CSV Structure:**
```csv
name,url,username,password
"Example Site","https://example.com","user@example.com","password123"
```

**Mapping:**
- `name` → `name`
- `url` → `login.uris[0].uri`
- `username` → `login.username`
- `password` → `login.password`
- All items are type "login"
- No folders

### 3.3 Format Detection

Auto-detect Bitwarden formats by examining file structure:

```rust
pub struct FormatDetector;

impl FormatDetector {
    /// Detect format from file contents
    pub fn detect(&self, data: &[u8]) -> Option<String> {
        // Try to parse as JSON
        if let Ok(json) = serde_json::from_slice::<serde_json::Value>(data) {
            // Check for encrypted JSON
            if json.get("encrypted") == Some(&serde_json::Value::Bool(true)) {
                return Some("encrypted_json".to_string());
            }

            // Check for Bitwarden JSON structure
            if json.get("items").is_some() && json.get("folders").is_some() {
                return Some("bitwardenjson".to_string());
            }
        }

        // Try to parse as CSV
        if let Ok(csv_str) = std::str::from_utf8(data) {
            let first_line = csv_str.lines().next()?;

            // Check for Bitwarden CSV header
            if first_line.contains("login_uri") && first_line.contains("login_username") {
                return Some("bitwardencsv".to_string());
            }
        }

        None
    }
}
```

## 4. Integration Strategy

### 4.1 Dependencies on Existing Services

#### VaultService (Enhancement 5)

**Used by:** ExportService

**Required Operations:**
- `list_all_ciphers()` - Get all vault items
- `list_folders()` - Get all folders
- `list_collections()` - Get all collections
- Filter by organization ID

**Integration Pattern:**
```rust
// In ExportService::collect_data()
let vault_service = VaultService::new(/* ... */);
let ciphers = vault_service.list_all_ciphers().await?;
let folders = vault_service.list_folders().await?;
let collections = vault_service.list_collections().await?;
```

#### WriteService (Enhancement 6)

**Used by:** ImportService

**Required Operations:**
- `create_cipher()` - Create vault items
- `create_folder()` - Create folders
- Batch operations for efficiency

**Integration Pattern:**
```rust
// In ImportService::import()
let write_service = WriteService::new(/* ... */);

// Create folders first (deduplicate by name)
for folder in import_data.folders {
    write_service.create_folder(folder).await?;
}

// Create ciphers in batches
for batch in import_data.items.chunks(100) {
    for item in batch {
        write_service.create_cipher(item).await?;
    }
}
```

#### CipherService

**Used by:** ExportService, ImportService

**Required Operations:**
- `decrypt_cipher()` - Decrypt for export
- `encrypt_cipher()` - Encrypt for import
- `decrypt_string()` - For encrypted exports
- `encrypt_string()` - For encrypted exports

**Integration Pattern:**
```rust
// Export: decrypt all ciphers
let decrypted = cipher_service.decrypt_ciphers(&ciphers).await?;

// Import: encrypt before creating
let encrypted = cipher_service.encrypt_cipher(&cipher_view).await?;
```

#### SDK Client

**Used by:** EncryptedJsonFormatter, EncryptedJsonParser

**Required Operations:**
- Key derivation (PBKDF2-SHA256)
- AES-256-CBC encryption/decryption
- EncString format handling

**Integration Pattern:**
```rust
// Derive key from password
let key = sdk_client.derive_key(&password, &salt, iterations).await?;

// Encrypt data
let encrypted = sdk_client.encrypt(&data, &key).await?;

// Decrypt data
let decrypted = sdk_client.decrypt(&encrypted, &key).await?;
```

### 4.2 Storage Integration

Export and import do not directly modify storage except through existing services:

- **Export:** Reads from vault cache (via VaultService)
- **Import:** Updates cache after creation (via WriteService)

**Sync Before Export:**
```rust
// In ExportService::export()
// Force sync to ensure latest data
let sync_service = SyncService::new(/* ... */);
sync_service.sync(true).await?;

// Then collect data
let data = self.collect_data(options).await?;
```

### 4.3 API Client Integration

Import requires API calls to create items:

**Batch Strategy:**
```rust
// Create items in batches to manage API rate limits
const BATCH_SIZE: usize = 100;
const BATCH_DELAY_MS: u64 = 100;

for (i, batch) in items.chunks(BATCH_SIZE).enumerate() {
    // Create batch
    for item in batch {
        write_service.create_cipher(item).await?;
    }

    // Progress indication
    progress_bar.set_position((i + 1) * BATCH_SIZE);

    // Small delay between batches
    tokio::time::sleep(Duration::from_millis(BATCH_DELAY_MS)).await;
}
```

**Retry Logic:**
```rust
// Implement retry for transient failures
let result = retry_async(
    || write_service.create_cipher(item),
    3, // max retries
    Duration::from_millis(1000), // initial delay
).await?;
```

## 5. Data Validation Strategy

### 5.1 Export Validation

**Pre-Export Checks:**
```rust
pub struct ExportValidator;

impl ExportValidator {
    pub fn validate_export_request(
        &self,
        format: &str,
        options: &ExportOptions,
    ) -> Result<(), ExportError> {
        // Check format is supported
        if !SUPPORTED_FORMATS.contains(&format) {
            return Err(ExportError::UnsupportedFormat(format.to_string()));
        }

        // Check password provided if required
        if format == "encrypted_json" && options.password.is_none() {
            return Err(ExportError::PasswordRequired);
        }

        // Warn if password is weak
        if let Some(password) = &options.password {
            if password.len() < 12 {
                tracing::warn!("Password is weak (< 12 characters)");
            }
        }

        Ok(())
    }
}
```

**Security Warnings:**
```rust
// Warn for unencrypted exports
if !formatter.is_encrypted() {
    eprintln!("⚠️  WARNING: This export is NOT encrypted.");
    eprintln!("⚠️  Your vault data will be stored in PLAIN TEXT.");
    eprintln!("⚠️  Ensure the output file is stored securely.");
    eprintln!();
}
```

### 5.2 Import Validation

**Validation Pipeline:**
```rust
pub struct ImportValidator;

impl ImportValidator {
    /// Validate import data structure
    pub fn validate(&self, data: &ImportData) -> Result<(), ImportError> {
        let mut errors = Vec::new();

        // Validate each item
        for (i, item) in data.items.iter().enumerate() {
            // Name required
            if item.name.trim().is_empty() {
                errors.push(ValidationError {
                    line: Some(i + 1),
                    field: Some("name".to_string()),
                    message: "Name is required".to_string(),
                });
            }

            // Type-specific validation
            match item.item_type {
                ImportItemType::Login => {
                    self.validate_login(&item.login, i + 1, &mut errors);
                }
                ImportItemType::Card => {
                    self.validate_card(&item.card, i + 1, &mut errors);
                }
                ImportItemType::Identity => {
                    self.validate_identity(&item.identity, i + 1, &mut errors);
                }
                _ => {}
            }
        }

        // Fail-fast if any errors
        if !errors.is_empty() {
            return Err(ImportError::ValidationError { errors });
        }

        Ok(())
    }

    fn validate_login(
        &self,
        login: &Option<ImportLogin>,
        line: usize,
        errors: &mut Vec<ValidationError>,
    ) {
        if let Some(login) = login {
            // At least username or password should be present
            if login.username.is_none() && login.password.is_none() {
                errors.push(ValidationError {
                    line: Some(line),
                    field: Some("login".to_string()),
                    message: "Login must have username or password".to_string(),
                });
            }

            // Validate URIs if present
            for (i, uri) in login.uris.iter().enumerate() {
                if uri.trim().is_empty() {
                    errors.push(ValidationError {
                        line: Some(line),
                        field: Some(format!("login.uri[{}]", i)),
                        message: "URI cannot be empty".to_string(),
                    });
                }
            }
        }
    }
}
```

**Validation Error Display:**
```rust
// In execute_import()
match import_service.import(format, file, options).await {
    Err(ImportError::ValidationError { errors }) => {
        eprintln!("❌ Validation failed with {} errors:\n", errors.len());
        for error in errors {
            if let Some(line) = error.line {
                eprint!("  Line {}: ", line);
            }
            if let Some(field) = error.field {
                eprint!("{}: ", field);
            }
            eprintln!("{}", error.message);
        }
        eprintln!("\nNo items were imported. Please fix the errors and try again.");
        return Ok(Response::error("Validation failed"));
    }
    // ... other errors
}
```

## 6. Progress Indication

### 6.1 Progress Bar Implementation

Use `indicatif` crate (already in dependencies):

```rust
use indicatif::{ProgressBar, ProgressStyle};

pub struct ProgressReporter {
    bar: Option<ProgressBar>,
    quiet: bool,
}

impl ProgressReporter {
    pub fn new(total: usize, quiet: bool, no_interaction: bool) -> Self {
        // No progress bar if quiet or no-interaction
        if quiet || no_interaction {
            return Self { bar: None, quiet: true };
        }

        // No progress bar for small operations
        if total < 100 {
            return Self { bar: None, quiet: false };
        }

        let bar = ProgressBar::new(total as u64);
        bar.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
                .unwrap()
                .progress_chars("=>-"),
        );

        Self { bar: Some(bar), quiet: false }
    }

    pub fn set_message(&self, msg: &str) {
        if let Some(bar) = &self.bar {
            bar.set_message(msg.to_string());
        }
    }

    pub fn inc(&self, delta: usize) {
        if let Some(bar) = &self.bar {
            bar.inc(delta as u64);
        }
    }

    pub fn finish_with_message(&self, msg: &str) {
        if let Some(bar) = &self.bar {
            bar.finish_with_message(msg.to_string());
        } else if !self.quiet {
            println!("{}", msg);
        }
    }
}
```

### 6.2 Progress in Export

```rust
// In ExportService::export()
let progress = ProgressReporter::new(
    ciphers.len(),
    global_args.quiet,
    global_args.nointeraction,
);

progress.set_message("Decrypting vault items...");

let mut decrypted = Vec::new();
for cipher in ciphers {
    let decrypted_cipher = cipher_service.decrypt_cipher(&cipher).await?;
    decrypted.push(decrypted_cipher);
    progress.inc(1);
}

progress.set_message("Formatting export...");
let formatted = formatter.format(&data, &options).await?;

progress.finish_with_message("Export complete");
```

### 6.3 Progress in Import

```rust
// In ImportService::import()
let progress = ProgressReporter::new(
    import_data.items.len(),
    global_args.quiet,
    global_args.nointeraction,
);

progress.set_message("Creating folders...");
for folder in folders {
    write_service.create_folder(folder).await?;
}

progress.set_message("Importing items...");
for item in import_data.items {
    write_service.create_cipher(item).await?;
    progress.inc(1);
}

progress.finish_with_message(&format!("Imported {} items", import_data.items.len()));
```

## 7. Security Considerations

### 7.1 Sensitive Data Handling

**Memory Security:**
```rust
use secrecy::{Secret, ExposeSecret};
use zeroize::Zeroize;

// Store passwords as secrets
pub struct ExportOptions {
    pub password: Option<Secret<String>>,
    // ...
}

// Zeroize temporary decrypted data
let mut decrypted_data = decrypt_vault();
// ... use data ...
decrypted_data.zeroize();
```

**No Logging of Sensitive Data:**
```rust
// NEVER log passwords, vault data, or decrypted content
tracing::info!("Exporting {} items", count); // OK
// tracing::debug!("Export data: {:?}", data); // NEVER DO THIS
```

### 7.2 Export Security

**File Permission Warnings:**
```rust
// Check output file permissions (Unix only)
#[cfg(unix)]
fn check_file_permissions(path: &Path) -> Result<(), ExportError> {
    use std::os::unix::fs::PermissionsExt;

    if path.exists() {
        let metadata = path.metadata()?;
        let permissions = metadata.permissions();
        let mode = permissions.mode();

        // Warn if file is world-readable (mode & 0o004)
        if mode & 0o004 != 0 {
            eprintln!("⚠️  WARNING: Output file is world-readable");
            eprintln!("⚠️  Consider restricting permissions: chmod 600 {}", path.display());
        }
    }

    Ok(())
}
```

**Unencrypted Export Warnings:**
```rust
if !formatter.is_encrypted() {
    eprintln!("⚠️  WARNING: This export is NOT encrypted.");
    eprintln!("⚠️  Your vault data will be stored in PLAIN TEXT.");
    eprintln!("⚠️  Anyone with access to this file can read your passwords.");
    eprintln!();

    // Require confirmation in interactive mode
    if !global_args.nointeraction {
        if !dialoguer::Confirm::new()
            .with_prompt("Continue with unencrypted export?")
            .default(false)
            .interact()?
        {
            return Err(ExportError::OperationCancelled);
        }
    }
}
```

### 7.3 Import Security

**Validate Input Files:**
```rust
// Check file size (reject files > 100MB)
const MAX_FILE_SIZE: u64 = 100 * 1024 * 1024;

fn validate_import_file(path: &Path) -> Result<(), ImportError> {
    let metadata = path.metadata()?;

    if metadata.len() > MAX_FILE_SIZE {
        return Err(ImportError::FileTooLarge {
            size: metadata.len(),
            max: MAX_FILE_SIZE,
        });
    }

    Ok(())
}
```

**Encrypt Before Upload:**
```rust
// All data encrypted via SDK before API calls
let encrypted = cipher_service.encrypt_cipher(&cipher_view).await?;
let request = CipherRequest::from(encrypted);
api_client.post_with_auth("/api/ciphers", &request).await?;
```

## 8. Performance Optimization

### 8.1 Streaming Export

For large vaults, stream processing avoids loading entire vault in memory:

```rust
/// Stream-based CSV export
pub struct CsvFormatter;

impl CsvFormatter {
    pub async fn format_stream(
        &self,
        ciphers: impl Stream<Item = CipherView>,
        writer: &mut impl Write,
    ) -> Result<(), ExportError> {
        let mut csv_writer = csv::Writer::from_writer(writer);

        // Write header
        csv_writer.write_record(&[
            "folder", "favorite", "type", "name", "notes",
            "login_uri", "login_username", "login_password",
            // ... all fields
        ])?;

        // Stream ciphers
        tokio::pin!(ciphers);
        while let Some(cipher) = ciphers.next().await {
            let record = self.cipher_to_csv_record(&cipher);
            csv_writer.write_record(&record)?;
        }

        csv_writer.flush()?;
        Ok(())
    }
}
```

### 8.2 Batch Import

Import items in batches to improve performance:

```rust
const BATCH_SIZE: usize = 100;

// Group items into batches
for batch in import_data.items.chunks(BATCH_SIZE) {
    // Create batch concurrently
    let mut handles = Vec::new();
    for item in batch {
        let write_service = Arc::clone(&self.write_service);
        let item = item.clone();

        let handle = tokio::spawn(async move {
            write_service.create_cipher(item).await
        });
        handles.push(handle);
    }

    // Wait for batch completion
    for handle in handles {
        handle.await??;
    }

    progress.inc(batch.len());
}
```

**Note:** Parallel requests must respect API rate limits. Start with sequential, add parallelism if needed.

### 8.3 Memory Optimization

**Streaming Decryption:**
```rust
// Don't load entire vault at once
// Instead, decrypt in chunks
const DECRYPT_CHUNK_SIZE: usize = 100;

let mut decrypted = Vec::new();
for chunk in ciphers.chunks(DECRYPT_CHUNK_SIZE) {
    let decrypted_chunk = cipher_service.decrypt_ciphers(chunk).await?;
    decrypted.extend(decrypted_chunk);

    // Allow other tasks to run
    tokio::task::yield_now().await;
}
```

**Drop Large Structures Early:**
```rust
{
    let formatted_data = formatter.format(&export_data, &options).await?;
    writer.write_all(&formatted_data)?;
    // formatted_data dropped here, freeing memory
}

// export_data also dropped when out of scope
```

## 9. Testing Strategy

### 9.1 Unit Tests

**Formatter Tests:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_csv_formatter_single_login() {
        let formatter = CsvFormatter::new();
        let data = create_test_export_data();
        let options = ExportOptions::default();

        let result = formatter.format(&data, &options).await.unwrap();
        let csv_str = String::from_utf8(result).unwrap();

        // Verify CSV structure
        assert!(csv_str.contains("folder,favorite,type"));
        assert!(csv_str.contains("Example Site"));
    }

    #[tokio::test]
    async fn test_json_formatter_structure() {
        let formatter = JsonFormatter::new();
        let data = create_test_export_data();
        let options = ExportOptions::default();

        let result = formatter.format(&data, &options).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&result).unwrap();

        // Verify JSON structure
        assert_eq!(json["encrypted"], false);
        assert!(json["items"].is_array());
        assert!(json["folders"].is_array());
    }

    #[tokio::test]
    async fn test_encrypted_json_roundtrip() {
        let formatter = EncryptedJsonFormatter::new();
        let parser = EncryptedJsonParser::new();

        let data = create_test_export_data();
        let password = "test_password_12345".to_string();
        let export_options = ExportOptions {
            password: Some(password.clone()),
            organization_id: None,
        };

        // Export
        let encrypted = formatter.format(&data, &export_options).await.unwrap();

        // Import
        let import_options = ImportOptions {
            password: Some(password),
            organization_id: None,
        };
        let imported = parser.parse(&encrypted, &import_options).await.unwrap();

        // Verify round-trip
        assert_eq!(imported.items.len(), data.ciphers.len());
    }
}
```

**Parser Tests:**
```rust
#[tokio::test]
async fn test_lastpass_parser() {
    let parser = LastPassParser::new();
    let csv_data = r#"url,username,password,extra,name,grouping,fav
https://example.com,user@example.com,pass123,"notes","Example","Personal",1
"#;

    let import_data = parser.parse(csv_data.as_bytes(), &ImportOptions::default())
        .await
        .unwrap();

    assert_eq!(import_data.items.len(), 1);
    assert_eq!(import_data.items[0].name, "Example");
    assert_eq!(import_data.folders.len(), 1);
    assert_eq!(import_data.folders[0].name, "Personal");
}
```

**Validator Tests:**
```rust
#[test]
fn test_validator_rejects_empty_name() {
    let validator = ImportValidator::new();
    let mut data = create_test_import_data();
    data.items[0].name = "".to_string();

    let result = validator.validate(&data);
    assert!(result.is_err());

    if let Err(ImportError::ValidationError { errors }) = result {
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("Name is required"));
    }
}
```

### 9.2 Integration Tests

**Round-Trip Tests:**
```rust
#[tokio::test]
async fn test_csv_roundtrip() {
    // Setup test environment
    let (export_service, import_service) = setup_test_services().await;

    // Create test vault
    create_test_vault_items().await;

    // Export to CSV
    let temp_file = tempfile::NamedTempFile::new().unwrap();
    let export_result = export_service.export(
        "csv",
        Some(temp_file.path().to_str().unwrap()),
        ExportOptions::default(),
    ).await.unwrap();

    // Clear vault
    clear_vault().await;

    // Import from CSV
    let import_result = import_service.import(
        "bitwardencsv",
        temp_file.path().to_str().unwrap(),
        ImportOptions::default(),
    ).await.unwrap();

    // Verify data integrity
    assert_eq!(export_result.item_count, import_result.items_created);
    verify_vault_items_match_original().await;
}
```

**Cross-Compatibility Tests:**
```rust
#[tokio::test]
async fn test_typescript_cli_compatibility() {
    // Load TypeScript CLI export
    let ts_export = load_test_file("typescript_cli_export.json");

    // Parse with Rust parser
    let parser = BitwardenJsonParser::new();
    let import_data = parser.parse(&ts_export, &ImportOptions::default())
        .await
        .unwrap();

    // Verify all items parsed correctly
    assert_eq!(import_data.items.len(), 150);
    // ... detailed assertions
}
```

**Error Handling Tests:**
```rust
#[tokio::test]
async fn test_import_handles_invalid_csv() {
    let import_service = setup_test_import_service().await;
    let invalid_csv = "invalid,csv,data\n";

    let temp_file = create_temp_file(invalid_csv);
    let result = import_service.import(
        "bitwardencsv",
        temp_file.path().to_str().unwrap(),
        ImportOptions::default(),
    ).await;

    assert!(result.is_err());
}
```

### 9.3 Performance Benchmarks

```rust
#[cfg(test)]
mod benchmarks {
    use super::*;

    #[tokio::test]
    async fn bench_export_1000_items() {
        let export_service = setup_test_export_service().await;
        create_test_vault_items(1000).await;

        let start = std::time::Instant::now();
        let result = export_service.export(
            "json",
            None,
            ExportOptions::default(),
        ).await.unwrap();
        let duration = start.elapsed();

        // Target: < 10 seconds for 1000 items
        assert!(duration.as_secs() < 10, "Export took {:?}", duration);
        println!("Export 1000 items: {:?}", duration);
    }

    #[tokio::test]
    async fn bench_import_1000_items() {
        let import_service = setup_test_import_service().await;
        let test_file = create_large_test_file(1000);

        let start = std::time::Instant::now();
        let result = import_service.import(
            "bitwardenjson",
            &test_file,
            ImportOptions::default(),
        ).await.unwrap();
        let duration = start.elapsed();

        // Target: < 30 seconds for 1000 items
        assert!(duration.as_secs() < 30, "Import took {:?}", duration);
        println!("Import 1000 items: {:?}", duration);
    }
}
```

## 10. Migration Path & Backwards Compatibility

### 10.1 TypeScript CLI Compatibility

**Export Format Compatibility:**
- CSV format MUST match byte-for-byte for same data
- JSON format MUST have identical structure
- Encrypted JSON MUST be interoperable

**Verification Strategy:**
```rust
// Cross-compatibility test
#[tokio::test]
async fn test_rust_export_ts_import() {
    // 1. Create test vault in Rust CLI
    // 2. Export with Rust CLI
    // 3. Import with TypeScript CLI
    // 4. Verify data integrity
}

#[tokio::test]
async fn test_ts_export_rust_import() {
    // 1. Load TypeScript CLI export
    // 2. Import with Rust CLI
    // 3. Verify data integrity
}
```

### 10.2 Configuration & Flags

Match TypeScript CLI flag names exactly:

**Export Command:**
```bash
bw export [options]
  --format <format>           Format: csv, json, encrypted_json (default: csv)
  --password <password>       Password for encrypted export
  --output <file>             Output file path (default: stdout)
  --organizationid <id>       Export organization vault
```

**Import Command:**
```bash
bw import <format> <file> [options]
  --organizationid <id>       Import to organization vault
  --formats                   List supported formats
```

### 10.3 Phased Rollout

**Phase 1: MVP (Week 1-2)**
- Bitwarden CSV export/import
- Bitwarden JSON export/import
- Encrypted JSON export/import
- Basic validation
- Progress indication

**Phase 2: Security & Org (Week 3)**
- Organization export/import
- Security warnings
- Enhanced validation
- File permission checks

**Phase 3: Additional Formats (Week 4-5)**
- LastPass import
- 1Password import
- Chrome import
- Format auto-detection

**Phase 4: Polish (Week 6)**
- Performance optimization
- Error message improvements
- Comprehensive tests
- Documentation

## 11. Command Line Interface

### 11.1 Export Command Implementation

```rust
// In commands/tools.rs

pub async fn execute_export(
    cmd: ExportCommand,
    global_args: &GlobalArgs,
) -> anyhow::Result<Response> {
    // Get format (default to csv)
    let format = cmd.format.as_deref().unwrap_or("csv");

    // Create export service
    let container = ServiceContainer::new().await?;
    let export_service = container.export_service();

    // Security warnings for unencrypted exports
    if format != "encrypted_json" && !global_args.quiet {
        eprintln!("⚠️  WARNING: This export is NOT encrypted.");
        eprintln!("⚠️  Your vault data will be stored in PLAIN TEXT.");

        if !global_args.nointeraction {
            if !dialoguer::Confirm::new()
                .with_prompt("Continue?")
                .default(false)
                .interact()?
            {
                return Ok(Response::error("Export cancelled"));
            }
        }
    }

    // Build options
    let options = ExportOptions {
        password: cmd.password.map(Secret::new),
        organization_id: cmd.organizationid,
    };

    // Execute export
    let result = export_service
        .export(format, cmd.output.as_deref(), options)
        .await?;

    // Format response
    if global_args.response {
        Ok(Response::success_json(serde_json::json!({
            "success": true,
            "data": {
                "itemCount": result.item_count,
                "format": result.format,
                "encrypted": result.encrypted,
            }
        })))
    } else if global_args.quiet {
        Ok(Response::success_raw(""))
    } else {
        Ok(Response::success_raw(format!(
            "Exported {} items to {}",
            result.item_count,
            result.output_path.as_deref().unwrap_or("stdout")
        )))
    }
}
```

### 11.2 Import Command Implementation

```rust
pub async fn execute_import(
    cmd: ImportCommand,
    global_args: &GlobalArgs,
) -> anyhow::Result<Response> {
    // Create import service
    let container = ServiceContainer::new().await?;
    let import_service = container.import_service();

    // Check if format is "formats" (list formats)
    if cmd.format == "formats" || cmd.format == "--formats" {
        let formats = import_service.supported_formats();

        if global_args.response {
            Ok(Response::success_json(serde_json::json!({
                "data": formats
            })))
        } else {
            println!("Supported import formats:\n");
            for format in formats {
                println!("  {} - {}", format.name, format.display_name);
                println!("    {}", format.description);
                println!();
            }
            Ok(Response::success_raw(""))
        }
    } else {
        // Build options
        let options = ImportOptions {
            password: None, // TODO: Prompt if needed
            organization_id: cmd.organizationid,
        };

        // Execute import
        let result = import_service
            .import(&cmd.format, &cmd.file, options)
            .await
            .map_err(|e| match e {
                ImportError::ValidationError { errors } => {
                    eprintln!("❌ Validation failed with {} errors:\n", errors.len());
                    for error in errors {
                        if let Some(line) = error.line {
                            eprint!("  Line {}: ", line);
                        }
                        if let Some(field) = error.field {
                            eprint!("{}: ", field);
                        }
                        eprintln!("{}", error.message);
                    }
                    eprintln!("\nNo items were imported.");
                    anyhow::anyhow!("Validation failed")
                }
                _ => anyhow::anyhow!(e),
            })?;

        // Format response
        if global_args.response {
            Ok(Response::success_json(serde_json::json!({
                "success": true,
                "data": {
                    "itemsCreated": result.items_created,
                    "foldersCreated": result.folders_created,
                    "format": result.format,
                }
            })))
        } else {
            Ok(Response::success_raw(format!(
                "Imported {} items ({} folders created)",
                result.items_created,
                result.folders_created
            )))
        }
    }
}
```

### 11.3 Service Container Integration

Add export/import services to container:

```rust
// In services/container.rs

pub struct ServiceContainer {
    // ... existing services
    export_service: Arc<ExportService>,
    import_service: Arc<ImportService>,
}

impl ServiceContainer {
    pub async fn new() -> Result<Self, ContainerError> {
        // ... existing initialization

        let export_service = Arc::new(ExportService::new(
            Arc::clone(&vault_service),
            Arc::clone(&cipher_service),
            Arc::clone(&storage),
        ));

        let import_service = Arc::new(ImportService::new(
            Arc::clone(&write_service),
            Arc::clone(&cipher_service),
            Arc::clone(&validation_service),
        ));

        Ok(Self {
            // ... existing fields
            export_service,
            import_service,
        })
    }

    pub fn export_service(&self) -> &Arc<ExportService> {
        &self.export_service
    }

    pub fn import_service(&self) -> &Arc<ImportService> {
        &self.import_service
    }
}
```

## 12. Cargo Dependencies

Add required dependencies to `Cargo.toml`:

```toml
[workspace.dependencies]
# CSV processing
csv = "1.3"

# Existing dependencies already available:
# - serde / serde_json (JSON)
# - indicatif (progress bars)
# - secrecy (sensitive data)
# - base64 (encoding)
# - dialoguer (confirmations)
```

## 13. Documentation Requirements

### 13.1 User Documentation

Create documentation files:

**docs/commands/export.md:**
```markdown
# bw export

Export your vault data to a file or stdout.

## Usage

```bash
bw export [options]
```

## Options

- `--format <format>` - Export format: csv, json, encrypted_json (default: csv)
- `--password <password>` - Password for encrypted export
- `--output <file>` - Output file path (default: stdout)
- `--organizationid <id>` - Export organization vault

## Examples

Export to CSV:
```bash
bw export --format csv --output backup.csv
```

Export to encrypted JSON:
```bash
bw export --format encrypted_json --password mypassword --output backup.json
```

## Security Warning

Unencrypted exports contain your passwords in plain text. Always use `encrypted_json` format or store exports securely.
```

**docs/commands/import.md:**
```markdown
# bw import

Import data from other password managers or Bitwarden exports.

## Usage

```bash
bw import <format> <file> [options]
```

## Supported Formats

- `bitwardencsv` - Bitwarden CSV export
- `bitwardenjson` - Bitwarden JSON export
- `encrypted_json` - Encrypted Bitwarden JSON
- `lastpass` - LastPass CSV export
- `1password` - 1Password CSV export
- `chrome` - Chrome passwords CSV

List all formats:
```bash
bw import --formats
```

## Examples

Import Bitwarden JSON:
```bash
bw import bitwardenjson backup.json
```

Import from LastPass:
```bash
bw import lastpass lastpass_export.csv
```

## Notes

- All items are created as new (duplicates are allowed)
- Folders are automatically created
- Validation errors will prevent import
```

### 13.2 API Documentation

Add comprehensive doc comments to all public APIs:

```rust
/// Export service for exporting vault data to various formats
///
/// # Features
/// - Multiple export formats (CSV, JSON, Encrypted JSON)
/// - Streaming processing for large vaults
/// - Automatic encryption with SDK
/// - Progress indication
///
/// # Security
/// - Warns for unencrypted exports
/// - Checks file permissions
/// - Zeroizes sensitive data after use
///
/// # Example
/// ```rust
/// let export_service = ExportService::new(/* ... */);
/// let result = export_service.export(
///     "json",
///     Some("backup.json"),
///     ExportOptions::default(),
/// ).await?;
/// println!("Exported {} items", result.item_count);
/// ```
pub struct ExportService {
    // ...
}
```

## 14. Implementation Checklist

### Phase 1: Foundation (MVP Core)

**Export:**
- [ ] Create `services/import_export/` module structure
- [ ] Implement `ExportService` core
- [ ] Implement `ExportFormatter` trait
- [ ] Implement `CsvFormatter`
- [ ] Implement `JsonFormatter`
- [ ] Implement `DataCollector` (read vault data)
- [ ] Implement file/stdout writer
- [ ] Add progress indication
- [ ] Update `execute_export()` in commands
- [ ] Add export unit tests

**Import:**
- [ ] Implement `ImportService` core
- [ ] Implement `ImportParser` trait
- [ ] Implement `BitwardenCsvParser`
- [ ] Implement `BitwardenJsonParser`
- [ ] Implement `ImportValidator`
- [ ] Implement `ImportTransformer`
- [ ] Add progress indication
- [ ] Update `execute_import()` in commands
- [ ] Add import unit tests

**Integration:**
- [ ] Add services to `ServiceContainer`
- [ ] Wire up CLI commands
- [ ] Add round-trip tests
- [ ] Test with TypeScript CLI exports

### Phase 2: Security & Organization Features

- [ ] Implement `EncryptedJsonFormatter`
- [ ] Implement `EncryptedJsonParser`
- [ ] Add SDK integration for encryption/decryption
- [ ] Add organization export/import support
- [ ] Add security warnings for unencrypted exports
- [ ] Add file permission checks
- [ ] Add password strength validation
- [ ] Add encrypted round-trip tests

### Phase 3: Additional Format Support

- [ ] Implement `FormatDetector`
- [ ] Implement `LastPassParser`
- [ ] Implement `OnePasswordParser`
- [ ] Implement `ChromeParser`
- [ ] Add format list command
- [ ] Add format-specific tests
- [ ] Test with real exports from each service

### Phase 4: Polish & Performance

- [ ] Optimize streaming for large vaults
- [ ] Implement batch import with retry
- [ ] Add memory usage monitoring
- [ ] Enhance error messages
- [ ] Add comprehensive integration tests
- [ ] Add performance benchmarks
- [ ] Create user documentation
- [ ] Add API documentation

### Final Verification

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-features --all-targets` (no warnings)
- [ ] `cargo build --release` (success)
- [ ] `cargo test` (all pass)
- [ ] Export 1000 items < 10s
- [ ] Import 1000 items < 30s
- [ ] Round-trip test passes
- [ ] Cross-compatibility with TypeScript CLI verified
- [ ] Documentation complete

## 15. Open Questions & Recommendations

### Resolved by Architecture

**Q5: Streaming vs In-Memory Processing**
- **Decision:** Streaming for export, in-memory for import
- **Rationale:** Export can stream per-item, import needs full dataset for validation

**Q6: Import Format Parser Architecture**
- **Decision:** Strategy pattern with trait-based parsers
- **Rationale:** Easy extensibility, clean separation, testable

**Q7: Progress Reporting Mechanism**
- **Decision:** Progress bar with percentage for > 100 items
- **Rationale:** User feedback for long operations, respects quiet/no-interaction flags

**Q8: API Rate Limiting During Import**
- **Decision:** Sequential with batches, add parallelism if needed
- **Rationale:** Start simple, optimize if performance issues arise

### Awaiting Product Decision

**Q1: Import Format Priority**
- **Recommendation:** MVP: Bitwarden + top 3 (LastPass, 1Password, Chrome)
- **Rationale:** Covers 80% of migration use cases, manageable scope

**Q2: Import Error Handling Strategy**
- **Recommendation:** Fail-fast for MVP
- **Rationale:** Data integrity critical, user can fix and retry

**Q3: Duplicate Item Handling**
- **Recommendation:** Allow duplicates for MVP
- **Rationale:** Simpler implementation, user can clean up manually

**Q4: Export Includes Attachments**
- **Recommendation:** No attachments for MVP
- **Rationale:** Complex feature, defer to future enhancement

## 16. Risk Mitigation

### High-Priority Risks

**R-1: Data Loss During Import**
- **Mitigation:** Fail-fast validation, comprehensive testing, clear error messages
- **Status:** Addressed by validation pipeline design

**R-2: Format Incompatibility**
- **Mitigation:** Extensive cross-compatibility testing, byte-for-byte comparison
- **Status:** Test strategy defined, requires implementation

**R-3: Security Vulnerability in Export**
- **Mitigation:** Clear warnings, encryption encouragement, secure defaults
- **Status:** Addressed by security warning design

### Medium-Priority Risks

**R-4: Performance Issues with Large Vaults**
- **Mitigation:** Streaming processing, batching, benchmarks
- **Status:** Addressed by streaming/batch architecture

**R-5: Import Format Parsing Complexity**
- **Mitigation:** Prioritize top formats, extensible architecture
- **Status:** Addressed by phased implementation plan

**R-6: API Rate Limiting**
- **Mitigation:** Batch requests, backoff logic, progress indication
- **Status:** Addressed by batch import design

## 17. Success Criteria

### Functional Requirements Met

- ✅ Export vault to CSV, JSON, encrypted JSON
- ✅ Import from Bitwarden formats
- ✅ Import from LastPass, 1Password, Chrome
- ✅ Format auto-detection for Bitwarden exports
- ✅ Organization export/import
- ✅ Progress indication
- ✅ Data validation before import
- ✅ Security warnings

### Technical Requirements Met

- ✅ Export 1,000 items < 10 seconds
- ✅ Import 1,000 items < 30 seconds
- ✅ Memory efficient (streaming)
- ✅ Round-trip data integrity
- ✅ TypeScript CLI compatibility
- ✅ Comprehensive error handling

### Quality Requirements Met

- ✅ Unit test coverage > 80%
- ✅ Integration tests for all formats
- ✅ Performance benchmarks documented
- ✅ Zero clippy warnings
- ✅ Complete documentation

## 18. Next Steps for Implementer

1. **Start with Phase 1 (MVP Core)**
   - Create module structure
   - Implement export service with CSV/JSON formatters
   - Implement import service with Bitwarden parsers
   - Add basic tests

2. **Validate Early**
   - Test CSV export matches TypeScript CLI exactly
   - Test JSON export structure
   - Test round-trip (export then import)

3. **Add Security (Phase 2)**
   - Implement encrypted JSON formatter/parser
   - Add SDK integration
   - Add warnings and confirmations

4. **Expand Formats (Phase 3)**
   - Add LastPass, 1Password, Chrome parsers
   - Test with real exports from each service

5. **Polish (Phase 4)**
   - Optimize performance
   - Enhance error messages
   - Complete testing and documentation

---

**Status:** READY_FOR_IMPLEMENTATION

This implementation plan provides a complete architecture for the import/export feature. All critical architectural decisions have been made, the component structure is defined, and the implementation path is clear. The implementer can proceed with confidence following this plan.
