---
enhancement: 01-project-bootstrap
agent: architect
task_id: task_1764790706_10298
timestamp: 2025-12-03T14:45:00-08:00
status: READY_FOR_IMPLEMENTATION
---

# Implementation Plan: CLI Rust Migration - Project Bootstrap

## Executive Summary

This document provides the technical architecture and implementation plan for bootstrapping the Rust CLI project. The design prioritizes **interface compatibility**, **extensibility**, and **clean separation of concerns** to support the 8-phase migration plan.

**Key Architectural Decisions:**
1. **Workspace Structure**: Two-crate workspace (bw-cli binary, bw-core library) for modularity
2. **CLI Framework**: clap v4 with derive macros for type-safe argument parsing
3. **Command Pattern**: Trait-based command execution for extensibility
4. **SDK Integration**: Service container pattern for dependency injection
5. **Error Handling**: anyhow::Error for simplicity in bootstrap phase
6. **Output Formatting**: Enum-based Response type with mode-specific formatting

## System Architecture

### High-Level Component Diagram

```
┌─────────────────────────────────────────────────────────┐
│                      bw-cli (binary)                    │
├─────────────────────────────────────────────────────────┤
│  main.rs                                                │
│    ├─> CLI Parsing (clap)                              │
│    ├─> Global Args & Env Vars                          │
│    └─> Command Router                                  │
│                                                         │
│  commands/ (command modules)                            │
│    ├─> auth/  (login, logout, lock, unlock, status)   │
│    ├─> vault/ (list, get, create, edit, delete, etc)  │
│    ├─> sync/  (sync)                                   │
│    ├─> tools/ (generate, encode, decrypt, etc)        │
│    ├─> send/  (send operations)                       │
│    └─> config/ (config server)                        │
│                                                         │
│  output/                                                │
│    ├─> Response (enum)                                 │
│    └─> Formatter (JSON, pretty, raw, quiet)           │
└─────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────┐
│                   bw-core (library)                     │
├─────────────────────────────────────────────────────────┤
│  services/                                              │
│    ├─> ServiceContainer (DI container)                 │
│    └─> sdk.rs (SDK client initialization)              │
│                                                         │
│  models/                                                │
│    └─> Domain types (placeholder for future)           │
└─────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────┐
│              Bitwarden SDK (external)                   │
├─────────────────────────────────────────────────────────┤
│  bitwarden-core                                         │
│    └─> Client, Auth, Vault, Generators                 │
│                                                         │
│  bitwarden-crypto                                       │
│    └─> Encryption, Decryption, Key Derivation          │
└─────────────────────────────────────────────────────────┘
```

### Data Flow

```
User Command
    ↓
┌─────────────────┐
│ CLI Parsing     │  Parse args, validate flags
│ (main.rs)       │
└────────┬────────┘
         ↓
┌─────────────────┐
│ Global Args     │  Process --session, --quiet, etc
│ Extraction      │
└────────┬────────┘
         ↓
┌─────────────────┐
│ Command Router  │  Match command enum
│                 │
└────────┬────────┘
         ↓
┌─────────────────┐
│ Command Stub    │  Return "Not yet implemented"
│ (commands/)     │
└────────┬────────┘
         ↓
┌─────────────────┐
│ Response        │  Wrap in Response::Error
│ Creation        │
└────────┬────────┘
         ↓
┌─────────────────┐
│ Output          │  Format based on --response, --pretty, etc
│ Formatting      │
└────────┬────────┘
         ↓
    stdout/stderr
```

## 1. Cargo Workspace Structure

### 1.1 Workspace Manifest

**File:** `Cargo.toml` (workspace root)

```toml
[workspace]
resolver = "2"
members = [
    "crates/bw-cli",
    "crates/bw-core",
]

[workspace.package]
version = "0.1.0"
edition = "2021"
authors = ["Bitwarden Inc."]
license = "GPL-3.0"
rust-version = "1.70.0"  # Minimum supported Rust version

[workspace.dependencies]
# CLI Framework
clap = { version = "4.5", features = ["derive", "env", "wrap_help"] }

# Async Runtime
tokio = { version = "1.40", features = ["full"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Error Handling
anyhow = "1.0"
thiserror = "1.0"

# Bitwarden SDK (path dependencies)
bitwarden-core = { path = "../sdk/crates/bitwarden-core" }
bitwarden-crypto = { path = "../sdk/crates/bitwarden-crypto" }

# HTTP Client
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "json"] }

# Security
secrecy = "0.8"
zeroize = "1.8"

# Utilities
directories = "5.0"
base64 = "0.22"
rand = "0.8"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

[profile.release]
opt-level = "z"        # Optimize for size
lto = true             # Link-time optimization
codegen-units = 1      # Better optimization
strip = true           # Strip symbols
panic = "abort"        # Smaller binary
```

**Rationale:**
- **Workspace resolver = "2"**: Modern dependency resolution
- **Shared version/metadata**: Consistent versioning across crates
- **Workspace dependencies**: Single source of truth for versions
- **Profile optimization**: Target <5MB binary size
- **Path dependencies**: Development against local SDK

### 1.2 Binary Crate (bw-cli)

**File:** `crates/bw-cli/Cargo.toml`

```toml
[package]
name = "bw-cli"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
rust-version.workspace = true

[[bin]]
name = "bw"
path = "src/main.rs"

[dependencies]
# Workspace dependencies
clap.workspace = true
tokio.workspace = true
serde.workspace = true
serde_json.workspace = true
anyhow.workspace = true
tracing.workspace = true
tracing-subscriber.workspace = true

# Internal dependencies
bw-core = { path = "../bw-core" }

[dev-dependencies]
assert_cmd = "2.0"
predicates = "3.1"
```

**Rationale:**
- **Binary name "bw"**: Matches TypeScript CLI
- **Minimal dependencies**: CLI parsing and core library
- **Test dependencies**: Integration testing with assert_cmd

### 1.3 Library Crate (bw-core)

**File:** `crates/bw-core/Cargo.toml`

```toml
[package]
name = "bw-core"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
rust-version.workspace = true

[dependencies]
# Workspace dependencies
tokio.workspace = true
serde.workspace = true
serde_json.workspace = true
anyhow.workspace = true
thiserror.workspace = true
tracing.workspace = true

# Bitwarden SDK
bitwarden-core.workspace = true
bitwarden-crypto.workspace = true

# HTTP and Storage (for future enhancements)
reqwest.workspace = true
directories.workspace = true

# Security
secrecy.workspace = true
zeroize.workspace = true

[dev-dependencies]
tokio = { workspace = true, features = ["test-util"] }
```

**Rationale:**
- **Library crate**: Reusable business logic
- **SDK dependencies**: Direct access to crypto and vault operations
- **No CLI dependencies**: Clean separation from presentation layer

## 2. CLI Parsing Architecture

### 2.1 Command Structure Design

**File:** `crates/bw-cli/src/main.rs`

