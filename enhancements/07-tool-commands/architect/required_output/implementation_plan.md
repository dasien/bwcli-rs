---
enhancement: 07-tool-commands
agent: plan
task_id: task_1764954654_22677
timestamp: 2025-12-05T00:00:00Z
status: READY_FOR_IMPLEMENTATION
---

# Implementation Plan: Tool Commands Enhancement

## Executive Summary

This plan details the implementation approach for four categories of tool commands in the Bitwarden CLI Rust migration:

1. **Password Generation** (`bw generate`) - Standalone cryptographically secure password/passphrase generation
2. **Send Operations** (`bw send`) - Temporary secure sharing with encryption
3. **Receive** (`bw receive`) - Public access to shared Send content
4. **Encode** (`bw encode`) - Base64 encoding utility

**Key Architectural Decisions:**
- Implement password generation directly with `rand` crate (SDK currently mocked)
- Create new Send domain models matching API structure
- Implement Send encryption using SDK when available, fallback to rust-crypto
- Add `receive` as top-level command (TypeScript CLI convention)
- Use streaming for file Send operations
- Embed EFF long wordlist for passphrase generation

## Architecture Overview

### Component Structure

```
crates/bw-core/src/
├── models/
│   └── send/                           [NEW]
│       ├── mod.rs                      - Public API exports
│       ├── send.rs                     - Core Send model
│       ├── send_text.rs                - Text Send type
│       ├── send_file.rs                - File Send type
│       ├── send_request.rs             - Create/Edit request models
│       └── send_access.rs              - Public access models
├── services/
│   ├── generator/                      [NEW]
│   │   ├── mod.rs                      - Public API exports
│   │   ├── password.rs                 - Password generation logic
│   │   ├── passphrase.rs               - Passphrase generation logic
│   │   ├── wordlist.rs                 - Embedded EFF wordlist
│   │   └── errors.rs                   - Generator errors
│   ├── send/                           [NEW]
│   │   ├── mod.rs                      - Public API exports
│   │   ├── encryption.rs               - Send encryption/decryption
│   │   ├── send_service.rs             - CRUD operations
│   │   └── errors.rs                   - Send-specific errors
│   └── api/
│       ├── send_api.rs                 [NEW] - Send API endpoints
│       └── client.rs                   [MODIFY] - Add Send endpoints

crates/bw-cli/src/commands/
├── tools.rs                            [MODIFY] - Implement generate/encode
├── send.rs                             [MODIFY] - Implement Send commands
└── receive.rs                          [NEW] - Receive command
```

### Data Flow Diagrams

#### Password/Passphrase Generation Flow
```
User Input → CLI Parser → Generator Service → OsRng → Output
                                ↓
                         Validation Logic
```

#### Send Creation Flow
```
User Input → CLI Parser → Send Service → Encryption → API Client → Server
                              ↓              ↓
                        Validation      SDK/Crypto
                              ↓
                        Storage (keys)
```

#### Receive Flow
```
Send URL → CLI Parser → API Client (public) → Decryption → Output
                            ↓                      ↓
                    Parse URL/Access Key    SDK/Crypto
```

## Data Models

### Send Domain Models

**Location:** `crates/bw-core/src/models/send/`

#### Send (Core Model)
```rust
/// Send object representing a temporary secure share
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Send {
    /// Send ID (UUID)
    pub id: String,

    /// Access ID used in public URL
    pub access_id: String,

    /// Send type: 0=Text, 1=File
    #[serde(rename = "type")]
    pub send_type: SendType,

    /// Encrypted name (EncString)
    pub name: String,

    /// Encrypted notes (EncString)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,

    /// Text Send data (if type=0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<SendText>,

    /// File Send data (if type=1)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<SendFile>,

    /// Encrypted encryption key (EncString)
    pub key: String,

    /// Maximum access count (null = unlimited)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_access_count: Option<u32>,

    /// Current access count
    pub access_count: u32,

    /// Expiration date (ISO 8601, null = no expiration)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiration_date: Option<String>,

    /// Deletion date (ISO 8601)
    pub deletion_date: String,

    /// Whether Send is disabled
    pub disabled: bool,

    /// Whether password is required for access
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,

    /// Revision date (ISO 8601)
    pub revision_date: String,

    /// Whether Send has been soft-deleted
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hide_email: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum SendType {
    Text = 0,
    File = 1,
}
```

#### SendText
```rust
/// Text Send data
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendText {
    /// Encrypted text content (EncString)
    pub text: String,

    /// Whether text should be hidden by default
    pub hidden: bool,
}
```

#### SendFile
```rust
/// File Send data
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendFile {
    /// Encrypted file name (EncString)
    pub file_name: String,

    /// File size in bytes
    pub size: u64,

    /// File size string (human-readable)
    pub size_name: String,

    /// File ID
    pub id: String,
}
```

#### SendRequest (Create/Edit)
```rust
/// Request model for creating/editing Send
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendRequest {
    #[serde(rename = "type")]
    pub send_type: SendType,

    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<SendTextRequest>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<SendFileRequest>,

    pub key: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_access_count: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiration_date: Option<String>,

    pub deletion_date: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,

    pub disabled: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub hide_email: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendTextRequest {
    pub text: String,
    pub hidden: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendFileRequest {
    pub file_name: String,
    pub size: u64,
    pub size_name: String,
}
```

#### SendAccess (Public Access)
```rust
/// Response from public Send access
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendAccess {
    /// Send ID
    pub id: String,

    /// Send type
    #[serde(rename = "type")]
    pub send_type: SendType,

    /// Encrypted name
    pub name: String,

    /// Text data (if type=0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<SendText>,

    /// File data (if type=1)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<SendFile>,

    /// Encrypted key
    pub key: String,

    /// Access count
    pub access_count: u32,

    /// Whether password required
    pub password_required: bool,
}
```

