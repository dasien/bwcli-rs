# API Reference: Import/Export Services

## Table of Contents

1. [Overview](#overview)
2. [Export Service](#export-service)
3. [Import Service](#import-service)
4. [Data Structures](#data-structures)
5. [Error Types](#error-types)
6. [Traits](#traits)
7. [Usage Examples](#usage-examples)

## Overview

The import/export services provide programmatic access to vault data export and import functionality. This API reference documents the service layer implementation for integration with CLI commands and other consumers.

**Module Path**: `bw_core::services::import_export`

**Dependencies**:
- `csv` - CSV parsing and generation
- `serde_json` - JSON serialization
- `async_trait` - Async trait support
- `secrecy` - Password handling
- `thiserror` - Error types

## Export Service

### ExportService

The main service for exporting vault data in various formats.

**Path**: `bw_core::services::import_export::ExportService`

#### Constructor

```rust
pub fn new() -> Self
```

Creates a new ExportService instance with all registered formatters.

**Registered Formatters**:
- `csv` - CSV formatter
- `json` - JSON formatter
- `encrypted_json` - Encrypted JSON formatter (placeholder)

**Example**:
```rust
use bw_core::services::import_export::ExportService;

let export_service = ExportService::new();
```

#### Methods

##### export

```rust
pub async fn export(
    &self,
    format: &str,
    output_path: Option<&str>,
    data: ExportData,
    options: ExportOptions,
) -> Result<ExportResult, ExportError>
```

Exports vault data in the specified format.

**Parameters**:
- `format` (`&str`) - Format identifier:
  - `"csv"` - Export as CSV
  - `"json"` - Export as JSON
  - `"encrypted_json"` - Export as encrypted JSON (requires SDK)
- `output_path` (`Option<&str>`) - Optional file path:
  - `Some("/path/to/file.csv")` - Write to file
  - `None` - Write to stdout
- `data` (`ExportData`) - Vault data to export (folders and ciphers)
- `options` (`ExportOptions`) - Export options (e.g., password for encryption)

**Returns**:
- `Ok(ExportResult)` - Export successful
- `Err(ExportError)` - Export failed

**Errors**:
- `ExportError::UnsupportedFormat` - Unknown format identifier
- `ExportError::PasswordRequired` - Encrypted format without password
- `ExportError::FileWriteError` - Cannot write to file
- `ExportError::CsvError` - CSV generation error
- `ExportError::JsonError` - JSON serialization error
- `ExportError::IoError` - I/O error

**Example**:
```rust
use bw_core::services::import_export::{ExportService, ExportData, ExportOptions};

let export_service = ExportService::new();
let data = ExportData {
    folders: vec![folder1, folder2],
    ciphers: vec![cipher1, cipher2, cipher3],
};
let options = ExportOptions::default();

// Export to CSV file
let result = export_service
    .export("csv", Some("backup.csv"), data, options)
    .await?;
println!("Exported {} items", result.item_count);
```

##### list_formats

```rust
pub fn list_formats(&self) -> Vec<FormatInfo>
```

Lists all supported export formats.

**Returns**:
- `Vec<FormatInfo>` - List of format metadata

**Example**:
```rust
let formats = export_service.list_formats();
for format in formats {
    println!("{}: {}", format.id, format.name);
}
// Output:
// csv: CSV
// json: JSON
// encrypted_json: Encrypted JSON
```

## Import Service

### ImportService

The main service for importing vault data from various formats.

**Path**: `bw_core::services::import_export::ImportService`

#### Constructor

```rust
pub fn new() -> Self
```

Creates a new ImportService instance with all registered parsers.

**Registered Parsers**:
- `bitwardencsv` - Bitwarden CSV parser
- `bitwardenjson` - Bitwarden JSON parser
- `lastpass` - LastPass CSV parser
- `1password` - 1Password CSV parser
- `chrome` - Chrome passwords CSV parser

**Example**:
```rust
use bw_core::services::import_export::ImportService;

let import_service = ImportService::new();
```

#### Methods

##### import

```rust
pub async fn import(
    &self,
    format: &str,
    file_path: &str,
    options: ImportOptions,
) -> Result<ImportResult, ImportError>
```

Imports vault data from a file in the specified format.

**Parameters**:
- `format` (`&str`) - Format identifier:
  - `"bitwardencsv"` - Bitwarden CSV
  - `"bitwardenjson"` - Bitwarden JSON
  - `"lastpass"` - LastPass CSV
  - `"1password"` - 1Password CSV
  - `"chrome"` - Chrome passwords CSV
- `file_path` (`&str`) - Path to import file
- `options` (`ImportOptions`) - Import options

**Returns**:
- `Ok(ImportResult)` - Import successful
- `Err(ImportError)` - Import failed

**Errors**:
- `ImportError::UnsupportedFormat` - Unknown format identifier
- `ImportError::FileReadError` - Cannot read file
- `ImportError::FileTooLarge` - File exceeds 100MB limit
- `ImportError::ParseError` - File format invalid
- `ImportError::ValidationError` - Data validation failed
- `ImportError::CsvError` - CSV parsing error
- `ImportError::JsonError` - JSON parsing error
- `ImportError::IoError` - I/O error

**Example**:
```rust
use bw_core::services::import_export::{ImportService, ImportOptions};

let import_service = ImportService::new();
let options = ImportOptions::default();

// Import Bitwarden CSV
let result = import_service
    .import("bitwardencsv", "backup.csv", options)
    .await?;
println!("Imported {} items and {} folders",
    result.items_created, result.folders_created);
```

##### list_formats

```rust
pub fn list_formats(&self) -> Vec<FormatInfo>
```

Lists all supported import formats.

**Returns**:
- `Vec<FormatInfo>` - List of format metadata

**Example**:
```rust
let formats = import_service.list_formats();
for format in formats {
    println!("{}: {}", format.id, format.name);
}
// Output:
// bitwardencsv: Bitwarden CSV
// bitwardenjson: Bitwarden JSON
// lastpass: LastPass
// 1password: 1Password
// chrome: Chrome Passwords
```

## Data Structures

### ExportData

Vault data to be exported.

**Path**: `bw_core::services::import_export::ExportData`

**Fields**:
```rust
pub struct ExportData {
    pub folders: Vec<FolderView>,
    pub ciphers: Vec<CipherView>,
}
```

- `folders` (`Vec<FolderView>`) - List of folders to export
- `ciphers` (`Vec<CipherView>`) - List of decrypted ciphers to export

**Example**:
```rust
use bw_core::services::import_export::ExportData;

let data = ExportData {
    folders: vec![
        FolderView {
            id: Some("folder1".to_string()),
            name: "Work".to_string(),
        },
    ],
    ciphers: vec![
        cipher1,
        cipher2,
    ],
};
```

### ExportOptions

Options for export operations.

**Path**: `bw_core::services::import_export::ExportOptions`

**Fields**:
```rust
pub struct ExportOptions {
    pub password: Option<Secret<String>>,
}
```

- `password` (`Option<Secret<String>>`) - Password for encrypted exports

**Example**:
```rust
use bw_core::services::import_export::ExportOptions;
use secrecy::Secret;

// Default options (no password)
let options = ExportOptions::default();

// With password for encrypted export
let mut options = ExportOptions::default();
options.password = Some(Secret::new("mypassword".to_string()));
```

### ExportResult

Result of an export operation.

**Path**: `bw_core::services::import_export::ExportResult`

**Fields**:
```rust
pub struct ExportResult {
    pub format: String,
    pub item_count: usize,
    pub output: Option<String>,
}
```

- `format` (`String`) - Format that was exported
- `item_count` (`usize`) - Number of items exported
- `output` (`Option<String>`) - File path or None for stdout

**Example**:
```rust
let result = export_service.export("csv", Some("backup.csv"), data, options).await?;
println!("Format: {}", result.format);        // "csv"
println!("Items: {}", result.item_count);      // 247
println!("File: {:?}", result.output);         // Some("backup.csv")
```

### ImportData

Parsed import data before vault creation.

**Path**: `bw_core::services::import_export::ImportData`

**Fields**:
```rust
pub struct ImportData {
    pub folders: Vec<ImportFolder>,
    pub items: Vec<ImportItem>,
}
```

- `folders` (`Vec<ImportFolder>`) - Folders parsed from import file
- `items` (`Vec<ImportItem>`) - Items parsed from import file

**Note**: This is an internal structure. Users work with `ImportResult`.

### ImportFolder

A folder parsed from import data.

**Path**: `bw_core::services::import_export::ImportFolder`

**Fields**:
```rust
pub struct ImportFolder {
    pub name: String,
}
```

- `name` (`String`) - Folder name

### ImportItem

An item parsed from import data.

**Path**: `bw_core::services::import_export::ImportItem`

**Fields**:
```rust
pub struct ImportItem {
    pub item_type: CipherType,
    pub name: String,
    pub folder: Option<String>,
    pub favorite: bool,
    pub notes: Option<String>,
    pub fields: Vec<Field>,
    pub login: Option<Login>,
    pub card: Option<Card>,
    pub identity: Option<Identity>,
}
```

- `item_type` (`CipherType`) - Type of cipher (Login, SecureNote, Card, Identity)
- `name` (`String`) - Item name (required)
- `folder` (`Option<String>`) - Folder name (optional)
- `favorite` (`bool`) - Favorite flag
- `notes` (`Option<String>`) - Notes text
- `fields` (`Vec<Field>`) - Custom fields
- `login` (`Option<Login>`) - Login data (if type is Login)
- `card` (`Option<Card>`) - Card data (if type is Card)
- `identity` (`Option<Identity>`) - Identity data (if type is Identity)

### ImportOptions

Options for import operations.

**Path**: `bw_core::services::import_export::ImportOptions`

**Fields**:
```rust
pub struct ImportOptions {
    // Currently no options defined
    // Future: skip_validation, dry_run, etc.
}
```

**Example**:
```rust
use bw_core::services::import_export::ImportOptions;

let options = ImportOptions::default();
```

### ImportResult

Result of an import operation.

**Path**: `bw_core::services::import_export::ImportResult`

**Fields**:
```rust
pub struct ImportResult {
    pub format: String,
    pub items_created: usize,
    pub folders_created: usize,
}
```

- `format` (`String`) - Format that was imported
- `items_created` (`usize`) - Number of items created
- `folders_created` (`usize`) - Number of folders created

**Example**:
```rust
let result = import_service.import("lastpass", "export.csv", options).await?;
println!("Format: {}", result.format);              // "lastpass"
println!("Items: {}", result.items_created);        // 183
println!("Folders: {}", result.folders_created);    // 8
```

### FormatInfo

Metadata about a supported format.

**Path**: `bw_core::services::import_export::FormatInfo`

**Fields**:
```rust
pub struct FormatInfo {
    pub id: String,
    pub name: String,
}
```

- `id` (`String`) - Format identifier (e.g., "csv", "lastpass")
- `name` (`String`) - Human-readable name (e.g., "CSV", "LastPass")

**Example**:
```rust
let formats = export_service.list_formats();
for format in formats {
    println!("ID: {}, Name: {}", format.id, format.name);
}
```

## Error Types

### ExportError

Errors that can occur during export operations.

**Path**: `bw_core::services::import_export::ExportError`

**Variants**:

```rust
pub enum ExportError {
    NotAuthenticated,
    UnsupportedFormat(String),
    PasswordRequired,
    FileWriteError(String),
    DecryptionError,
    IoError(std::io::Error),
    CsvError(csv::Error),
    JsonError(serde_json::Error),
    OperationCancelled,
}
```

#### Variant Details

##### NotAuthenticated

User is not authenticated.

**When**: No valid session key

**Solution**: Login or unlock vault first

##### UnsupportedFormat(String)

Unknown or unsupported format identifier.

**When**: Invalid format passed to `export()`

**Example**:
```rust
// This will return UnsupportedFormat("xml")
export_service.export("xml", Some("out.xml"), data, options).await?;
```

**Solution**: Use a supported format (csv, json, encrypted_json)

##### PasswordRequired

Password required for encrypted export but not provided.

**When**: `format = "encrypted_json"` and `options.password = None`

**Solution**: Provide password in options:
```rust
let mut options = ExportOptions::default();
options.password = Some(Secret::new("password".to_string()));
```

##### FileWriteError(String)

Cannot write to output file.

**When**: Permissions, disk full, invalid path

**Solution**: Check file path and permissions

##### DecryptionError

Failed to decrypt vault data for export.

**When**: Vault data corrupted or key invalid

**Solution**: Verify session key is valid

##### IoError(std::io::Error)

I/O operation failed.

**When**: File system errors

**Solution**: Check disk space, permissions

##### CsvError(csv::Error)

CSV generation failed.

**When**: Invalid data structure

**Solution**: Verify cipher data is valid

##### JsonError(serde_json::Error)

JSON serialization failed.

**When**: Invalid data structure

**Solution**: Verify cipher data is valid

##### OperationCancelled

User cancelled the operation.

**When**: User interrupts export (future)

**Solution**: Restart export

#### Error Display

All errors implement `Display` with user-friendly messages:

```rust
match export_service.export("xml", None, data, options).await {
    Err(ExportError::UnsupportedFormat(format)) => {
        eprintln!("Unsupported format: {}", format);
    }
    Err(ExportError::PasswordRequired) => {
        eprintln!("Password required for encrypted export");
    }
    Err(e) => {
        eprintln!("Export failed: {}", e);
    }
    Ok(result) => {
        println!("Export successful!");
    }
}
```

### ImportError

Errors that can occur during import operations.

**Path**: `bw_core::services::import_export::ImportError`

**Variants**:

```rust
pub enum ImportError {
    NotAuthenticated,
    UnsupportedFormat(String),
    FileReadError(String),
    ParseError { line: Option<usize>, message: String },
    ValidationError { error_count: usize, details: String },
    PasswordRequired,
    ImportFailed(String),
    FileTooLarge { size: usize, max_size: usize },
    IoError(std::io::Error),
    CsvError(csv::Error),
    JsonError(serde_json::Error),
}
```

#### Variant Details

##### NotAuthenticated

User is not authenticated.

**When**: No valid session key

**Solution**: Login or unlock vault first

##### UnsupportedFormat(String)

Unknown or unsupported format identifier.

**When**: Invalid format passed to `import()`

**Example**:
```rust
// This will return UnsupportedFormat("keepass")
import_service.import("keepass", "file.csv", options).await?;
```

**Solution**: Use a supported format or implement new parser

##### FileReadError(String)

Cannot read import file.

**When**: File doesn't exist, no permissions

**Solution**: Verify file path and permissions

##### ParseError

Failed to parse import file.

**Fields**:
- `line` (`Option<usize>`) - Line number where error occurred
- `message` (`String`) - Error description

**When**: Invalid CSV/JSON structure

**Example**:
```rust
ImportError::ParseError {
    line: Some(5),
    message: "Expected 7 columns, found 5".to_string(),
}
```

**Solution**: Fix file format or use correct parser

##### ValidationError

Data validation failed.

**Fields**:
- `error_count` (`usize`) - Number of validation errors
- `details` (`String`) - Formatted error details

**When**: Data violates business rules

**Example**:
```rust
ImportError::ValidationError {
    error_count: 2,
    details: "Line 5: login must have username or password\nLine 12: name is required".to_string(),
}
```

**Solution**: Fix data in import file

##### PasswordRequired

Password required for encrypted import but not provided.

**When**: Encrypted JSON import without password

**Solution**: Provide password or decrypt file first

##### ImportFailed(String)

Import operation failed.

**When**: Vault write error, network error

**Solution**: Check error message for details

##### FileTooLarge

Import file exceeds size limit.

**Fields**:
- `size` (`usize`) - File size in bytes
- `max_size` (`usize`) - Maximum allowed size (100MB)

**When**: File over 100MB

**Example**:
```rust
ImportError::FileTooLarge {
    size: 105906176,
    max_size: 104857600,
}
```

**Solution**: Split file or remove unnecessary data

##### IoError(std::io::Error)

I/O operation failed.

**When**: File system errors

**Solution**: Check file path and permissions

##### CsvError(csv::Error)

CSV parsing failed.

**When**: Invalid CSV structure

**Solution**: Verify CSV format

##### JsonError(serde_json::Error)

JSON parsing failed.

**When**: Invalid JSON structure

**Example**:
```rust
ImportError::JsonError(Error("missing field `revisionDate`", line: 4, column: 38))
```

**Solution**: Verify JSON structure

#### Error Display

All errors implement `Display` with user-friendly messages:

```rust
match import_service.import("keepass", "file.csv", options).await {
    Err(ImportError::UnsupportedFormat(format)) => {
        eprintln!("Unsupported format: {}", format);
    }
    Err(ImportError::FileTooLarge { size, max_size }) => {
        eprintln!("File too large: {} bytes (max: {} bytes)", size, max_size);
    }
    Err(ImportError::ValidationError { error_count, details }) => {
        eprintln!("❌ Validation failed with {} error(s):\n{}", error_count, details);
    }
    Err(e) => {
        eprintln!("Import failed: {}", e);
    }
    Ok(result) => {
        println!("Import successful!");
    }
}
```

### ValidationError

Individual validation error details.

**Path**: `bw_core::services::import_export::ValidationError`

**Fields**:
```rust
pub struct ValidationError {
    pub line: Option<usize>,
    pub field: Option<String>,
    pub message: String,
}
```

- `line` (`Option<usize>`) - Line number in import file (1-indexed)
- `field` (`Option<String>`) - Field name that failed validation
- `message` (`String`) - Error message

**Example**:
```rust
ValidationError {
    line: Some(5),
    field: Some("login_username".to_string()),
    message: "Login must have username or password".to_string(),
}
```

**Display Format**:
```
Line 5: login_username: Login must have username or password
```

## Traits

### ExportFormatter

Trait for implementing export formatters.

**Path**: `bw_core::services::import_export::export::ExportFormatter`

**Definition**:
```rust
#[async_trait]
pub trait ExportFormatter: Send + Sync {
    fn format_id(&self) -> &str;
    fn format_name(&self) -> &str;
    async fn format(
        &self,
        data: &ExportData,
        options: &ExportOptions,
    ) -> Result<String, ExportError>;
}
```

#### Methods

##### format_id

```rust
fn format_id(&self) -> &str
```

Returns the unique identifier for this format.

**Returns**: Format ID (e.g., "csv", "json")

**Example**:
```rust
impl ExportFormatter for CsvFormatter {
    fn format_id(&self) -> &str {
        "csv"
    }
}
```

##### format_name

```rust
fn format_name(&self) -> &str
```

Returns the human-readable name for this format.

**Returns**: Format name (e.g., "CSV", "JSON")

**Example**:
```rust
impl ExportFormatter for CsvFormatter {
    fn format_name(&self) -> &str {
        "CSV"
    }
}
```

##### format

```rust
async fn format(
    &self,
    data: &ExportData,
    options: &ExportOptions,
) -> Result<String, ExportError>
```

Formats the export data as a string.

**Parameters**:
- `data` (`&ExportData`) - Vault data to format
- `options` (`&ExportOptions`) - Export options

**Returns**:
- `Ok(String)` - Formatted output
- `Err(ExportError)` - Formatting failed

**Example**:
```rust
impl ExportFormatter for CsvFormatter {
    async fn format(
        &self,
        data: &ExportData,
        options: &ExportOptions,
    ) -> Result<String, ExportError> {
        let mut writer = csv::Writer::from_writer(vec![]);

        // Write headers
        writer.write_record(&[
            "folder", "favorite", "type", "name", /* ... */
        ])?;

        // Write data rows
        for cipher in &data.ciphers {
            // ...
        }

        let output = String::from_utf8(writer.into_inner()?)?;
        Ok(output)
    }
}
```

#### Implementing Custom Formatters

To add a new export format:

1. **Create formatter struct**:
```rust
pub struct XmlFormatter;
```

2. **Implement ExportFormatter trait**:
```rust
#[async_trait]
impl ExportFormatter for XmlFormatter {
    fn format_id(&self) -> &str {
        "xml"
    }

    fn format_name(&self) -> &str {
        "XML"
    }

    async fn format(
        &self,
        data: &ExportData,
        options: &ExportOptions,
    ) -> Result<String, ExportError> {
        // Convert data to XML format
        let xml = format_as_xml(data)?;
        Ok(xml)
    }
}
```

3. **Register in ExportService**:
```rust
impl ExportService {
    pub fn new() -> Self {
        let mut formatters: HashMap<String, Box<dyn ExportFormatter>> = HashMap::new();
        formatters.insert("csv".to_string(), Box::new(CsvFormatter));
        formatters.insert("json".to_string(), Box::new(JsonFormatter));
        formatters.insert("xml".to_string(), Box::new(XmlFormatter));
        Self { formatters }
    }
}
```

### ImportParser

Trait for implementing import parsers.

**Path**: `bw_core::services::import_export::import::ImportParser`

**Definition**:
```rust
#[async_trait]
pub trait ImportParser: Send + Sync {
    fn format_id(&self) -> &str;
    fn format_name(&self) -> &str;
    async fn parse(
        &self,
        content: &str,
        options: &ImportOptions,
    ) -> Result<ImportData, ImportError>;
}
```

#### Methods

##### format_id

```rust
fn format_id(&self) -> &str
```

Returns the unique identifier for this format.

**Returns**: Format ID (e.g., "lastpass", "1password")

**Example**:
```rust
impl ImportParser for LastPassParser {
    fn format_id(&self) -> &str {
        "lastpass"
    }
}
```

##### format_name

```rust
fn format_name(&self) -> &str
```

Returns the human-readable name for this format.

**Returns**: Format name (e.g., "LastPass", "1Password")

**Example**:
```rust
impl ImportParser for LastPassParser {
    fn format_name(&self) -> &str {
        "LastPass"
    }
}
```

##### parse

```rust
async fn parse(
    &self,
    content: &str,
    options: &ImportOptions,
) -> Result<ImportData, ImportError>
```

Parses import file content into structured data.

**Parameters**:
- `content` (`&str`) - File content to parse
- `options` (`&ImportOptions`) - Import options

**Returns**:
- `Ok(ImportData)` - Parsed data
- `Err(ImportError)` - Parsing failed

**Example**:
```rust
impl ImportParser for LastPassParser {
    async fn parse(
        &self,
        content: &str,
        options: &ImportOptions,
    ) -> Result<ImportData, ImportError> {
        let mut reader = csv::Reader::from_reader(content.as_bytes());
        let mut items = Vec::new();
        let mut folders = HashSet::new();

        for (line_num, result) in reader.records().enumerate() {
            let record = result.map_err(|e| ImportError::ParseError {
                line: Some(line_num + 2),
                message: e.to_string(),
            })?;

            // Parse record into ImportItem
            let item = parse_lastpass_record(&record)?;
            items.push(item);

            // Extract folder name
            if let Some(folder) = &item.folder {
                folders.insert(folder.clone());
            }
        }

        Ok(ImportData {
            folders: folders.into_iter()
                .map(|name| ImportFolder { name })
                .collect(),
            items,
        })
    }
}
```

#### Implementing Custom Parsers

To add a new import format:

1. **Create parser struct**:
```rust
pub struct KeePassParser;
```

2. **Implement ImportParser trait**:
```rust
#[async_trait]
impl ImportParser for KeePassParser {
    fn format_id(&self) -> &str {
        "keepass"
    }

    fn format_name(&self) -> &str {
        "KeePass"
    }

    async fn parse(
        &self,
        content: &str,
        options: &ImportOptions,
    ) -> Result<ImportData, ImportError> {
        // Parse KeePass CSV format
        let data = parse_keepass_format(content)?;
        Ok(data)
    }
}
```

3. **Register in ImportService**:
```rust
impl ImportService {
    pub fn new() -> Self {
        let mut parsers: HashMap<String, Box<dyn ImportParser>> = HashMap::new();
        parsers.insert("bitwardencsv".to_string(), Box::new(BitwardenCsvParser));
        parsers.insert("keepass".to_string(), Box::new(KeePassParser));
        Self { parsers, validator: Validator::new() }
    }
}
```

## Usage Examples

### Complete Export Workflow

```rust
use bw_core::services::import_export::{ExportService, ExportData, ExportOptions};
use secrecy::Secret;

async fn export_vault_example() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize service
    let export_service = ExportService::new();

    // Prepare data (from vault service)
    let data = ExportData {
        folders: vec![
            FolderView {
                id: Some("folder1".to_string()),
                name: "Work".to_string(),
            },
            FolderView {
                id: Some("folder2".to_string()),
                name: "Personal".to_string(),
            },
        ],
        ciphers: vec![
            cipher1,  // Login item
            cipher2,  // Secure note
            cipher3,  // Card
        ],
    };

    // Export to CSV file
    let options = ExportOptions::default();
    let result = export_service
        .export("csv", Some("vault-backup.csv"), data.clone(), options)
        .await?;

    println!("✓ Exported {} items to {}",
        result.item_count,
        result.output.unwrap()
    );

    // Export to JSON stdout
    let options = ExportOptions::default();
    let result = export_service
        .export("json", None, data.clone(), options)
        .await?;

    println!("✓ Exported {} items to stdout", result.item_count);

    // Export to encrypted JSON (when SDK available)
    let mut options = ExportOptions::default();
    options.password = Some(Secret::new("mySecurePassword".to_string()));

    match export_service.export("encrypted_json", Some("secure.json"), data, options).await {
        Ok(result) => {
            println!("✓ Exported {} items (encrypted)", result.item_count);
        }
        Err(e) => {
            eprintln!("✗ Encrypted export failed: {}", e);
        }
    }

    Ok(())
}
```

### Complete Import Workflow

```rust
use bw_core::services::import_export::{ImportService, ImportOptions};

async fn import_vault_example() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize service
    let import_service = ImportService::new();

    // List available formats
    println!("Supported formats:");
    for format in import_service.list_formats() {
        println!("  {}: {}", format.id, format.name);
    }

    // Import Bitwarden CSV
    let options = ImportOptions::default();
    match import_service.import("bitwardencsv", "backup.csv", options).await {
        Ok(result) => {
            println!("✓ Imported {} items and {} folders from {}",
                result.items_created,
                result.folders_created,
                result.format
            );
        }
        Err(e) => {
            eprintln!("✗ Import failed: {}", e);
            return Err(e.into());
        }
    }

    // Import LastPass
    let options = ImportOptions::default();
    match import_service.import("lastpass", "lastpass_export.csv", options).await {
        Ok(result) => {
            println!("✓ Migrated {} items from LastPass", result.items_created);
        }
        Err(e) => {
            eprintln!("✗ LastPass import failed: {}", e);
        }
    }

    Ok(())
}
```

### Error Handling

```rust
use bw_core::services::import_export::{ImportService, ImportError};

async fn import_with_error_handling() -> Result<(), Box<dyn std::error::Error>> {
    let import_service = ImportService::new();
    let options = ImportOptions::default();

    match import_service.import("bitwardencsv", "import.csv", options).await {
        Ok(result) => {
            println!("✓ Import successful!");
            println!("  Items created: {}", result.items_created);
            println!("  Folders created: {}", result.folders_created);
        }
        Err(ImportError::UnsupportedFormat(format)) => {
            eprintln!("✗ Format '{}' is not supported", format);
            eprintln!("  Use 'import --formats' to list available formats");
        }
        Err(ImportError::FileTooLarge { size, max_size }) => {
            eprintln!("✗ File is too large: {} bytes", size);
            eprintln!("  Maximum allowed: {} bytes ({}MB)",
                max_size,
                max_size / 1024 / 1024
            );
            eprintln!("  Try splitting the file into smaller parts");
        }
        Err(ImportError::ValidationError { error_count, details }) => {
            eprintln!("✗ Validation failed with {} error(s):", error_count);
            eprintln!("{}", details);
            eprintln!("\nNo items were imported. Please fix the errors and try again.");
        }
        Err(ImportError::ParseError { line, message }) => {
            if let Some(line_num) = line {
                eprintln!("✗ Parse error at line {}: {}", line_num, message);
            } else {
                eprintln!("✗ Parse error: {}", message);
            }
        }
        Err(e) => {
            eprintln!("✗ Import failed: {}", e);
        }
    }

    Ok(())
}
```

### Custom Formatter Example

```rust
use bw_core::services::import_export::export::{ExportFormatter, ExportData, ExportOptions, ExportError};
use async_trait::async_trait;

pub struct XmlFormatter;

#[async_trait]
impl ExportFormatter for XmlFormatter {
    fn format_id(&self) -> &str {
        "xml"
    }

    fn format_name(&self) -> &str {
        "XML"
    }

    async fn format(
        &self,
        data: &ExportData,
        options: &ExportOptions,
    ) -> Result<String, ExportError> {
        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        xml.push_str("<vault>\n");

        // Export folders
        xml.push_str("  <folders>\n");
        for folder in &data.folders {
            xml.push_str(&format!(
                "    <folder id=\"{}\" name=\"{}\" />\n",
                folder.id.as_ref().unwrap_or(&"".to_string()),
                escape_xml(&folder.name)
            ));
        }
        xml.push_str("  </folders>\n");

        // Export ciphers
        xml.push_str("  <ciphers>\n");
        for cipher in &data.ciphers {
            xml.push_str(&format!(
                "    <cipher type=\"{:?}\" name=\"{}\">\n",
                cipher.r#type,
                escape_xml(&cipher.name)
            ));

            // Add type-specific data
            match cipher.r#type {
                CipherType::Login => {
                    if let Some(login) = &cipher.login {
                        xml.push_str(&format!(
                            "      <login username=\"{}\" />\n",
                            escape_xml(&login.username.unwrap_or_default())
                        ));
                    }
                }
                _ => {}
            }

            xml.push_str("    </cipher>\n");
        }
        xml.push_str("  </ciphers>\n");

        xml.push_str("</vault>\n");

        Ok(xml)
    }
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
     .replace('<', "&lt;")
     .replace('>', "&gt;")
     .replace('"', "&quot;")
     .replace('\'', "&apos;")
}
```

### Custom Parser Example

```rust
use bw_core::services::import_export::import::{ImportParser, ImportData, ImportOptions, ImportError};
use async_trait::async_trait;

pub struct CustomParser;

#[async_trait]
impl ImportParser for CustomParser {
    fn format_id(&self) -> &str {
        "custom"
    }

    fn format_name(&self) -> &str {
        "Custom Format"
    }

    async fn parse(
        &self,
        content: &str,
        options: &ImportOptions,
    ) -> Result<ImportData, ImportError> {
        let mut items = Vec::new();
        let mut folders = HashSet::new();

        // Parse custom format
        for (line_num, line) in content.lines().enumerate() {
            if line.is_empty() {
                continue;
            }

            // Parse line (example: "folder|name|username|password")
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() < 4 {
                return Err(ImportError::ParseError {
                    line: Some(line_num + 1),
                    message: format!("Expected 4 fields, found {}", parts.len()),
                });
            }

            let folder = if !parts[0].is_empty() {
                folders.insert(parts[0].to_string());
                Some(parts[0].to_string())
            } else {
                None
            };

            items.push(ImportItem {
                item_type: CipherType::Login,
                name: parts[1].to_string(),
                folder,
                favorite: false,
                notes: None,
                fields: vec![],
                login: Some(Login {
                    username: Some(parts[2].to_string()),
                    password: Some(parts[3].to_string()),
                    uris: vec![],
                    totp: None,
                }),
                card: None,
                identity: None,
            });
        }

        Ok(ImportData {
            folders: folders.into_iter()
                .map(|name| ImportFolder { name })
                .collect(),
            items,
        })
    }
}
```

## Integration with CLI

Example CLI command handler:

```rust
use bw_core::services::import_export::{ExportService, ImportService};
use clap::{Args, Subcommand};

#[derive(Subcommand)]
pub enum ToolsCommands {
    Export(ExportArgs),
    Import(ImportArgs),
}

#[derive(Args)]
pub struct ExportArgs {
    /// Export format (csv, json, encrypted_json)
    #[arg(long, default_value = "csv")]
    format: String,

    /// Output file path (stdout if not specified)
    #[arg(long)]
    output: Option<String>,

    /// Password for encrypted exports
    #[arg(long)]
    password: Option<String>,
}

#[derive(Args)]
pub struct ImportArgs {
    /// Import format
    format: String,

    /// Import file path
    file: String,

    /// List available formats
    #[arg(long)]
    formats: bool,
}

pub async fn execute_export(args: ExportArgs) -> Result<(), Box<dyn std::error::Error>> {
    let export_service = ExportService::new();

    // Get vault data (from vault service)
    let data = get_vault_data().await?;

    // Prepare options
    let mut options = ExportOptions::default();
    if let Some(password) = args.password {
        options.password = Some(Secret::new(password));
    }

    // Execute export
    let result = export_service
        .export(&args.format, args.output.as_deref(), data, options)
        .await?;

    // Display result
    if let Some(output_path) = result.output {
        println!("✓ Exported {} items to {}", result.item_count, output_path);
    } else {
        // Output was written to stdout
    }

    Ok(())
}

pub async fn execute_import(args: ImportArgs) -> Result<(), Box<dyn std::error::Error>> {
    let import_service = ImportService::new();

    // Handle --formats flag
    if args.formats {
        println!("Supported import formats:");
        for format in import_service.list_formats() {
            println!("  {:<15} - {}", format.id, format.name);
        }
        return Ok(());
    }

    // Execute import
    let options = ImportOptions::default();
    let result = import_service
        .import(&args.format, &args.file, options)
        .await?;

    // Display result
    println!("✓ Successfully imported {} items and {} folders",
        result.items_created,
        result.folders_created
    );

    Ok(())
}
```

## Performance Considerations

### Memory Usage

- **Export**: Loads all ciphers into memory
  - Acceptable for typical vaults (< 1000 items)
  - Consider streaming for very large vaults

- **Import**: Loads entire file into memory
  - Limited to 100MB to prevent DoS
  - Processes sequentially to minimize memory spikes

### Optimization Tips

1. **Large Exports**:
   ```rust
   // For very large vaults, consider pagination
   let batch_size = 1000;
   for batch in data.ciphers.chunks(batch_size) {
       // Export batch
   }
   ```

2. **Large Imports**:
   ```rust
   // File size check is automatic
   // Split files if over 100MB limit
   ```

3. **Format Selection**:
   - CSV: Fastest, smallest file size
   - JSON: More features, larger file size
   - Encrypted JSON: Slowest (encryption overhead)

## Testing

### Unit Test Example

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_export_csv() {
        let service = ExportService::new();
        let data = ExportData {
            folders: vec![],
            ciphers: vec![create_test_cipher()],
        };
        let options = ExportOptions::default();

        let result = service
            .export("csv", None, data, options)
            .await
            .unwrap();

        assert_eq!(result.item_count, 1);
        assert_eq!(result.format, "csv");
    }

    #[tokio::test]
    async fn test_import_bitwarden_csv() {
        let service = ImportService::new();
        let csv_content = "folder,favorite,type,name,notes,fields,reprompt,login_uri,login_username,login_password\nWork,0,login,test,,,,https://test.com,user,pass";

        // Write to temp file
        let temp_file = write_temp_file(csv_content);

        let options = ImportOptions::default();
        let result = service
            .import("bitwardencsv", &temp_file, options)
            .await
            .unwrap();

        assert_eq!(result.items_created, 1);
        assert_eq!(result.folders_created, 1);
    }
}
```

## See Also

- **User Guide**: `user_guide.md` - Detailed usage instructions
- **Documentation Summary**: `documentation_summary.md` - High-level overview
- **Test Summary**: `../tester/required_output/test_summary.md` - Test results
- **Implementation Summary**: `../implementer/required_output/implementation_summary.md` - Implementation details