```rust
use clap::{Parser, Subcommand};
use std::process::ExitCode;

/// Bitwarden CLI - A secure and free password manager for all of your devices
#[derive(Parser)]
#[command(
    name = "bw",
    version,
    about = "Bitwarden CLI",
    long_about = "A secure and free password manager for all of your devices.\n\n\
                  Documentation: https://bitwarden.com/help/cli",
    after_help = "Use 'bw <command> --help' for more information about a command."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[command(flatten)]
    global_args: GlobalArgs,
}

/// Global flags available on all commands
#[derive(Parser, Debug, Clone)]
struct GlobalArgs {
    /// Session key for authentication
    #[arg(long, env = "BW_SESSION", global = true, hide_env_values = true)]
    session: Option<String>,

    /// Suppress all output
    #[arg(long, env = "BW_QUIET", global = true)]
    quiet: bool,

    /// Return raw JSON response
    #[arg(long, env = "BW_RESPONSE", global = true)]
    response: bool,

    /// Return raw output (no formatting)
    #[arg(long, env = "BW_RAW", global = true)]
    raw: bool,

    /// Pretty-print JSON output
    #[arg(long, env = "BW_PRETTY", global = true)]
    pretty: bool,

    /// Do not prompt for interactive input
    #[arg(long, env = "BW_NOINTERACTION", global = true)]
    nointeraction: bool,

    /// Always exit with code 0 (success)
    #[arg(long, env = "BW_CLEANEXIT", global = true)]
    cleanexit: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Authentication commands
    #[command(subcommand)]
    Login(AuthCommands),

    Logout(LogoutCommand),
    Lock(LockCommand),
    Unlock(UnlockCommand),

    /// Vault management commands
    #[command(subcommand)]
    List(ListCommands),

    #[command(subcommand)]
    Get(GetCommands),

    #[command(subcommand)]
    Create(CreateCommands),

    #[command(subcommand)]
    Edit(EditCommands),

    #[command(subcommand)]
    Delete(DeleteCommands),

    Restore(RestoreCommand),
    Move(MoveCommand),
    Confirm(ConfirmCommand),

    /// Sync vault with server
    Sync(SyncCommand),

    /// Utility commands
    Generate(GenerateCommand),
    Encode(EncodeCommand),
    Decrypt(DecryptCommand),
    Import(ImportCommand),
    Export(ExportCommand),

    /// Send commands
    #[command(subcommand)]
    Send(SendCommands),

    /// Configuration
    Config(ConfigCommand),

    /// Status
    Status(StatusCommand),
}

#[tokio::main]
async fn main() -> ExitCode {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn"))
        )
        .init();

    let cli = Cli::parse();

    // Execute command and format output
    let result = execute_command(cli.command, &cli.global_args).await;

    let exit_code = match result {
        Ok(response) => {
            output::print_response(response, &cli.global_args);
            ExitCode::SUCCESS
        }
        Err(e) => {
            if !cli.global_args.quiet {
                eprintln!("Error: {:#}", e);
            }
            if cli.global_args.cleanexit {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
    };

    exit_code
}

async fn execute_command(
    command: Commands,
    global_args: &GlobalArgs,
) -> anyhow::Result<output::Response> {
    use Commands::*;

    match command {
        Login(cmd) => commands::auth::execute_login(cmd, global_args).await,
        Logout(cmd) => commands::auth::execute_logout(cmd, global_args).await,
        Lock(cmd) => commands::auth::execute_lock(cmd, global_args).await,
        Unlock(cmd) => commands::auth::execute_unlock(cmd, global_args).await,
        List(cmd) => commands::vault::execute_list(cmd, global_args).await,
        Get(cmd) => commands::vault::execute_get(cmd, global_args).await,
        Create(cmd) => commands::vault::execute_create(cmd, global_args).await,
        Edit(cmd) => commands::vault::execute_edit(cmd, global_args).await,
        Delete(cmd) => commands::vault::execute_delete(cmd, global_args).await,
        Restore(cmd) => commands::vault::execute_restore(cmd, global_args).await,
        Move(cmd) => commands::vault::execute_move(cmd, global_args).await,
        Confirm(cmd) => commands::vault::execute_confirm(cmd, global_args).await,
        Sync(cmd) => commands::sync::execute_sync(cmd, global_args).await,
        Generate(cmd) => commands::tools::execute_generate(cmd, global_args).await,
        Encode(cmd) => commands::tools::execute_encode(cmd, global_args).await,
        Decrypt(cmd) => commands::tools::execute_decrypt(cmd, global_args).await,
        Import(cmd) => commands::tools::execute_import(cmd, global_args).await,
        Export(cmd) => commands::tools::execute_export(cmd, global_args).await,
        Send(cmd) => commands::send::execute_send(cmd, global_args).await,
        Config(cmd) => commands::config::execute_config(cmd, global_args).await,
        Status(cmd) => commands::status::execute_status(cmd, global_args).await,
    }
}

// Module declarations
mod commands;
mod output;
```

**Design Principles:**
1. **Type-safe parsing**: clap derive macros ensure compile-time validation
2. **Global args**: Flatten pattern makes flags available everywhere
3. **Environment variables**: Automatic fallback with `env` attribute
4. **Extensibility**: Easy to add new commands without touching routing logic
5. **Separation**: CLI parsing separate from command execution

### 2.2 Command Module Structure

**File:** `crates/bw-cli/src/commands/mod.rs`

```rust
pub mod auth;
pub mod vault;
pub mod sync;
pub mod tools;
pub mod send;
pub mod config;
pub mod status;

// Re-export command types
pub use auth::*;
pub use vault::*;
pub use sync::*;
pub use tools::*;
pub use send::*;
pub use config::*;
pub use status::*;
```

### 2.3 Auth Commands Example

**File:** `crates/bw-cli/src/commands/auth.rs`

```rust
use clap::{Args, Subcommand};
use crate::output::Response;
use crate::GlobalArgs;

#[derive(Subcommand)]
pub enum AuthCommands {
    /// Log in using email and master password
    Password(LoginPasswordCommand),

    /// Log in using API key
    ApiKey(LoginApiKeyCommand),

    /// Log in using SSO
    Sso(LoginSsoCommand),
}

#[derive(Args)]
pub struct LoginPasswordCommand {
    /// Email address
    #[arg(value_name = "EMAIL")]
    pub email: Option<String>,

    /// Master password
    #[arg(value_name = "PASSWORD")]
    pub password: Option<String>,

    /// Two-step login code
    #[arg(long)]
    pub code: Option<String>,

    /// Two-step login method
    #[arg(long)]
    pub method: Option<u8>,
}

#[derive(Args)]
pub struct LoginApiKeyCommand {
    /// Client ID
    #[arg(long)]
    pub client_id: Option<String>,

    /// Client secret
    #[arg(long)]
    pub client_secret: Option<String>,
}

#[derive(Args)]
pub struct LoginSsoCommand {
    /// Organization identifier
    #[arg(long)]
    pub org_identifier: Option<String>,
}

#[derive(Args)]
pub struct LogoutCommand;

#[derive(Args)]
pub struct LockCommand;

#[derive(Args)]
pub struct UnlockCommand {
    /// Master password
    #[arg(value_name = "PASSWORD")]
    pub password: Option<String>,
}

// Stub implementations
pub async fn execute_login(
    cmd: AuthCommands,
    _global_args: &GlobalArgs,
) -> anyhow::Result<Response> {
    Ok(Response::error("Not yet implemented"))
}

pub async fn execute_logout(
    _cmd: LogoutCommand,
    _global_args: &GlobalArgs,
) -> anyhow::Result<Response> {
    Ok(Response::error("Not yet implemented"))
}

pub async fn execute_lock(
    _cmd: LockCommand,
    _global_args: &GlobalArgs,
) -> anyhow::Result<Response> {
    Ok(Response::error("Not yet implemented"))
}

pub async fn execute_unlock(
    _cmd: UnlockCommand,
    _global_args: &GlobalArgs,
) -> anyhow::Result<Response> {
    Ok(Response::error("Not yet implemented"))
}
```

**Pattern Notes:**
- **Nested subcommands**: `login` has variants (password, apikey, sso)
- **Optional args**: Allows interactive prompting in future
- **Stub returns**: All return "Not yet implemented" Response
- **Async signatures**: Prepared for future SDK calls

### 2.4 Vault Commands Example

**File:** `crates/bw-cli/src/commands/vault.rs`