### Generator Configuration Models

**Location:** `crates/bw-core/src/services/generator/`

```rust
/// Password generation options
#[derive(Debug, Clone)]
pub struct PasswordOptions {
    pub length: usize,
    pub include_lowercase: bool,
    pub include_uppercase: bool,
    pub include_numbers: bool,
    pub include_special: bool,
    pub min_lowercase: usize,
    pub min_uppercase: usize,
    pub min_numbers: usize,
    pub min_special: usize,
    pub exclude_chars: Option<String>,
}

impl Default for PasswordOptions {
    fn default() -> Self {
        Self {
            length: 14,
            include_lowercase: true,
            include_uppercase: true,
            include_numbers: true,
            include_special: true,
            min_lowercase: 0,
            min_uppercase: 0,
            min_numbers: 1,
            min_special: 1,
            exclude_chars: None,
        }
    }
}

/// Passphrase generation options
#[derive(Debug, Clone)]
pub struct PassphraseOptions {
    pub num_words: usize,
    pub separator: String,
    pub capitalize: bool,
    pub include_number: bool,
}

impl Default for PassphraseOptions {
    fn default() -> Self {
        Self {
            num_words: 3,
            separator: "-".to_string(),
            capitalize: false,
            include_number: false,
        }
    }
}
```

## Service Interfaces

### Generator Service

**Location:** `crates/bw-core/src/services/generator/mod.rs`

```rust
/// Password and passphrase generation service
pub struct GeneratorService;

impl GeneratorService {
    /// Generate a password with given options
    ///
    /// # Arguments
    /// * `options` - Password generation configuration
    ///
    /// # Returns
    /// Generated password string
    ///
    /// # Errors
    /// - Invalid options (constraints cannot be satisfied)
    /// - RNG failure (should be extremely rare)
    pub fn generate_password(options: &PasswordOptions) -> Result<String, GeneratorError>;

    /// Generate a passphrase with given options
    ///
    /// # Arguments
    /// * `options` - Passphrase generation configuration
    ///
    /// # Returns
    /// Generated passphrase string
    ///
    /// # Errors
    /// - Invalid options (word count out of range)
    /// - RNG failure (should be extremely rare)
    pub fn generate_passphrase(options: &PassphraseOptions) -> Result<String, PassphraseError>;

    /// Validate password options for feasibility
    ///
    /// Checks that minimum requirements don't exceed length,
    /// at least one character set is enabled, etc.
    pub fn validate_password_options(options: &PasswordOptions) -> Result<(), GeneratorError>;

    /// Validate passphrase options
    pub fn validate_passphrase_options(options: &PassphraseOptions) -> Result<(), GeneratorError>;
}
```

### Send Service

**Location:** `crates/bw-core/src/services/send/send_service.rs`

```rust
/// Send CRUD operations service
pub struct SendService {
    api_client: Arc<dyn ApiClient>,
    encryption: Arc<SendEncryption>,
}

impl SendService {
    pub fn new(api_client: Arc<dyn ApiClient>, encryption: Arc<SendEncryption>) -> Self;

    /// Create a new Send
    ///
    /// # Arguments
    /// * `request` - Send creation request with plaintext data
    /// * `file_data` - Optional file content for file Sends
    ///
    /// # Returns
    /// Created Send object with access URL
    pub async fn create_send(
        &self,
        request: SendRequest,
        file_data: Option<Vec<u8>>,
    ) -> Result<Send, SendError>;

    /// List user's Sends
    pub async fn list_sends(&self) -> Result<Vec<Send>, SendError>;

    /// Get Send by ID
    pub async fn get_send(&self, id: &str) -> Result<Send, SendError>;

    /// Edit existing Send
    pub async fn edit_send(&self, id: &str, request: SendRequest) -> Result<Send, SendError>;

    /// Delete Send
    pub async fn delete_send(&self, id: &str) -> Result<(), SendError>;

    /// Remove password from Send
    pub async fn remove_password(&self, id: &str) -> Result<Send, SendError>;

    /// Access public Send (no auth required)
    ///
    /// # Arguments
    /// * `access_id` - Access ID from Send URL
    /// * `password` - Optional password if Send is password-protected
    ///
    /// # Returns
    /// Decrypted Send content
    pub async fn access_send(
        &self,
        access_id: &str,
        password: Option<&str>,
    ) -> Result<SendAccess, SendError>;

    /// Build Send template for JSON input
    pub fn create_template(send_type: SendType) -> SendRequest;
}
```

### Send Encryption Service

**Location:** `crates/bw-core/src/services/send/encryption.rs`

```rust
/// Send encryption/decryption service
pub struct SendEncryption {
    // SDK client when available, or crypto primitives
}

impl SendEncryption {
    /// Encrypt Send data for upload
    ///
    /// # Arguments
    /// * `data` - Plaintext Send data
    /// * `key` - Send encryption key
    ///
    /// # Returns
    /// Encrypted data in EncString format
    pub fn encrypt_send_data(&self, data: &str, key: &[u8]) -> Result<String, SendError>;

    /// Decrypt Send data after download
    pub fn decrypt_send_data(&self, encrypted: &str, key: &[u8]) -> Result<String, SendError>;

    /// Generate new Send encryption key
    pub fn generate_send_key(&self) -> Result<Vec<u8>, SendError>;

    /// Derive key from password for password-protected Sends
    pub fn derive_password_key(&self, password: &str, salt: &[u8]) -> Result<Vec<u8>, SendError>;
}
```

## API Integration