```rust
use clap::{Args, Subcommand};
use crate::output::Response;
use crate::GlobalArgs;

#[derive(Subcommand)]
pub enum ListCommands {
    /// List vault items
    Items(ListItemsCommand),

    /// List folders
    Folders(ListFoldersCommand),

    /// List collections
    Collections(ListCollectionsCommand),

    /// List organizations
    Organizations(ListOrganizationsCommand),

    /// List organization collections
    #[command(name = "org-collections")]
    OrgCollections(ListOrgCollectionsCommand),

    /// List organization members
    #[command(name = "org-members")]
    OrgMembers(ListOrgMembersCommand),
}

#[derive(Args)]
pub struct ListItemsCommand {
    /// Filter by organization ID
    #[arg(long)]
    pub organizationid: Option<String>,

    /// Filter by collection ID
    #[arg(long)]
    pub collectionid: Option<String>,

    /// Filter by folder ID
    #[arg(long)]
    pub folderid: Option<String>,

    /// Include trash items
    #[arg(long)]
    pub trash: bool,

    /// Search query
    #[arg(long)]
    pub search: Option<String>,

    /// Filter by URL
    #[arg(long)]
    pub url: Option<String>,
}

#[derive(Args)]
pub struct ListFoldersCommand {
    /// Search query
    #[arg(long)]
    pub search: Option<String>,
}

#[derive(Args)]
pub struct ListCollectionsCommand {
    /// Filter by organization ID
    #[arg(long)]
    pub organizationid: Option<String>,

    /// Search query
    #[arg(long)]
    pub search: Option<String>,
}

#[derive(Args)]
pub struct ListOrganizationsCommand;

#[derive(Args)]
pub struct ListOrgCollectionsCommand {
    /// Organization ID (required)
    #[arg(long, required = true)]
    pub organizationid: String,
}

#[derive(Args)]
pub struct ListOrgMembersCommand {
    /// Organization ID (required)
    #[arg(long, required = true)]
    pub organizationid: String,
}

#[derive(Subcommand)]
pub enum GetCommands {
    /// Get a vault item
    Item(GetItemCommand),

    /// Get username from an item
    Username(GetUsernameCommand),

    /// Get password from an item
    Password(GetPasswordCommand),

    /// Get URI from an item
    Uri(GetUriCommand),

    /// Get TOTP code
    Totp(GetTotpCommand),

    /// Check if password is exposed
    Exposed(GetExposedCommand),

    /// Download attachment
    Attachment(GetAttachmentCommand),

    /// Get folder
    Folder(GetFolderCommand),

    /// Get collection
    Collection(GetCollectionCommand),

    /// Get organization
    #[command(name = "org")]
    Organization(GetOrganizationCommand),

    /// Get item template
    Template(GetTemplateCommand),

    /// Get account fingerprint
    Fingerprint(GetFingerprintCommand),
}

#[derive(Args)]
pub struct GetItemCommand {
    /// Item ID or name
    #[arg(value_name = "ID")]
    pub id: String,
}

#[derive(Args)]
pub struct GetUsernameCommand {
    /// Item ID or name
    #[arg(value_name = "ID")]
    pub id: String,
}

#[derive(Args)]
pub struct GetPasswordCommand {
    /// Item ID or name
    #[arg(value_name = "ID")]
    pub id: String,
}

#[derive(Args)]
pub struct GetUriCommand {
    /// Item ID or name
    #[arg(value_name = "ID")]
    pub id: String,
}

#[derive(Args)]
pub struct GetTotpCommand {
    /// Item ID or name
    #[arg(value_name = "ID")]
    pub id: String,
}

#[derive(Args)]
pub struct GetExposedCommand {
    /// Item ID or name
    #[arg(value_name = "ID")]
    pub id: String,
}

#[derive(Args)]
pub struct GetAttachmentCommand {
    /// Attachment ID
    #[arg(value_name = "ID")]
    pub id: String,

    /// Item ID
    #[arg(long, required = true)]
    pub itemid: String,

    /// Output file path
    #[arg(long)]
    pub output: Option<String>,
}

#[derive(Args)]
pub struct GetFolderCommand {
    /// Folder ID or name
    #[arg(value_name = "ID")]
    pub id: String,
}

#[derive(Args)]
pub struct GetCollectionCommand {
    /// Collection ID or name
    #[arg(value_name = "ID")]
    pub id: String,
}

#[derive(Args)]
pub struct GetOrganizationCommand {
    /// Organization ID or name
    #[arg(value_name = "ID")]
    pub id: String,
}

#[derive(Args)]
pub struct GetTemplateCommand {
    /// Template type (item, folder, collection, etc.)
    #[arg(value_name = "TYPE")]
    pub template_type: String,
}

#[derive(Args)]
pub struct GetFingerprintCommand {
    /// Email address
    #[arg(value_name = "EMAIL")]
    pub email: String,
}

#[derive(Subcommand)]
pub enum CreateCommands {
    /// Create vault item
    Item(CreateItemCommand),

    /// Upload attachment
    Attachment(CreateAttachmentCommand),

    /// Create folder
    Folder(CreateFolderCommand),

    /// Create organization collection
    #[command(name = "org-collection")]
    OrgCollection(CreateOrgCollectionCommand),
}

#[derive(Args)]
pub struct CreateItemCommand {
    /// JSON encoded item
    #[arg(value_name = "JSON")]
    pub json: String,
}

#[derive(Args)]
pub struct CreateAttachmentCommand {
    /// File path to upload
    #[arg(long, required = true)]
    pub file: String,

    /// Item ID
    #[arg(long, required = true)]
    pub itemid: String,
}

#[derive(Args)]
pub struct CreateFolderCommand {
    /// JSON encoded folder
    #[arg(value_name = "JSON")]
    pub json: String,
}

#[derive(Args)]
pub struct CreateOrgCollectionCommand {
    /// JSON encoded collection
    #[arg(value_name = "JSON")]
    pub json: String,

    /// Organization ID
    #[arg(long, required = true)]
    pub organizationid: String,
}

#[derive(Subcommand)]
pub enum EditCommands {
    /// Edit vault item
    Item(EditItemCommand),

    /// Edit item collections
    #[command(name = "item-collections")]
    ItemCollections(EditItemCollectionsCommand),

    /// Edit folder
    Folder(EditFolderCommand),

    /// Edit organization collection
    #[command(name = "org-collection")]
    OrgCollection(EditOrgCollectionCommand),
}

#[derive(Args)]
pub struct EditItemCommand {
    /// Item ID
    #[arg(value_name = "ID")]
    pub id: String,

    /// JSON encoded item
    #[arg(value_name = "JSON")]
    pub json: String,
}

#[derive(Args)]
pub struct EditItemCollectionsCommand {
    /// Item ID
    #[arg(value_name = "ID")]
    pub id: String,

    /// Comma-separated collection IDs
    #[arg(value_name = "COLLECTION_IDS")]
    pub collection_ids: String,
}

#[derive(Args)]
pub struct EditFolderCommand {
    /// Folder ID
    #[arg(value_name = "ID")]
    pub id: String,

    /// JSON encoded folder
    #[arg(value_name = "JSON")]
    pub json: String,
}

#[derive(Args)]
pub struct EditOrgCollectionCommand {
    /// Collection ID
    #[arg(value_name = "ID")]
    pub id: String,

    /// JSON encoded collection
    #[arg(value_name = "JSON")]
    pub json: String,

    /// Organization ID
    #[arg(long, required = true)]
    pub organizationid: String,
}

#[derive(Subcommand)]
pub enum DeleteCommands {
    /// Delete (trash) vault item
    Item(DeleteItemCommand),

    /// Delete attachment
    Attachment(DeleteAttachmentCommand),

    /// Delete folder
    Folder(DeleteFolderCommand),

    /// Delete organization collection
    #[command(name = "org-collection")]
    OrgCollection(DeleteOrgCollectionCommand),
}

#[derive(Args)]
pub struct DeleteItemCommand {
    /// Item ID
    #[arg(value_name = "ID")]
    pub id: String,

    /// Permanently delete (skip trash)
    #[arg(long)]
    pub permanent: bool,
}

#[derive(Args)]
pub struct DeleteAttachmentCommand {
    /// Attachment ID
    #[arg(value_name = "ID")]
    pub id: String,

    /// Item ID
    #[arg(long, required = true)]
    pub itemid: String,
}

#[derive(Args)]
pub struct DeleteFolderCommand {
    /// Folder ID
    #[arg(value_name = "ID")]
    pub id: String,
}

#[derive(Args)]
pub struct DeleteOrgCollectionCommand {
    /// Collection ID
    #[arg(value_name = "ID")]
    pub id: String,

    /// Organization ID
    #[arg(long, required = true)]
    pub organizationid: String,
}

#[derive(Args)]
pub struct RestoreCommand {
    /// Item ID to restore
    #[arg(value_name = "ID")]
    pub id: String,
}

#[derive(Args)]
pub struct MoveCommand {
    /// Item ID to move
    #[arg(value_name = "ITEM_ID")]
    pub item_id: String,

    /// Destination folder ID
    #[arg(value_name = "FOLDER_ID")]
    pub folder_id: String,
}

#[derive(Args)]
pub struct ConfirmCommand {
    /// Member ID to confirm
    #[arg(value_name = "ID")]
    pub id: String,

    /// Organization ID
    #[arg(long, required = true)]
    pub organizationid: String,
}

// Stub implementations
pub async fn execute_list(
    _cmd: ListCommands,
    _global_args: &GlobalArgs,
) -> anyhow::Result<Response> {
    Ok(Response::error("Not yet implemented"))
}

pub async fn execute_get(
    _cmd: GetCommands,
    _global_args: &GlobalArgs,
) -> anyhow::Result<Response> {
    Ok(Response::error("Not yet implemented"))
}

pub async fn execute_create(
    _cmd: CreateCommands,
    _global_args: &GlobalArgs,
) -> anyhow::Result<Response> {
    Ok(Response::error("Not yet implemented"))
}

pub async fn execute_edit(
    _cmd: EditCommands,
    _global_args: &GlobalArgs,
) -> anyhow::Result<Response> {
    Ok(Response::error("Not yet implemented"))
}

pub async fn execute_delete(
    _cmd: DeleteCommands,
    _global_args: &GlobalArgs,
) -> anyhow::Result<Response> {
    Ok(Response::error("Not yet implemented"))
}

pub async fn execute_restore(
    _cmd: RestoreCommand,
    _global_args: &GlobalArgs,
) -> anyhow::Result<Response> {
    Ok(Response::error("Not yet implemented"))
}

pub async fn execute_move(
    _cmd: MoveCommand,
    _global_args: &GlobalArgs,
) -> anyhow::Result<Response> {
    Ok(Response::error("Not yet implemented"))
}

pub async fn execute_confirm(
    _cmd: ConfirmCommand,
    _global_args: &GlobalArgs,
) -> anyhow::Result<Response> {
    Ok(Response::error("Not yet implemented"))
}
```

**Pattern Notes:**
- **Comprehensive coverage**: All vault commands from requirements
- **Type-safe arguments**: clap validates required vs optional
- **Consistent naming**: Matches TypeScript CLI exactly
- **Future-ready**: Structure supports real implementations

## 3. Response Type & Output Formatting

### 3.1 Response Type Design

**File:** `crates/bw-cli/src/output/mod.rs`

```rust
use serde::{Deserialize, Serialize};
use serde_json::Value;

mod formatter;
pub use formatter::print_response;

/// Response types matching TypeScript CLI Response class
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Response {
    Success(SuccessResponse),
    Error(ErrorResponse),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub success: bool,
    pub message: String,
}

impl Response {
    /// Create a success response with data
    pub fn success(data: impl Serialize) -> Self {
        Response::Success(SuccessResponse {
            success: true,
            data: serde_json::to_value(data).ok(),
            message: None,
        })
    }

    /// Create a success response with just a message
    pub fn success_message(message: impl Into<String>) -> Self {
        Response::Success(SuccessResponse {
            success: true,
            data: None,
            message: Some(message.into()),
        })
    }

    /// Create an error response
    pub fn error(message: impl Into<String>) -> Self {
        Response::Error(ErrorResponse {
            success: false,
            message: message.into(),
        })
    }

    /// Check if this is a success response
    pub fn is_success(&self) -> bool {
        matches!(self, Response::Success(_))
    }

    /// Extract data as a specific type
    pub fn data<T: for<'de> Deserialize<'de>>(&self) -> Option<T> {
        match self {
            Response::Success(s) => s.data.as_ref().and_then(|v| {
                serde_json::from_value(v.clone()).ok()
            }),
            Response::Error(_) => None,
        }
    }
}
```

**Design Principles:**
1. **TypeScript compatibility**: Matches Response class structure
2. **Type safety**: Strongly typed success/error variants
3. **Serialization**: Direct JSON output support
4. **Builder methods**: Ergonomic API for creating responses
5. **Extensibility**: Easy to add more response types

### 3.2 Output Formatter

**File:** `crates/bw-cli/src/output/formatter.rs`

```rust
use super::Response;
use crate::GlobalArgs;
use serde_json::Value;

/// Print response according to global args (--response, --pretty, --quiet, --raw)
pub fn print_response(response: Response, args: &GlobalArgs) {
    // Quiet mode: suppress all output
    if args.quiet {
        return;
    }

    // Response mode: JSON output
    if args.response {
        print_json(&response, args.pretty);
        return;
    }

    // Raw mode: minimal output
    if args.raw {
        print_raw(&response);
        return;
    }

    // Default: human-readable output
    print_human(&response);
}

fn print_json(response: &Response, pretty: bool) {
    if pretty {
        match serde_json::to_string_pretty(response) {
            Ok(json) => println!("{}", json),
            Err(e) => eprintln!("Error formatting response: {}", e),
        }
    } else {
        match serde_json::to_string(response) {
            Ok(json) => println!("{}", json),
            Err(e) => eprintln!("Error formatting response: {}", e),
        }
    }
}

fn print_raw(response: &Response) {
    match response {
        Response::Success(s) => {
            if let Some(data) = &s.data {
                print_raw_value(data);
            } else if let Some(msg) = &s.message {
                println!("{}", msg);
            }
        }
        Response::Error(e) => {
            eprintln!("{}", e.message);
        }
    }
}

fn print_raw_value(value: &Value) {
    match value {
        Value::String(s) => println!("{}", s),
        Value::Number(n) => println!("{}", n),
        Value::Bool(b) => println!("{}", b),
        Value::Null => println!("null"),
        Value::Array(arr) => {
            for item in arr {
                print_raw_value(item);
            }
        }
        Value::Object(_) => {
            // For objects, print compact JSON
            if let Ok(json) = serde_json::to_string(value) {
                println!("{}", json);
            }
        }
    }
}

fn print_human(response: &Response) {
    match response {
        Response::Success(s) => {
            if let Some(data) = &s.data {
                // Pretty-print data by default in human mode
                match serde_json::to_string_pretty(data) {
                    Ok(json) => println!("{}", json),
                    Err(e) => eprintln!("Error formatting response: {}", e),
                }
            } else if let Some(msg) = &s.message {
                println!("{}", msg);
            } else {
                println!("Success");
            }
        }
        Response::Error(e) => {
            eprintln!("Error: {}", e.message);
        }
    }
}
```

**Design Principles:**
1. **Mode routing**: Clear logic for each output mode
2. **Quiet first**: Early return for quiet mode
3. **JSON formatting**: Both compact and pretty variants
4. **Raw extraction**: Smart extraction of primitive values
5. **Human-friendly**: Default mode is readable

### 3.3 Remaining Command Stubs

The following command modules need similar implementations:

**File:** `crates/bw-cli/src/commands/sync.rs`

```rust
use clap::Args;
use crate::output::Response;
use crate::GlobalArgs;

#[derive(Args)]
pub struct SyncCommand {
    /// Force full sync
    #[arg(long)]
    pub force: bool,

    /// Sync only this session (no server communication)
    #[arg(long)]
    pub last: bool,
}

pub async fn execute_sync(
    _cmd: SyncCommand,
    _global_args: &GlobalArgs,
) -> anyhow::Result<Response> {
    Ok(Response::error("Not yet implemented"))
}
```

**File:** `crates/bw-cli/src/commands/tools.rs`