### Send API Endpoints

**Location:** `crates/bw-core/src/services/api/send_api.rs`

Add new trait to `ApiClient`:

```rust
#[async_trait]
pub trait SendApi {
    /// POST /sends - Create new Send
    async fn create_send(&self, request: &SendRequest) -> Result<Send, ApiError>;

    /// GET /sends - List user's Sends
    async fn list_sends(&self) -> Result<Vec<Send>, ApiError>;

    /// GET /sends/{id} - Get Send by ID
    async fn get_send(&self, id: &str) -> Result<Send, ApiError>;

    /// PUT /sends/{id} - Update Send
    async fn update_send(&self, id: &str, request: &SendRequest) -> Result<Send, ApiError>;

    /// DELETE /sends/{id} - Delete Send
    async fn delete_send(&self, id: &str) -> Result<(), ApiError>;

    /// PUT /sends/{id}/remove-password - Remove Send password
    async fn remove_send_password(&self, id: &str) -> Result<Send, ApiError>;

    /// GET /sends/access/{access_id} - Public Send access (no auth)
    async fn access_send(&self, access_id: &str, password: Option<&str>) -> Result<SendAccess, ApiError>;

    /// POST /sends/file/v2 - Upload file Send
    async fn upload_file_send(
        &self,
        request: &SendRequest,
        file_data: Vec<u8>,
    ) -> Result<Send, ApiError>;
}
```

**Implementation in:** `crates/bw-core/src/services/api/client.rs`

## Implementation Phases

### Phase 1: Password Generation (High Priority, 3-5 days)

**Goal:** Implement cryptographically secure password and passphrase generation.

#### Tasks:

1. **Create Generator Service Structure** (0.5 day)
   - Create `crates/bw-core/src/services/generator/` directory
   - Set up module structure (mod.rs, password.rs, passphrase.rs, errors.rs)
   - Define error types
   - Define configuration structs (PasswordOptions, PassphraseOptions)

2. **Implement Password Generation** (1 day)
   - File: `crates/bw-core/src/services/generator/password.rs`
   - Use `rand::rngs::OsRng` for CSPRNG
   - Implement character set selection logic:
     ```rust
     const LOWERCASE: &str = "abcdefghijklmnopqrstuvwxyz";
     const UPPERCASE: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
     const NUMBERS: &str = "0123456789";
     const SPECIAL: &str = "!@#$%^&*";
     ```
   - Implement minimum character requirements:
     1. Generate minimum required characters from each set
     2. Fill remaining length with random characters from all enabled sets
     3. Shuffle result using Fisher-Yates algorithm with OsRng
   - Handle excluded characters
   - Validate constraints (min sum ≤ length, at least one set enabled)

3. **Embed EFF Wordlist** (0.5 day)
   - File: `crates/bw-core/src/services/generator/wordlist.rs`
   - Download EFF long wordlist (7776 words)
   - Embed using `include_str!` or const array
   - Implement word selection using uniform random index

4. **Implement Passphrase Generation** (1 day)
   - File: `crates/bw-core/src/services/generator/passphrase.rs`
   - Select N words using OsRng
   - Apply capitalization if requested
   - Join with separator
   - Append random number if requested
   - Validate word count (3-20 range)

5. **CLI Integration** (1 day)
   - File: `crates/bw-cli/src/commands/tools.rs`
   - Implement `execute_generate`:
     ```rust
     pub async fn execute_generate(
         cmd: GenerateCommand,
         global_args: &GlobalArgs,
     ) -> anyhow::Result<Response> {
         if cmd.passphrase {
             let options = PassphraseOptions {
                 num_words: cmd.words.unwrap_or(3),
                 separator: cmd.separator.unwrap_or_else(|| "-".to_string()),
                 capitalize: cmd.capitalize,
                 include_number: cmd.include_number,
             };
             let passphrase = GeneratorService::generate_passphrase(&options)?;
             Ok(Response::success_raw(passphrase))
         } else {
             let options = PasswordOptions {
                 length: cmd.length.unwrap_or(14),
                 min_lowercase: cmd.lowercase.unwrap_or(0),
                 min_uppercase: cmd.uppercase.unwrap_or(0),
                 min_numbers: cmd.number.unwrap_or(1),
                 min_special: cmd.special.unwrap_or(1),
                 ..Default::default()
             };
             let password = GeneratorService::generate_password(&options)?;
             Ok(Response::success_raw(password))
         }
     }
     ```
   - Handle `--response` flag for JSON output
   - Handle error cases with clear messages

6. **Testing** (1 day)
   - Unit tests:
     - Password length verification
     - Character set presence
     - Minimum requirements satisfied
     - Excluded characters work
     - Constraint validation
     - RNG is OsRng (verify type)
   - Integration tests:
     - CLI argument parsing
     - Output format (plain text vs JSON)
     - Error messages
   - Property tests:
     - Generated passwords always meet constraints
     - Passphrase word count correct

**Deliverables:**
- Working `bw generate` command
- Working `bw generate --passphrase` command
- Full test coverage
- Documentation comments

**Success Criteria:**
- All tests passing
- Generates cryptographically secure passwords
- Performance <100ms (P95)
- Output matches TypeScript CLI format

### Phase 2: Encode Utility (High Priority, 0.5 day)

**Goal:** Implement base64 encoding utility.

#### Tasks:

1. **Implement Encode Command** (0.25 day)
   - File: `crates/bw-cli/src/commands/tools.rs`
   - Use `base64` crate (already in workspace)
   - Implement `execute_encode`:
     ```rust
     pub async fn execute_encode(
         cmd: EncodeCommand,
         global_args: &GlobalArgs,
     ) -> anyhow::Result<Response> {
         use base64::{Engine as _, engine::general_purpose};
         let encoded = general_purpose::STANDARD.encode(&cmd.data);

         if global_args.response {
             Ok(Response::success_json(serde_json::json!({
                 "data": encoded
             })))
         } else {
             Ok(Response::success_raw(encoded))
         }
     }
     ```

2. **Testing** (0.25 day)
   - Unit tests with known base64 values
   - Integration tests for CLI
   - Test `--response` flag

**Deliverables:**
- Working `bw encode` command
- Test coverage

**Success Criteria:**
- Produces standard RFC 4648 base64
- Matches TypeScript CLI output
- Performance <100ms

### Phase 3: Send Models & Encryption (High Priority, 4-6 days)

**Goal:** Define Send data models and implement encryption.

#### Tasks:

1. **Create Send Model Structure** (1 day)
   - Create `crates/bw-core/src/models/send/` directory
   - Implement models:
     - `send.rs` - Core Send model
     - `send_text.rs` - Text Send type
     - `send_file.rs` - File Send type
     - `send_request.rs` - Request models
     - `send_access.rs` - Access models
     - `mod.rs` - Public exports
   - Add serialization tests matching API format
   - Document all fields with API behavior

2. **Research SDK Send APIs** (0.5 day)
   - Investigate if Bitwarden SDK provides:
     - Send encryption/decryption methods
     - Key generation for Sends
     - Password derivation
   - Document findings
   - **Decision:** Use SDK if available, otherwise implement with rust-crypto

3. **Implement Send Encryption** (2-3 days)

   **If SDK Available:**
   - File: `crates/bw-core/src/services/send/encryption.rs`
   - Wrap SDK encryption methods
   - Implement error conversion
   - Add tests with test vectors

   **If SDK Not Available:**
   - Use rust-crypto primitives:
     - AES-256-CBC for content encryption
     - HMAC-SHA256 for authentication
     - PBKDF2 for password derivation
   - Match Bitwarden EncString format: `"2.base64_iv|base64_ciphertext|base64_mac"`
   - Implement:
     ```rust
     impl SendEncryption {
         pub fn encrypt_send_data(&self, data: &str, key: &[u8]) -> Result<String> {
             // 1. Generate random IV
             // 2. Encrypt with AES-256-CBC
             // 3. Generate HMAC
             // 4. Format as EncString
         }

         pub fn decrypt_send_data(&self, encrypted: &str, key: &[u8]) -> Result<String> {
             // 1. Parse EncString
             // 2. Verify HMAC
             // 3. Decrypt with AES-256-CBC
         }

         pub fn generate_send_key(&self) -> Result<Vec<u8>> {
             // Generate 32-byte key with OsRng
         }
     }
     ```

4. **Cross-Validation with Web Vault** (0.5 day)
   - Create test Send in web vault
   - Retrieve and decrypt with Rust implementation
   - Create Send with Rust, verify in web vault
   - Document encryption format

5. **Testing** (1 day)
   - Unit tests for encryption/decryption
   - Test with known test vectors
   - Round-trip tests (encrypt then decrypt)
   - Error case tests (wrong key, corrupted data)
   - Cross-platform validation tests

**Deliverables:**
- Send models in `models/send/`
- Send encryption service
- Comprehensive tests
- Documentation of encryption format

**Success Criteria:**
- Models serialize to match API format
- Encryption compatible with web vault
- All tests passing
- Security review approved

### Phase 4: Send API Integration (High Priority, 2-3 days)

**Goal:** Add Send endpoints to API client.

#### Tasks:

1. **Define Send API Trait** (0.5 day)
   - File: `crates/bw-core/src/services/api/send_api.rs`
   - Define `SendApi` trait with all endpoints
   - Document each method with API contract
   - Define request/response types

2. **Implement Send API Methods** (1.5 days)
   - File: `crates/bw-core/src/services/api/client.rs`
   - Implement `SendApi` for `BitwardenApiClient`:
     ```rust
     async fn create_send(&self, request: &SendRequest) -> Result<Send, ApiError> {
         let url = self.build_url("/sends", false);
         let response = self.post(url)
             .json(request)
             .send_authenticated()
             .await?;
         response.json().await.map_err(Into::into)
     }

     async fn access_send(&self, access_id: &str, password: Option<&str>) -> Result<SendAccess, ApiError> {
         let url = self.build_url(&format!("/sends/access/{}", access_id), false);
         let mut request = self.get(url);
         if let Some(pwd) = password {
             request = request.header("X-Send-Password", pwd);
         }
         let response = request.send().await?; // No auth needed
         response.json().await.map_err(Into::into)
     }
     ```
   - Handle authentication for private endpoints
   - No authentication for public receive endpoint
   - Implement file upload with multipart/form-data

3. **Testing** (1 day)
   - Mock API tests for all endpoints
   - Integration tests with test Bitwarden account
   - Test error cases (401, 404, expired Send)
   - Test file upload

**Deliverables:**
- Send API methods in API client
- API integration tests
- Error handling

**Success Criteria:**
- All API methods work with live API
- Proper authentication handling
- Clear error messages

### Phase 5: Send CRUD Commands (High Priority, 3-4 days)

**Goal:** Implement Send management commands.

#### Tasks:

1. **Implement Send Service** (1 day)
   - File: `crates/bw-core/src/services/send/send_service.rs`
   - Implement `SendService` methods
   - Integrate encryption and API client
   - Handle Send URL generation:
     ```rust
     fn build_send_url(&self, send: &Send) -> String {
         format!("{}/#/send/{}/{}",
             self.web_vault_url,
             send.access_id,
             urlsafe_base64(send.key)
         )
     }
     ```