```rust
use clap::Args;
use crate::output::Response;
use crate::GlobalArgs;

#[derive(Args)]
pub struct GenerateCommand {
    /// Generate a passphrase instead of password
    #[arg(long)]
    pub passphrase: bool,

    /// Password length (default: 14)
    #[arg(long)]
    pub length: Option<usize>,

    /// Minimum number of lowercase characters
    #[arg(long)]
    pub lowercase: Option<usize>,

    /// Minimum number of uppercase characters
    #[arg(long)]
    pub uppercase: Option<usize>,

    /// Minimum number of numeric characters
    #[arg(long)]
    pub number: Option<usize>,

    /// Minimum number of special characters
    #[arg(long)]
    pub special: Option<usize>,

    /// Number of passphrase words (default: 3)
    #[arg(long)]
    pub words: Option<usize>,

    /// Passphrase word separator (default: -)
    #[arg(long)]
    pub separator: Option<String>,

    /// Capitalize passphrase words
    #[arg(long)]
    pub capitalize: bool,

    /// Include number in passphrase
    #[arg(long, alias = "includeNumber")]
    pub include_number: bool,
}

#[derive(Args)]
pub struct EncodeCommand {
    /// Data to encode
    #[arg(value_name = "DATA")]
    pub data: String,
}

#[derive(Args)]
pub struct DecryptCommand {
    /// Encrypted string to decrypt
    #[arg(value_name = "ENCRYPTED")]
    pub encrypted: String,

    /// Organization ID (for org-encrypted data)
    #[arg(long)]
    pub organizationid: Option<String>,
}

#[derive(Args)]
pub struct ImportCommand {
    /// Import format (bitwardenjson, lastpass, etc.)
    #[arg(value_name = "FORMAT")]
    pub format: String,

    /// Input file path
    #[arg(value_name = "FILE")]
    pub file: String,

    /// Organization ID (import to org)
    #[arg(long)]
    pub organizationid: Option<String>,
}

#[derive(Args)]
pub struct ExportCommand {
    /// Export format (json, csv, encrypted_json)
    #[arg(long)]
    pub format: Option<String>,

    /// Master password (required for encrypted export)
    #[arg(long)]
    pub password: Option<String>,

    /// Organization ID (export org vault)
    #[arg(long)]
    pub organizationid: Option<String>,

    /// Output file path
    #[arg(long)]
    pub output: Option<String>,
}

pub async fn execute_generate(
    _cmd: GenerateCommand,
    _global_args: &GlobalArgs,
) -> anyhow::Result<Response> {
    Ok(Response::error("Not yet implemented"))
}

pub async fn execute_encode(
    _cmd: EncodeCommand,
    _global_args: &GlobalArgs,
) -> anyhow::Result<Response> {
    Ok(Response::error("Not yet implemented"))
}

pub async fn execute_decrypt(
    _cmd: DecryptCommand,
    _global_args: &GlobalArgs,
) -> anyhow::Result<Response> {
    Ok(Response::error("Not yet implemented"))
}

pub async fn execute_import(
    _cmd: ImportCommand,
    _global_args: &GlobalArgs,
) -> anyhow::Result<Response> {
    Ok(Response::error("Not yet implemented"))
}

pub async fn execute_export(
    _cmd: ExportCommand,
    _global_args: &GlobalArgs,
) -> anyhow::Result<Response> {
    Ok(Response::error("Not yet implemented"))
}
```

**File:** `crates/bw-cli/src/commands/send.rs`

```rust
use clap::{Args, Subcommand};
use crate::output::Response;
use crate::GlobalArgs;

#[derive(Subcommand)]
pub enum SendCommands {
    /// List all Sends
    List(SendListCommand),

    /// Get Send template
    Template(SendTemplateCommand),

    /// Get Send by ID
    Get(SendGetCommand),

    /// Create a new Send
    Create(SendCreateCommand),

    /// Edit existing Send
    Edit(SendEditCommand),

    /// Remove password from Send
    #[command(name = "remove-password")]
    RemovePassword(SendRemovePasswordCommand),

    /// Delete Send
    Delete(SendDeleteCommand),
}

#[derive(Args)]
pub struct SendListCommand;

#[derive(Args)]
pub struct SendTemplateCommand {
    /// Template type (file or text)
    #[arg(value_name = "TYPE")]
    pub send_type: Option<String>,
}

#[derive(Args)]
pub struct SendGetCommand {
    /// Send ID
    #[arg(value_name = "ID")]
    pub id: String,
}

#[derive(Args)]
pub struct SendCreateCommand {
    /// JSON encoded Send
    #[arg(value_name = "JSON")]
    pub json: String,

    /// File path (for file sends)
    #[arg(long)]
    pub file: Option<String>,

    /// Send text content
    #[arg(long)]
    pub text: Option<String>,

    /// Hidden text (for text sends)
    #[arg(long)]
    pub hidden: bool,
}

#[derive(Args)]
pub struct SendEditCommand {
    /// Send ID
    #[arg(value_name = "ID")]
    pub id: String,

    /// JSON encoded Send
    #[arg(value_name = "JSON")]
    pub json: String,
}

#[derive(Args)]
pub struct SendRemovePasswordCommand {
    /// Send ID
    #[arg(value_name = "ID")]
    pub id: String,
}

#[derive(Args)]
pub struct SendDeleteCommand {
    /// Send ID
    #[arg(value_name = "ID")]
    pub id: String,
}

pub async fn execute_send(
    _cmd: SendCommands,
    _global_args: &GlobalArgs,
) -> anyhow::Result<Response> {
    Ok(Response::error("Not yet implemented"))
}
```

**File:** `crates/bw-cli/src/commands/config.rs`

```rust
use clap::{Args, Subcommand};
use crate::output::Response;
use crate::GlobalArgs;

#[derive(Args)]
pub struct ConfigCommand {
    #[command(subcommand)]
    pub subcommand: ConfigSubcommand,
}

#[derive(Subcommand)]
pub enum ConfigSubcommand {
    /// Set server URL
    Server(ConfigServerCommand),
}

#[derive(Args)]
pub struct ConfigServerCommand {
    /// Server URL
    #[arg(value_name = "URL")]
    pub url: String,
}

pub async fn execute_config(
    _cmd: ConfigCommand,
    _global_args: &GlobalArgs,
) -> anyhow::Result<Response> {
    Ok(Response::error("Not yet implemented"))
}
```

**File:** `crates/bw-cli/src/commands/status.rs`

```rust
use clap::Args;
use crate::output::Response;
use crate::GlobalArgs;

#[derive(Args)]
pub struct StatusCommand;

pub async fn execute_status(
    _cmd: StatusCommand,
    _global_args: &GlobalArgs,
) -> anyhow::Result<Response> {
    Ok(Response::error("Not yet implemented"))
}
```

## 4. Service Container & SDK Integration

### 4.1 Service Container Design

**File:** `crates/bw-core/src/lib.rs`

```rust
pub mod services;
pub mod models;

// Re-export commonly used types
pub use services::ServiceContainer;
```

**File:** `crates/bw-core/src/services/mod.rs`

```rust
mod sdk;
mod container;

pub use sdk::create_sdk_client;
pub use container::ServiceContainer;
```

**File:** `crates/bw-core/src/services/sdk.rs`

```rust
use bitwarden_core::{Client, ClientSettings, DeviceType};
use anyhow::Result;

/// Create the SDK client for all crypto and vault operations
///
/// # Arguments
/// * `api_url` - Optional API server URL (default: https://api.bitwarden.com)
/// * `identity_url` - Optional Identity server URL (default: https://identity.bitwarden.com)
///
/// # Returns
/// Configured SDK client ready for authentication and vault operations
pub fn create_sdk_client(
    api_url: Option<String>,
    identity_url: Option<String>,
) -> Result<Client> {
    let settings = ClientSettings {
        identity_url: identity_url.unwrap_or_else(|| {
            "https://identity.bitwarden.com".to_string()
        }),
        api_url: api_url.unwrap_or_else(|| {
            "https://api.bitwarden.com".to_string()
        }),
        user_agent: format!("Bitwarden_CLI/{}", env!("CARGO_PKG_VERSION")),
        device_type: DeviceType::CLI,
    };

    Client::new(Some(settings))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_sdk_client_defaults() {
        let client = create_sdk_client(None, None);
        assert!(client.is_ok(), "Should create client with default URLs");
    }

    #[test]
    fn test_create_sdk_client_custom_urls() {
        let client = create_sdk_client(
            Some("https://api.example.com".to_string()),
            Some("https://identity.example.com".to_string()),
        );
        assert!(client.is_ok(), "Should create client with custom URLs");
    }
}
```

**File:** `crates/bw-core/src/services/container.rs`