2. **Implement Send List Command** (0.5 day)
   - File: `crates/bw-cli/src/commands/send.rs`
   - Execute: `SendCommands::List`
   - Fetch Sends from API
   - Format as JSON array
   - Include expiration status

3. **Implement Send Create Command** (1 day)
   - Execute: `SendCommands::Create`
   - Parse JSON or use `--text` flag
   - Validate inputs
   - Generate encryption key
   - Encrypt content
   - Upload to API
   - Return Send with access URL
   - Handle `--hidden` flag for text Sends

4. **Implement Send Get Command** (0.5 day)
   - Execute: `SendCommands::Get`
   - Fetch by ID
   - Return full Send JSON

5. **Implement Send Delete Command** (0.5 day)
   - Execute: `SendCommands::Delete`
   - Call delete API
   - Return success message

6. **Implement Send Template Command** (0.5 day)
   - Execute: `SendCommands::Template`
   - Return JSON template for text or file Send
   - Include all fields with defaults

7. **Testing** (1 day)
   - Integration tests for each command
   - Test with various Send configurations
   - Test expiration and max access count
   - Test output formatting

**Deliverables:**
- Working Send CRUD commands
- Send service implementation
- Integration tests

**Success Criteria:**
- Can create, list, get, delete Sends
- Output matches TypeScript CLI
- Proper error handling
- Performance <2s (P95)

### Phase 6: Receive Command (High Priority, 2-3 days)

**Goal:** Implement public Send access.

#### Tasks:

1. **Add Receive Command Structure** (0.5 day)
   - File: `crates/bw-cli/src/commands/receive.rs` [NEW]
   - Define command:
     ```rust
     #[derive(Args)]
     pub struct ReceiveCommand {
         /// Send URL or access ID
         #[arg(value_name = "URL")]
         pub url: String,

         /// Password for password-protected Send
         #[arg(long)]
         pub password: Option<String>,
     }
     ```
   - Update `main.rs` to include Receive command
   - Add to Commands enum

2. **Implement URL Parsing** (0.5 day)
   - Parse Send URL formats:
     - `https://send.bitwarden.com/#/send/ACCESS_ID/KEY`
     - `https://vault.bitwarden.com/#/send/ACCESS_ID/KEY`
     - Just `ACCESS_ID` (for testing)
   - Extract access ID and decryption key

3. **Implement Receive Logic** (1 day)
   - File: `crates/bw-cli/src/commands/receive.rs`
   - Parse URL to get access ID and key
   - Call public API (no auth)
   - Prompt for password if required:
     ```rust
     let password = if send_access.password_required {
         if let Some(pwd) = cmd.password {
             Some(pwd)
         } else if !global_args.nointeraction {
             Some(prompt_password("Enter Send password:")?)
         } else {
             return Err(anyhow!("Send requires password (use --password)"));
         }
     } else {
         None
     };
     ```
   - Decrypt content
   - Output text or download file
   - Handle errors (expired, access limit, wrong password)

4. **Testing** (1 day)
   - Test with public Sends
   - Test with password-protected Sends
   - Test expired Sends (verify error)
   - Test access limit exceeded
   - Test --nointeraction flag
   - Test URL parsing variations

**Deliverables:**
- Working `bw receive` command
- URL parsing logic
- Password prompting
- Error handling

**Success Criteria:**
- Can receive public Sends
- Handles password-protected Sends
- Clear error messages
- Works without authentication
- Performance <2s (P95)

### Phase 7: Optional Send Features (Medium Priority, 3-4 days)

**Goal:** Implement nice-to-have features.

#### Tasks:

1. **Send Edit Command** (1 day)
   - Execute: `SendCommands::Edit`
   - Parse JSON with updates
   - Validate mutable fields (name, notes, expiration, max access count)
   - Prevent changing type or content
   - Call update API

2. **Send Remove Password Command** (0.5 day)
   - Execute: `SendCommands::RemovePassword`
   - Call remove password API
   - Return success

3. **File Send Support** (1.5-2 days)
   - Implement file reading
   - Stream file content for large files
   - Encrypt file data
   - Upload with multipart/form-data
   - Show progress indicator for large files
   - Test with various file sizes

4. **Testing** (1 day)
   - Test edit command
   - Test remove password
   - Test file Sends with various sizes
   - Test progress indicators

**Deliverables:**
- Optional Send commands
- File Send support
- Progress indicators

**Success Criteria:**
- Edit and remove password work
- File Sends work with streaming
- Progress shown for large files
- Memory usage <10MB for file operations

### Phase 8: Integration & Polish (High Priority, 2-3 days)

**Goal:** Ensure compatibility and quality.

#### Tasks:

1. **Output Format Compatibility** (1 day)
   - Compare all command outputs with TypeScript CLI
   - Fix any discrepancies in JSON structure
   - Verify error message format
   - Test all global flags (--quiet, --response, etc.)

2. **Performance Optimization** (0.5 day)
   - Profile password generation
   - Profile Send operations
   - Optimize hot paths if needed
   - Verify performance targets met

3. **Security Review** (1 day)
   - Review all crypto operations
   - Verify OsRng usage
   - Check for memory clearing (zeroize)
   - Verify no sensitive data in logs
   - Check input validation

4. **Documentation** (0.5 day)
   - Add doc comments to all public APIs
   - Create usage examples
   - Document Send encryption format
   - Update README if needed

**Deliverables:**
- Full TypeScript CLI compatibility
- Performance meeting targets
- Security approved
- Complete documentation

**Success Criteria:**
- All outputs match TypeScript CLI
- All performance targets met
- No security vulnerabilities
- Documentation complete

## Testing Strategy

### Unit Tests

**Coverage Areas:**
- Password generation algorithm
- Passphrase generation
- Send encryption/decryption
- URL parsing
- Validation logic
- Error handling

**Framework:** Rust `#[test]` with `tokio::test` for async

**Target:** >80% code coverage

### Integration Tests

**Coverage Areas:**
- Full command execution (CLI → Service → API)
- Global flag behavior
- Output formatting
- Send CRUD workflows
- Receive workflows
- Authentication checks

**Requirements:**
- Test Bitwarden account
- Mock API server for offline tests

### Compatibility Tests

**Approach:**
- Run same commands on TypeScript and Rust CLIs
- Compare outputs (ignoring timestamps/IDs)
- Verify JSON structure matches
- Verify error messages similar

**Coverage:**
- All generate options
- All Send operations
- Receive from TypeScript-created Sends
- Encode command

### Security Tests

**Coverage:**
- Verify OsRng usage (not thread_rng)
- Memory clearing validation
- No sensitive data in logs
- Input validation (fuzzing)
- Cross-validate encryption with web vault

**Tools:**
- `cargo-audit` for dependencies
- Memory profiler (valgrind/heaptrack)
- `cargo-fuzz` for fuzzing

### Performance Tests

**Metrics:**
- Password generation: P50, P95, P99 latency
- Send operations: P50, P95, P99 latency
- Memory usage for file operations

**Targets:**
- Generate: <100ms (P95)
- Send operations: <2s (P95)
- Memory: <10MB for file operations

## Technical Decisions & Rationale

### Decision 1: Direct Password Generation vs SDK

**Decision:** Implement password generation directly with `rand` crate

**Rationale:**
- SDK is currently mocked (not available)
- Password generation is straightforward with OsRng
- No external dependencies needed beyond `rand`
- Full control over implementation
- Can optimize for performance
- If SDK becomes available later, can refactor

**Trade-offs:**
- Pro: Works immediately, no SDK dependency
- Pro: Full control and testability
- Con: Duplicate logic if SDK also provides it
- Con: Need to maintain generator code

### Decision 2: Embedded EFF Wordlist

**Decision:** Embed EFF long wordlist in binary

**Rationale:**
- Guarantees availability (no external file)
- Wordlist is small (~60KB)
- No runtime file I/O needed
- Simplified deployment
- Matches TypeScript CLI approach

**Trade-offs:**
- Pro: Reliable, always available
- Pro: Fast (no disk I/O)
- Con: Increases binary size by ~60KB
- Con: Cannot customize wordlist without rebuild

**Implementation:**
```rust
// crates/bw-core/src/services/generator/wordlist.rs
const EFF_WORDLIST: &str = include_str!("eff_large_wordlist.txt");

pub fn get_wordlist() -> Vec<&'static str> {
    EFF_WORDLIST.lines().collect()
}
```

### Decision 3: Send Encryption Approach

**Decision:** Use SDK if available, fallback to rust-crypto

**Rationale:**
- SDK provides vetted, compatible encryption
- If SDK unavailable, implement with standard crypto libraries
- Must match web vault encryption format
- Security is critical

**Implementation Strategy:**
1. Check if SDK provides Send encryption APIs
2. If yes, wrap SDK methods
3. If no, implement with:
   - `aes` crate for AES-256-CBC
   - `hmac` and `sha2` for authentication
   - `pbkdf2` for password derivation
   - Match EncString format exactly

**Trade-offs:**
- Pro: SDK ensures compatibility
- Pro: Fallback keeps project moving
- Con: Custom crypto requires careful review
- Con: Must maintain compatibility

### Decision 4: Receive as Top-Level Command

**Decision:** Make `receive` a top-level command, not under `send`

**Rationale:**
- Matches TypeScript CLI convention: `bw receive`
- Receive doesn't require authentication (different security model)
- Semantically separate from Send management
- Simpler UX (shorter command)

**Implementation:**
- Add `Receive` variant to `Commands` enum in `main.rs`
- Create `commands/receive.rs` module
- Add to command router

### Decision 5: File Send Streaming

**Decision:** Use streaming for file Send uploads/downloads

**Rationale:**
- Prevents memory exhaustion with large files
- Matches production requirements
- Better user experience with progress
- API may support chunked uploads

**Implementation:**
```rust
// Stream file in chunks
const CHUNK_SIZE: usize = 1024 * 1024; // 1MB chunks

async fn upload_file_send(file_path: &Path) -> Result<Send> {
    let file = File::open(file_path).await?;
    let mut reader = BufReader::new(file);
    let mut buffer = vec![0u8; CHUNK_SIZE];

    // Stream upload with progress
    while let n = reader.read(&mut buffer).await? {
        if n == 0 { break; }
        // Encrypt and upload chunk
        upload_chunk(&buffer[..n]).await?;
    }
}
```

**Trade-offs:**
- Pro: Memory efficient
- Pro: Works with large files
- Pro: Can show progress
- Con: More complex implementation
- Con: API must support streaming

### Decision 6: Send URL Format

**Decision:** Generate Send URLs using Bitwarden web vault URL format

**Format:** `https://send.bitwarden.com/#/send/{access_id}/{urlsafe_base64(key)}`

**Rationale:**
- Matches TypeScript CLI output
- Compatible with web vault
- Includes key in URL for convenience
- Standard Bitwarden format

**Implementation:**
```rust
fn build_send_url(send: &Send, web_vault_url: &str) -> String {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    let key_b64 = URL_SAFE_NO_PAD.encode(&send.key);
    format!("{}/#/send/{}/{}", web_vault_url, send.access_id, key_b64)
}
```

## Error Handling Strategy