```rust
use bitwarden_core::Client;
use anyhow::Result;
use super::create_sdk_client;

/// Service container for dependency injection
///
/// Provides access to:
/// - SDK client (crypto, vault, auth operations)
/// - Storage (future: enhancement 2)
/// - HTTP client (future: enhancement 3)
pub struct ServiceContainer {
    /// Bitwarden SDK client - handles all crypto and most business logic
    sdk: Client,
}

impl ServiceContainer {
    /// Create a new service container
    ///
    /// # Arguments
    /// * `api_url` - Optional API server URL
    /// * `identity_url` - Optional Identity server URL
    pub fn new(
        api_url: Option<String>,
        identity_url: Option<String>,
    ) -> Result<Self> {
        let sdk = create_sdk_client(api_url, identity_url)?;

        Ok(Self { sdk })
    }

    /// Get reference to SDK client
    ///
    /// Use this for all crypto operations (encrypt, decrypt, key derivation)
    /// and vault operations (sync, cipher operations, etc.)
    pub fn sdk(&self) -> &Client {
        &self.sdk
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_container_creation() {
        let container = ServiceContainer::new(None, None);
        assert!(container.is_ok(), "Should create service container");
    }
}
```

**File:** `crates/bw-core/src/models/mod.rs`

```rust
// Placeholder for future domain models
// Enhancement 2+ will add:
// - Session data structures
// - Cached vault data
// - Configuration models
```

**Integration Notes:**
1. **Service container**: Centralized access point for all services
2. **SDK client**: Initialized once, reused across commands
3. **Future-proof**: Structure supports adding storage, HTTP client later
4. **Thread-safe**: Can be wrapped in Arc for concurrent access
5. **Testable**: Mock-friendly interface

## 5. Module Organization & File Structure

### 5.1 Complete Directory Tree

```
bwcli-rs/
├── Cargo.toml                          # Workspace manifest
├── Cargo.lock                          # Dependency lock file
├── .gitignore                          # Git ignore patterns
├── README.md                           # Build instructions, usage
├── LICENSE                             # GPL-3.0 license
│
├── crates/
│   ├── bw-cli/                         # Binary crate
│   │   ├── Cargo.toml                  # Binary dependencies
│   │   ├── src/
│   │   │   ├── main.rs                 # Entry point, CLI parsing
│   │   │   ├── commands/
│   │   │   │   ├── mod.rs              # Command module exports
│   │   │   │   ├── auth.rs             # Auth commands
│   │   │   │   ├── vault.rs            # Vault commands
│   │   │   │   ├── sync.rs             # Sync command
│   │   │   │   ├── tools.rs            # Tools commands
│   │   │   │   ├── send.rs             # Send commands
│   │   │   │   ├── config.rs           # Config command
│   │   │   │   └── status.rs           # Status command
│   │   │   └── output/
│   │   │       ├── mod.rs              # Response type
│   │   │       └── formatter.rs        # Output formatting
│   │   │
│   │   └── tests/
│   │       ├── integration_test.rs     # Integration tests
│   │       └── help_text_test.rs       # Help text validation
│   │
│   └── bw-core/                        # Library crate
│       ├── Cargo.toml                  # Library dependencies
│       ├── src/
│       │   ├── lib.rs                  # Library exports
│       │   ├── services/
│       │   │   ├── mod.rs              # Service exports
│       │   │   ├── container.rs        # Service container
│       │   │   └── sdk.rs              # SDK initialization
│       │   └── models/
│       │       └── mod.rs              # Domain models (placeholder)
│       │
│       └── tests/
│           └── sdk_integration_test.rs # SDK integration tests
│
├── .cargo/
│   └── config.toml                     # Cargo configuration (optional)
│
└── enhancements/                       # Enhancement tracking
    └── 01-project-bootstrap/
        └── ...
```

### 5.2 File Responsibilities

| File | Purpose | Key Contents |
|------|---------|--------------|
| `main.rs` | Entry point | CLI parsing, command routing, output |
| `commands/auth.rs` | Auth commands | login, logout, lock, unlock, status |
| `commands/vault.rs` | Vault commands | list, get, create, edit, delete |
| `commands/sync.rs` | Sync command | sync implementation stub |
| `commands/tools.rs` | Tools | generate, encode, decrypt, import, export |
| `commands/send.rs` | Send commands | send operations |
| `commands/config.rs` | Config | server configuration |
| `commands/status.rs` | Status | vault status check |
| `output/mod.rs` | Response type | Response enum, builder methods |
| `output/formatter.rs` | Formatting | JSON, pretty, raw, quiet modes |
| `services/container.rs` | DI container | ServiceContainer |
| `services/sdk.rs` | SDK init | create_sdk_client function |
| `models/mod.rs` | Domain models | Placeholder for future |

### 5.3 Supporting Files

**File:** `.gitignore`

```gitignore
# Rust
/target
Cargo.lock (if library)
**/*.rs.bk
*.pdb

# IDE
.vscode/
.idea/
*.swp
*.swo
*~

# OS
.DS_Store
Thumbs.db

# Build artifacts
*.exe
*.dll
*.so
*.dylib

# Logs
*.log

# Environment
.env
.env.local
```

**File:** `README.md`

```markdown
# Bitwarden CLI (Rust)

A secure and free password manager for all of your devices.

This is the Rust implementation of the Bitwarden CLI, providing improved performance and reduced binary size compared to the TypeScript version.

## Prerequisites

- Rust 1.70.0 or later
- Bitwarden SDK (cloned at `../sdk/`)

## Building

\`\`\`bash
# Build debug binary
cargo build

# Build optimized release binary
cargo build --release

# Run tests
cargo test

# Run clippy
cargo clippy --all-targets --all-features

# Format code
cargo fmt
\`\`\`

## Installation

\`\`\`bash
# Install from local build
cargo install --path crates/bw-cli

# Or copy binary directly
cp target/release/bw /usr/local/bin/
\`\`\`

## Usage

\`\`\`bash
# Show help
bw --help

# Show version
bw --version

# Login (stub - not yet implemented)
bw login

# Check status
bw status --response
\`\`\`

## Global Flags

- `--session <KEY>` - Session authentication key (env: BW_SESSION)
- `--quiet` - Suppress all output (env: BW_QUIET)
- `--response` - Return JSON formatted response (env: BW_RESPONSE)
- `--raw` - Return raw output (env: BW_RAW)
- `--pretty` - Format JSON with indentation (env: BW_PRETTY)
- `--nointeraction` - Disable interactive prompts (env: BW_NOINTERACTION)
- `--cleanexit` - Exit with code 0 even on errors (env: BW_CLEANEXIT)

## Development Status

This project is in early development. Currently implemented:
- ✅ Project structure and build configuration
- ✅ CLI parsing with all commands
- ✅ Global flags and environment variables
- ✅ Response formatting system
- ✅ SDK integration setup
- 🚧 Command implementations (stubs only)

All commands currently return "Not yet implemented". See the [enhancement plan](enhancements/) for implementation roadmap.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

GPL-3.0 - see [LICENSE](LICENSE) for details.
\`\`\`

**File:** `LICENSE`

```
GPL-3.0 License text
(Copy from existing Bitwarden CLI repository)
```

## 6. Testing Strategy

### 6.1 Unit Tests

**File:** `crates/bw-cli/tests/integration_test.rs`

```rust
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_cli_help() {
    let mut cmd = Command::cargo_bin("bw").unwrap();
    cmd.arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Bitwarden CLI"))
        .stdout(predicate::str::contains("Usage:"));
}

#[test]
fn test_cli_version() {
    let mut cmd = Command::cargo_bin("bw").unwrap();
    cmd.arg("--version");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn test_status_response_format() {
    let mut cmd = Command::cargo_bin("bw").unwrap();
    cmd.args(&["status", "--response"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(r#"{"success":false"#))
        .stdout(predicate::str::contains("Not yet implemented"));
}

#[test]
fn test_quiet_flag() {
    let mut cmd = Command::cargo_bin("bw").unwrap();
    cmd.args(&["status", "--quiet"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::is_empty());
}

#[test]
fn test_pretty_flag() {
    let mut cmd = Command::cargo_bin("bw").unwrap();
    cmd.args(&["status", "--response", "--pretty"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("  \"success\": false"));
}

#[test]
fn test_env_var_session() {
    let mut cmd = Command::cargo_bin("bw").unwrap();
    cmd.env("BW_SESSION", "test_session_key")
       .args(&["status", "--response"]);

    // Should accept session from env var without error
    cmd.assert().success();
}

#[test]
fn test_env_var_quiet() {
    let mut cmd = Command::cargo_bin("bw").unwrap();
    cmd.env("BW_QUIET", "true")
       .arg("status");

    cmd.assert()
        .success()
        .stdout(predicate::str::is_empty());
}

#[test]
fn test_all_auth_commands_exist() {
    for cmd_name in &["login", "logout", "lock", "unlock"] {
        let mut cmd = Command::cargo_bin("bw").unwrap();
        cmd.args(&[cmd_name, "--help"]);
        cmd.assert().success();
    }
}

#[test]
fn test_all_vault_commands_exist() {
    for cmd_name in &["list", "get", "create", "edit", "delete", "restore", "move", "confirm"] {
        let mut cmd = Command::cargo_bin("bw").unwrap();
        cmd.args(&[cmd_name, "--help"]);
        cmd.assert().success();
    }
}

#[test]
fn test_invalid_command() {
    let mut cmd = Command::cargo_bin("bw").unwrap();
    cmd.arg("nonexistent");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("unrecognized subcommand"));
}
```

**File:** `crates/bw-core/tests/sdk_integration_test.rs`

```rust
use bw_core::services::create_sdk_client;

#[test]
fn test_sdk_client_creation() {
    let result = create_sdk_client(None, None);
    assert!(result.is_ok(), "Should create SDK client with defaults");
}

#[test]
fn test_sdk_client_custom_urls() {
    let result = create_sdk_client(
        Some("https://api.example.com".to_string()),
        Some("https://identity.example.com".to_string()),
    );
    assert!(result.is_ok(), "Should create SDK client with custom URLs");
}

#[tokio::test]
async fn test_sdk_client_basic_usage() {
    let client = create_sdk_client(None, None).expect("Failed to create client");

    // Verify client is initialized (basic smoke test)
    // More comprehensive tests will come in enhancement 4 (auth)
    assert!(std::ptr::addr_of!(client) as usize != 0);
}
```

### 6.2 Test Organization

| Test Type | Location | Purpose |
|-----------|----------|---------|
| Integration | `bw-cli/tests/` | End-to-end CLI testing |
| Unit | `bw-core/tests/` | Service layer testing |
| Doc tests | Inline in code | Example verification |

### 6.3 Test Execution

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_cli_help

# Run with output
cargo test -- --nocapture

# Run integration tests only
cargo test --test integration_test

# Generate coverage report (requires cargo-tarpaulin)
cargo tarpaulin --out Html
```

## 7. Build & Verification

### 7.1 Build Process

```bash
# Format code
cargo fmt

# Run clippy
cargo clippy --all-targets --all-features -- -D warnings

# Build debug
cargo build

# Build release
cargo build --release

# Check binary size
ls -lh target/release/bw
strip target/release/bw
ls -lh target/release/bw

# Run tests
cargo test

# Generate docs
cargo doc --no-deps --open
```

### 7.2 Verification Checklist

Before marking implementation complete:

- [ ] `cargo fmt --check` passes
- [ ] `cargo clippy --all-features --all-targets` produces 0 warnings
- [ ] `cargo build --release` succeeds
- [ ] `cargo test` passes all tests
- [ ] `./target/release/bw --help` displays all commands
- [ ] `./target/release/bw --version` shows version
- [ ] `./target/release/bw status --response` returns JSON stub
- [ ] Binary size < 5MB when stripped
- [ ] Cross-platform build tested (macOS, Linux, Windows)

### 7.3 Cross-Platform Considerations

**macOS:**
```bash
cargo build --release --target x86_64-apple-darwin
cargo build --release --target aarch64-apple-darwin
```

**Linux:**
```bash
cargo build --release --target x86_64-unknown-linux-gnu
cargo build --release --target x86_64-unknown-linux-musl  # static binary
```

**Windows:**
```bash
cargo build --release --target x86_64-pc-windows-msvc
cargo build --release --target x86_64-pc-windows-gnu
```

## 8. Migration Notes

### 8.1 TypeScript CLI Reference Mapping

| TypeScript File | Rust Equivalent | Notes |
|-----------------|-----------------|-------|
| `apps/cli/src/bw.ts` | `crates/bw-cli/src/main.rs` | Entry point |
| `apps/cli/src/program.ts` | `crates/bw-cli/src/main.rs` | Command registration |
| `apps/cli/src/base-program.ts` | `crates/bw-cli/src/output/formatter.rs` | processResponse logic |
| `apps/cli/src/models/response.ts` | `crates/bw-cli/src/output/mod.rs` | Response class |
| `apps/cli/src/commands/login.command.ts` | `crates/bw-cli/src/commands/auth.rs` | Login variants |
| `apps/cli/src/commands/list.command.ts` | `crates/bw-cli/src/commands/vault.rs` | List operations |

### 8.2 Commander.js to clap Migration

**TypeScript (Commander.js):**
```typescript
program
  .command('login [email] [password]')
  .option('--apikey', 'Use API key')
  .option('--sso', 'Use SSO')
  .action(async (email, password, options) => {
    // implementation
  });
```

**Rust (clap):**
```rust
#[derive(Subcommand)]
enum AuthCommands {
    Password(LoginPasswordCommand),
    ApiKey(LoginApiKeyCommand),
    Sso(LoginSsoCommand),
}

#[derive(Args)]
struct LoginPasswordCommand {
    email: Option<String>,
    password: Option<String>,
}
```

**Key Differences:**
- clap uses types instead of dynamic parsing
- Variants replace boolean flags for command modes
- Optional args explicit in type system

### 8.3 Response Class to Response Enum

**TypeScript:**
```typescript
class Response {
  success: boolean;
  data?: any;
  message?: string;
}
```

**Rust:**
```rust
enum Response {
    Success(SuccessResponse),
    Error(ErrorResponse),
}

struct SuccessResponse {
    success: bool,
    data: Option<Value>,
    message: Option<String>,
}
```

**Benefits:**
- Type safety: Success always has success=true
- No invalid states: Can't have success=true with error message
- Pattern matching: Compiler enforces handling both cases

## 9. Architectural Decision Records

### ADR-001: Use Workspace Structure

**Context:** Need to organize code for modularity and future growth.

**Decision:** Use Cargo workspace with separate binary and library crates.

**Rationale:**
- Clear separation between CLI (presentation) and core (business logic)
- Library can be reused by other interfaces (GUI, API) in future
- Easier testing of business logic without CLI
- Follows Rust best practices

**Consequences:**
- Slightly more complex initial setup
- Better long-term maintainability
- Easier to add features in future enhancements

---

### ADR-002: Use clap derive macros

**Context:** Need CLI parsing framework.

**Decision:** Use clap v4 with derive macros instead of builder API.

**Rationale:**
- Type-safe: Compile-time validation of CLI structure
- Less boilerplate: Derive macros reduce code
- Better IDE support: Types are explicit
- Help text generation: Automatic from types
- Environment variables: Built-in support

**Consequences:**
- Learning curve for clap derive syntax
- Better type safety and maintainability
- Easier to add new commands

---

### ADR-003: Use anyhow for error handling

**Context:** Need error handling strategy for bootstrap phase.

**Decision:** Use `anyhow::Error` instead of custom error types.

**Rationale:**
- Rapid development: Less boilerplate
- Sufficient for stubs: Commands don't have complex error handling yet
- Easy migration: Can add custom errors in enhancement 4
- Context: anyhow provides good error context

**Consequences:**
- Less type safety: Can't match on specific error types
- Good enough for now: Stubs have minimal error handling
- Defer decision: Will revisit in enhancement 4 (auth)

---

### ADR-004: Response type in bw-cli crate

**Context:** Where to place Response formatting logic?

**Decision:** Place in `bw-cli` crate instead of `bw-core`.

**Rationale:**
- CLI-specific: Response formatting is presentation logic
- No other interfaces: No current need for reuse
- Easy refactoring: Can move to core if needed later
- YAGNI: Don't over-engineer for hypothetical requirements

**Consequences:**
- Simpler initial structure
- Can refactor later if needed
- Keeps presentation separate from business logic

---

### ADR-005: Service container pattern

**Context:** How to provide SDK client to commands?

**Decision:** Use service container for dependency injection.

**Rationale:**
- Testability: Easy to mock services
- Extensibility: Can add storage, HTTP client in future
- Single initialization: SDK client created once
- Thread-safe: Can wrap in Arc for concurrent access

**Consequences:**
- Slightly more complex than global singleton
- Better testability and extensibility
- Standard pattern in Rust applications

---

### ADR-006: Logging with tracing

**Context:** Which logging framework to use?

**Decision:** Use `tracing` instead of `env_logger`.

**Rationale:**
- SDK compatibility: SDK likely uses tracing
- Structured logging: Better debugging for crypto operations
- Industry standard: Widely adopted in Rust ecosystem
- Minimal overhead: Can disable in release builds

**Consequences:**
- Slightly more complex than env_logger
- Better long-term debugging capabilities
- Consistent with SDK

## 10. Implementation Phases

### Phase 1: Project Foundation
**Estimated Effort:** 2-3 hours

**Tasks:**
1. Create workspace Cargo.toml
2. Create bw-cli crate structure
3. Create bw-core crate structure
4. Add all dependencies
5. Create .gitignore and README.md
6. Verify `cargo build` succeeds

**Success Criteria:**
- `cargo build` completes without errors
- Directory structure matches specification
- All dependencies resolve correctly

---

### Phase 2: CLI Parsing Framework
**Estimated Effort:** 4-5 hours

**Tasks:**
1. Implement main.rs with clap CLI structure
2. Define GlobalArgs struct
3. Define Commands enum
4. Implement auth commands structure
5. Implement vault commands structure
6. Implement remaining commands
7. Test help text generation

**Success Criteria:**
- `bw --help` displays all commands
- All global flags recognized
- Command-specific help works
- Environment variables respected

---

### Phase 3: Response Formatting System
**Estimated Effort:** 2-3 hours

**Tasks:**
1. Define Response enum in output/mod.rs
2. Implement builder methods (success, error)
3. Implement formatter.rs with all modes
4. Test JSON, pretty, raw, quiet modes
5. Add unit tests for formatting

**Success Criteria:**
- All output modes work correctly
- JSON serialization correct
- Quiet mode suppresses output
- Pretty mode formats correctly

---

### Phase 4: Command Stubs Implementation
**Estimated Effort:** 3-4 hours

**Tasks:**
1. Implement all command stub functions
2. Wire up command router in main.rs
3. Ensure all commands return "Not yet implemented"
4. Add integration tests for each command
5. Verify help text for each command

**Success Criteria:**
- All commands compile
- All commands return stub responses
- Help text accurate
- No compilation warnings

---

### Phase 5: SDK Integration Setup
**Estimated Effort:** 2-3 hours

**Tasks:**
1. Implement create_sdk_client function
2. Implement ServiceContainer
3. Add SDK integration tests
4. Document SDK usage patterns
5. Verify SDK dependencies resolve

**Success Criteria:**
- SDK client initializes successfully
- ServiceContainer accessible
- Integration tests pass
- Documentation complete

---

### Phase 6: Testing & Polish
**Estimated Effort:** 2-3 hours

**Tasks:**
1. Add comprehensive integration tests
2. Test cross-platform compilation
3. Run clippy and fix warnings
4. Format code with cargo fmt
5. Measure and optimize binary size
6. Review and update documentation

**Success Criteria:**
- All tests pass
- Zero clippy warnings
- Binary size < 5MB stripped
- Documentation complete
- Cross-platform builds successful

---

### Total Estimated Effort: 15-21 hours

**Phases can run in parallel:**
- Phase 3 can start after Phase 1
- Phase 5 can start after Phase 1
- Phase 4 requires Phase 2 completion

## 11. Handoff to Implementer

### 11.1 Implementation Order

1. **Start with Phase 1** - Get the project structure in place
2. **Then Phase 2** - Implement CLI parsing (critical path)
3. **Parallel: Phase 3 & 5** - Response formatting and SDK setup
4. **Then Phase 4** - Wire up command stubs
5. **Finally Phase 6** - Testing and polish

### 11.2 Key Files to Create

**Must Create (Priority 1):**
1. `Cargo.toml` (workspace root)
2. `crates/bw-cli/Cargo.toml`
3. `crates/bw-core/Cargo.toml`
4. `crates/bw-cli/src/main.rs`
5. `crates/bw-cli/src/output/mod.rs`
6. `crates/bw-core/src/services/sdk.rs`
7. `crates/bw-core/src/services/container.rs`

**Should Create (Priority 2):**
8. All command modules in `crates/bw-cli/src/commands/`
9. `crates/bw-cli/src/output/formatter.rs`
10. `.gitignore`
11. `README.md`

**Can Defer (Priority 3):**
12. Integration tests
13. Documentation improvements
14. CI configuration

### 11.3 Implementation Tips

1. **Start Small:** Get a minimal CLI working first (just --help)
2. **Iterate:** Add commands incrementally, test each one
3. **Copy-Paste:** Command modules are very similar, use templates
4. **Reference TypeScript:** Check existing CLI for exact behavior
5. **Test Frequently:** Run `cargo clippy` and `cargo test` often
6. **Ask Questions:** Clarify ambiguities before implementing

### 11.4 Common Pitfalls to Avoid

1. **Don't implement command logic** - Only stubs in this phase
2. **Don't add custom crypto** - Always use SDK
3. **Don't skip tests** - Integration tests catch CLI breaking changes
4. **Don't ignore clippy** - Fix warnings as you go
5. **Don't forget cross-platform** - Test on multiple OS if possible

### 11.5 Success Indicators

You'll know you're done when:
- [ ] Binary compiles with zero warnings
- [ ] All commands listed in requirements are present
- [ ] Help text matches TypeScript CLI structure
- [ ] All output modes work (JSON, pretty, raw, quiet)
- [ ] Environment variables work
- [ ] All tests pass
- [ ] Binary size < 5MB stripped
- [ ] README has clear build instructions

### 11.6 Questions to Ask if Blocked

1. **CLI behavior unclear?** → Check TypeScript CLI source code
2. **SDK usage unclear?** → Check SDK documentation or example code
3. **Clap syntax unclear?** → Check clap documentation examples
4. **Test failing?** → Run with `--nocapture` to see output
5. **Build failing?** → Check SDK path dependency is correct

## 12. Appendix

### Appendix A: Complete Command List (40+ commands)

See requirements specification Appendix A (lines 605-688) for full list.

### Appendix B: Environment Variables

See requirements specification Appendix B (lines 690-700) for full list.

### Appendix C: Response Format Examples

See requirements specification Appendix C (lines 702-750) for examples.

### Appendix D: Dependencies Rationale

| Crate | Version | Purpose | Why This Version |
|-------|---------|---------|------------------|
| clap | 4.5 | CLI parsing | Latest stable, derive support |
| tokio | 1.40 | Async runtime | SDK requirement |
| serde | 1.0 | Serialization | Industry standard |
| anyhow | 1.0 | Error handling | Simple error context |
| bitwarden-core | path | SDK client | Internal SDK |
| bitwarden-crypto | path | Crypto operations | Internal SDK |
| reqwest | 0.12 | HTTP client | Async, rustls support |
| secrecy | 0.8 | Sensitive data | Memory protection |
| tracing | 0.1 | Logging | SDK compatibility |

### Appendix E: Useful Resources

- **Clap Documentation:** https://docs.rs/clap/latest/clap/
- **Clap Examples:** https://github.com/clap-rs/clap/tree/master/examples
- **Rust API Guidelines:** https://rust-lang.github.io/api-guidelines/
- **SDK Internal Docs:** https://sdk-api-docs.bitwarden.com/bitwarden_core/
- **TypeScript CLI Source:** `apps/cli/src/` in bitwarden/clients repo
- **Cargo Book:** https://doc.rust-lang.org/cargo/
- **Tokio Tutorial:** https://tokio.rs/tokio/tutorial

---

## Status: READY_FOR_IMPLEMENTATION

This implementation plan is complete and provides:
- ✅ Detailed architecture with code examples
- ✅ Complete file structure and organization
- ✅ All command stubs specified
- ✅ Response formatting system designed
- ✅ SDK integration pattern defined
- ✅ Testing strategy outlined
- ✅ Build and verification process documented
- ✅ Clear handoff instructions for implementer

The implementer can proceed with Phase 1 immediately.