### Generator Errors

```rust
#[derive(Debug, thiserror::Error)]
pub enum GeneratorError {
    #[error("Invalid password options: {0}")]
    InvalidOptions(String),

    #[error("Password length {0} is invalid (must be 5-128)")]
    InvalidLength(usize),

    #[error("Minimum character requirements ({0}) exceed password length ({1})")]
    RequirementsExceedLength(usize, usize),

    #[error("No character sets enabled")]
    NoCharacterSets,

    #[error("RNG failure: {0}")]
    RngError(String),

    #[error("Invalid passphrase options: {0}")]
    InvalidPassphraseOptions(String),
}
```

### Send Errors

```rust
#[derive(Debug, thiserror::Error)]
pub enum SendError {
    #[error("Encryption error: {0}")]
    Encryption(String),

    #[error("API error: {0}")]
    Api(#[from] ApiError),

    #[error("Send not found: {0}")]
    NotFound(String),

    #[error("Send expired")]
    Expired,

    #[error("Send access limit exceeded")]
    AccessLimitExceeded,

    #[error("Invalid password")]
    InvalidPassword,

    #[error("Invalid Send URL: {0}")]
    InvalidUrl(String),

    #[error("File error: {0}")]
    FileError(#[from] std::io::Error),
}
```

### Error Message Guidelines

1. **Be specific:** "Send expired on 2025-11-30" not "Send unavailable"
2. **Suggest fix:** "Not authenticated. Run 'bw login' or use --session"
3. **Include context:** Show Send ID, file path, etc.
4. **Respect flags:** Honor `--quiet` and `--cleanexit`
5. **Match TypeScript:** Similar error wording for compatibility

## Security Considerations

### Critical Security Requirements

1. **Cryptographic Randomness**
   - MUST use `rand::rngs::OsRng` for all password/passphrase generation
   - NEVER use `thread_rng()` or other PRNGs
   - Verify with tests that check RNG type

2. **Memory Clearing**
   - Use `zeroize` crate for sensitive data
   - Apply `#[zeroize(drop)]` to types containing secrets
   - Clear passwords, keys, decrypted content immediately after use

3. **No Logging of Secrets**
   - NEVER log generated passwords
   - NEVER log encryption keys or tokens
   - Use `secrecy::Secret` wrapper for automatic protection
   - Review all `tracing::` calls in sensitive code

4. **Encryption Best Practices**
   - Use SDK encryption when available (vetted implementation)
   - If custom crypto needed, use well-known libraries
   - Match Bitwarden EncString format exactly
   - Security review of all crypto code before release

5. **Input Validation**
   - Validate all user inputs (bounds, format, etc.)
   - Sanitize before passing to external systems
   - Use parameterized API calls (no string interpolation)
   - Test with fuzzing to find edge cases

6. **File Handling**
   - Enforce file size limits (prevent DoS)
   - Use streaming to avoid loading full file in memory
   - Validate file metadata
   - Set timeouts for long operations

### Security Testing Checklist

- [ ] Verify OsRng usage in generator
- [ ] Test memory clearing with profiler
- [ ] Audit logs for sensitive data leaks
- [ ] Fuzz test input validation
- [ ] Cross-validate Send encryption with web vault
- [ ] Review all `unsafe` code (should be none)
- [ ] Run `cargo audit` for dependency vulnerabilities
- [ ] Test password prompt hiding
- [ ] Verify API calls are parameterized
- [ ] Check file size limits enforced

## Performance Targets & Optimization

### Targets

| Operation | Target (P95) | Critical Path |
|-----------|--------------|---------------|
| Password generation | <100ms | OsRng, character selection |
| Passphrase generation | <100ms | OsRng, word selection |
| Send create (text) | <2s | Encryption, API call |
| Send list | <2s | API call, deserialization |
| Send get | <2s | API call |
| Send delete | <2s | API call |
| Receive | <2s | API call, decryption |
| Encode | <100ms | Base64 encoding |

### Optimization Strategies

1. **Password Generation**
   - Pre-allocate string capacity
   - Use efficient character selection (slice indexing)
   - Minimize allocations in hot path

2. **Encryption**
   - Reuse crypto contexts when possible
   - Use vectorized operations (AES-NI)
   - Profile encryption/decryption paths

3. **API Calls**
   - Use connection pooling (reqwest default)
   - Enable HTTP/2
   - Set reasonable timeouts
   - Consider compression for large payloads

4. **File Operations**
   - Stream data (don't load full file)
   - Use async I/O
   - Appropriate buffer sizes (1MB chunks)

## Dependencies

### New Dependencies Required

```toml
[workspace.dependencies]
# Cryptography (for Send encryption if SDK unavailable)
aes = "0.8"           # AES encryption
hmac = "0.12"         # HMAC authentication
pbkdf2 = "0.12"       # Password key derivation

# These already exist in workspace:
rand = "0.8"          # CSPRNG
base64 = "0.22"       # Base64 encoding
secrecy = "0.8"       # Secret protection
zeroize = "1.8"       # Memory clearing
```

### Dependency Justification

- `rand` with `OsRng`: Cryptographically secure random for passwords
- `base64`: Standard base64 encoding for encode command
- `aes`, `hmac`: Encryption primitives if SDK unavailable
- `pbkdf2`: Password-based key derivation for Send passwords
- `secrecy`: Automatic secret protection in types
- `zeroize`: Secure memory clearing for sensitive data

## Integration Points

### With Storage Layer (Enhancement 2)

- **Used for:** Storing Send encryption keys (if needed)
- **Interface:** `JsonFileStorage` for key persistence
- **Location:** `crates/bw-core/src/services/storage/`

### With API Client (Enhancement 3)

- **Used for:** All Send API operations
- **Interface:** `ApiClient` trait with new `SendApi` methods
- **Location:** `crates/bw-core/src/services/api/`

### With Authentication (Enhancement 4)

- **Used for:** Session management for Send CRUD
- **Interface:** Session token from `SessionManager`
- **Note:** Receive command does NOT require authentication

### With SDK (Enhancement 1)

- **Used for:** Send encryption/decryption (when available)
- **Interface:** Mock client → Real SDK client
- **Fallback:** Custom crypto implementation if SDK unavailable

## Migration from TypeScript CLI

### Behavior Compatibility

| Feature | TypeScript Behavior | Rust Implementation |
|---------|---------------------|---------------------|
| Default password length | 14 characters | Same: 14 |
| Default password sets | Upper, lower, numbers, special | Same |
| Default min numbers | 1 | Same: 1 |
| Default min special | 1 | Same: 1 |
| Default passphrase words | 3 | Same: 3 |
| Default separator | `-` | Same: `-` |
| Send URL format | `send.bitwarden.com/#/send/...` | Same format |
| Receive without auth | Supported | Same |
| Password prompt | Interactive with hidden input | Same with dialoguer |

### Output Format Compatibility

**Generate (plain):**
```
$ bw generate
Kx9!mP2zQw3nL5

$ bw generate --passphrase
correct-horse-battery
```

**Generate (JSON):**
```json
{
  "data": "Kx9!mP2zQw3nL5"
}
```

**Send create:**
```json
{
  "id": "...",
  "accessId": "...",
  "name": "...",
  "type": 0,
  "accessUrl": "https://send.bitwarden.com/#/send/..."
}
```

### Breaking Changes

None expected. Full backward compatibility maintained.

## Documentation Requirements

### Code Documentation

1. **Module-level docs:** Explain purpose of each module
2. **Function docs:** Document parameters, returns, errors, examples
3. **Type docs:** Explain each field and its constraints
4. **Example code:** Show typical usage patterns

### User Documentation

1. **Command help:** Update `--help` text for all commands
2. **Examples:** Provide common usage examples
3. **Error messages:** Document what they mean and how to fix
4. **Security notes:** Explain password generation security

### Internal Documentation

1. **Encryption format:** Document EncString structure
2. **API contracts:** Document expected API behavior
3. **Architecture decisions:** Explain key choices
4. **Testing approach:** Document test strategy

## Risk Mitigation

### Risk 1: SDK Encryption APIs Unavailable

**Mitigation:**
- Planned fallback to rust-crypto implementation
- Early research to determine SDK capabilities
- Budget extra time for custom crypto if needed
- Security review process for custom implementation

### Risk 2: API Compatibility Issues

**Mitigation:**
- Test with live API early and often
- Cross-validate with web vault
- Document API behavior as we learn it
- Have test Bitwarden account for integration tests

### Risk 3: Performance Targets Not Met

**Mitigation:**
- Profile early in development
- Optimize hot paths proactively
- Use efficient algorithms (Fisher-Yates shuffle, etc.)
- Set up performance benchmarks in CI

### Risk 4: File Send Complexity

**Mitigation:**
- Mark file Send as "optional" (can defer if needed)
- Implement text Send first (simpler, higher priority)
- Research API file upload requirements early
- Consider incremental implementation (small files first)

## Success Metrics

### Functional Metrics

- [ ] All "Must Have" commands implemented and working
- [ ] Output format matches TypeScript CLI
- [ ] All user stories accepted
- [ ] All acceptance criteria met

### Quality Metrics

- [ ] >80% code coverage
- [ ] All tests passing
- [ ] No security vulnerabilities
- [ ] Zero compiler warnings

### Performance Metrics

- [ ] Password generation <100ms (P95)
- [ ] Send operations <2s (P95)
- [ ] Memory usage <10MB for file operations

### Compatibility Metrics

- [ ] All outputs match TypeScript CLI structure
- [ ] Sends created by Rust CLI work in web vault
- [ ] Sends created by TypeScript CLI accessible from Rust CLI
- [ ] Error messages similar to TypeScript CLI

## Timeline Summary

| Phase | Duration | Priority | Dependencies |
|-------|----------|----------|--------------|
| 1. Password Generation | 3-5 days | High | None |
| 2. Encode Utility | 0.5 day | High | None |
| 3. Send Models & Encryption | 4-6 days | High | Phase 1 complete |
| 4. Send API Integration | 2-3 days | High | Phase 3 complete |
| 5. Send CRUD Commands | 3-4 days | High | Phase 4 complete |
| 6. Receive Command | 2-3 days | High | Phase 3-4 complete |
| 7. Optional Send Features | 3-4 days | Medium | Phase 5 complete |
| 8. Integration & Polish | 2-3 days | High | All phases |
| **Total** | **20-32 days** | | |

**Critical Path:** Phases 3 → 4 → 5 → 6 → 8 (16-22 days)
**Parallel Work:** Phase 1 and 2 can start immediately

## Next Steps for Implementation

1. **Begin Phase 1:** Start with password generation (no dependencies)
2. **Research SDK:** Investigate SDK Send encryption capabilities early
3. **Set up tests:** Create test Bitwarden account for integration tests
4. **Code review setup:** Establish security review process
5. **Performance baseline:** Set up benchmarking infrastructure

---

**Status:** READY_FOR_IMPLEMENTATION

This plan provides a comprehensive roadmap for implementing the Tool Commands enhancement. The implementation can begin immediately with Phase 1 (Password Generation) while researching SDK capabilities for later phases. All architectural decisions have been made with clear rationale, and risks have been identified with mitigation strategies.
