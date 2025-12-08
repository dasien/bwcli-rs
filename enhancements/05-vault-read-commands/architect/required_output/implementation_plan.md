---
enhancement: 05-vault-read-commands
agent: architect
task_id: task_1764949312_80733
timestamp: 2025-12-05T00:14:00Z
status: READY_FOR_IMPLEMENTATION
---

# Vault Read Commands - Implementation Plan

## Executive Summary

This implementation plan provides detailed architectural design and implementation guidance for the vault read operations enhancement. The design focuses on:

1. **Modular Architecture**: Clean separation between vault service, data models, and command handlers
2. **SDK Integration**: Leveraging Bitwarden SDK for all cryptographic operations
3. **Performance**: Efficient filtering and lazy decryption for large vaults
4. **Type Safety**: Comprehensive data models with proper serialization
5. **Compatibility**: Exact TypeScript CLI output format matching

**Estimated Implementation Effort**: 7-10 days for MVP (Phase 1)

**Critical Path Items**:
- Vault data models (blocking all operations)
- Sync service (blocking list/get operations)
- SDK decryption integration (blocking all decryption)

---

## Architecture Overview

### High-Level System Design

```
┌─────────────────────────────────────────────────────────────────┐
│                         CLI Commands                             │
│  (bw sync, bw list items, bw get password, etc.)               │
└──────────────────────┬──────────────────────────────────────────┘
                       │
                       v
┌─────────────────────────────────────────────────────────────────┐
│                     Command Handlers                             │
│  - execute_sync()  - execute_list()  - execute_get()           │
└──────────────────────┬──────────────────────────────────────────┘
                       │
                       v
┌─────────────────────────────────────────────────────────────────┐
│                       VaultService                               │
│  - sync_vault()                                                  │
│  - list_items() / list_folders() / list_collections()          │
│  - get_item() / get_field()                                     │
│  - search_items() / filter_items()                              │
└──────────┬─────────────────────┬───────────────┬────────────────┘
           │                     │               │
           v                     v               v
    ┌──────────┐        ┌──────────────┐   ┌─────────────┐
    │ API      │        │  Storage     │   │  SDK        │
    │ Client   │        │  (JsonFile)  │   │  Client     │
    └──────────┘        └──────────────┘   └─────────────┘
         │                      │                  │
         v                      v                  v
    [Bitwarden API]      [data.json]       [Crypto/Decrypt]
```

### Module Organization

```
crates/bw-core/src/
├── models/
│   ├── vault/                      [NEW]
│   │   ├── mod.rs
│   │   ├── cipher.rs              # Cipher data structures
│   │   ├── folder.rs              # Folder structures
│   │   ├── collection.rs          # Collection structures
│   │   ├── organization.rs        # Organization structures
│   │   ├── login.rs               # Login cipher type details
│   │   ├── card.rs                # Card cipher type details
│   │   ├── identity.rs            # Identity cipher type details
│   │   ├── secure_note.rs         # SecureNote cipher type details
│   │   └── sync_response.rs       # API sync response
│   └── ...
├── services/
│   ├── vault/                      [NEW]
│   │   ├── mod.rs
│   │   ├── sync_service.rs        # Sync operations
│   │   ├── cipher_service.rs      # Cipher decrypt/operations
│   │   ├── search_service.rs      # Search and filter logic
│   │   ├── totp_service.rs        # TOTP generation
│   │   └── errors.rs              # Vault-specific errors
│   └── ...

crates/bw-cli/src/
├── commands/
│   ├── sync.rs                     [UPDATE]
│   └── vault.rs                    [UPDATE]
└── ...
```

---

## Data Models Design

### Core Vault Models

All models follow the existing project pattern:
- Use `serde` for JSON serialization with `camelCase` for API compatibility
- Encrypted fields use `EncString` type (String containing "2.base64|base64|base64" format)
- UUIDs use String type for compatibility with TypeScript CLI

#### 1. Cipher Model (crates/bw-core/src/models/vault/cipher.rs)

```rust
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Encrypted vault cipher (item)
///
/// Matches Bitwarden API response format exactly.
/// All sensitive fields are encrypted using EncString format.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cipher {
    /// Cipher ID (UUID)
    pub id: String,

    /// Organization ID (if shared)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_id: Option<String>,

    /// Folder ID (if in folder, null if no folder)
    pub folder_id: Option<String>,

    /// Cipher type: 1=Login, 2=SecureNote, 3=Card, 4=Identity
    #[serde(rename = "type")]
    pub cipher_type: CipherType,

    /// Encrypted name (EncString format)
    pub name: String,

    /// Encrypted notes (EncString format)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,

    /// Whether this is a favorite
    pub favorite: bool,

    /// Collection IDs this cipher belongs to
    #[serde(default)]
    pub collection_ids: Vec<String>,

    /// Revision date (ISO 8601)
    pub revision_date: String,

    /// Creation date (ISO 8601)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creation_date: Option<String>,

    /// Deleted date (ISO 8601, present if in trash)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted_date: Option<String>,

    /// Login-specific data (if type=1)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub login: Option<CipherLogin>,

    /// Secure note data (if type=2)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secure_note: Option<CipherSecureNote>,

    /// Card data (if type=3)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card: Option<CipherCard>,

    /// Identity data (if type=4)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identity: Option<CipherIdentity>,

    /// Attachments
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<CipherAttachment>,

    /// Custom fields
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<CipherField>,

    /// Password history
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub password_history: Vec<PasswordHistory>,
}

/// Cipher type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum CipherType {
    Login = 1,
    SecureNote = 2,
    Card = 3,
    Identity = 4,
}

/// Login cipher type data
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CipherLogin {
    /// Encrypted username (EncString)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,

    /// Encrypted password (EncString)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,

    /// URIs associated with login
    #[serde(default)]
    pub uris: Vec<CipherLoginUri>,

    /// Encrypted TOTP secret (EncString)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub totp: Option<String>,

    /// Whether password should be auto-filled on page load
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autofill_on_page_load: Option<bool>,
}

/// Login URI
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CipherLoginUri {
    /// Encrypted URI (EncString)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,

    /// URI match type (0=Domain, 1=Host, 2=StartsWith, 3=Exact, 4=RegEx, 5=Never)
    #[serde(rename = "match")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_type: Option<UriMatchType>,
}

/// URI match type for URL matching
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum UriMatchType {
    Domain = 0,
    Host = 1,
    StartsWith = 2,
    Exact = 3,
    RegularExpression = 4,
    Never = 5,
}

/// Secure note data
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CipherSecureNote {
    /// Note type (always 0 for generic note)
    #[serde(rename = "type")]
    pub note_type: u8,
}

/// Card data
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CipherCard {
    /// Encrypted cardholder name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cardholder_name: Option<String>,

    /// Encrypted card number
    #[serde(skip_serializing_if = "Option::is_none")]
    pub number: Option<String>,

    /// Encrypted brand (Visa, Mastercard, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brand: Option<String>,

    /// Encrypted expiration month (MM format)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exp_month: Option<String>,

    /// Encrypted expiration year (YYYY format)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exp_year: Option<String>,

    /// Encrypted CVV code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

/// Identity data
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CipherIdentity {
    /// Encrypted title (Mr, Mrs, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// Encrypted first name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_name: Option<String>,

    /// Encrypted middle name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub middle_name: Option<String>,

    /// Encrypted last name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_name: Option<String>,

    /// Encrypted address line 1
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address1: Option<String>,

    /// Encrypted address line 2
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address2: Option<String>,

    /// Encrypted address line 3
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address3: Option<String>,

    /// Encrypted city
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,

    /// Encrypted state/province
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,

    /// Encrypted postal code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub postal_code: Option<String>,

    /// Encrypted country
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,

    /// Encrypted phone
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,

    /// Encrypted email
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,

    /// Encrypted SSN
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssn: Option<String>,

    /// Encrypted username
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,

    /// Encrypted passport number
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passport_number: Option<String>,

    /// Encrypted license number
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license_number: Option<String>,
}

/// Cipher attachment
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CipherAttachment {
    /// Attachment ID
    pub id: String,

    /// Encrypted filename
    pub file_name: String,

    /// File size in bytes
    pub size: Option<u64>,

    /// Size string (e.g., "1.2 MB")
    pub size_name: Option<String>,

    /// Download URL
    pub url: Option<String>,
}

/// Custom field
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CipherField {
    /// Encrypted field name
    pub name: String,

    /// Encrypted field value
    pub value: Option<String>,

    /// Field type: 0=Text, 1=Hidden, 2=Boolean
    #[serde(rename = "type")]
    pub field_type: u8,
}

/// Password history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PasswordHistory {
    /// Encrypted password
    pub password: String,

    /// Last used date (ISO 8601)
    pub last_used_date: String,
}

/// Decrypted cipher view for display
///
/// Used after SDK decryption for list/get operations.
/// All fields are plain text (decrypted).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CipherView {
    pub id: String,
    pub organization_id: Option<String>,
    pub folder_id: Option<String>,
    #[serde(rename = "type")]
    pub cipher_type: CipherType,
    pub name: String,
    pub notes: Option<String>,
    pub favorite: bool,
    pub collection_ids: Vec<String>,
    pub revision_date: String,
    pub creation_date: Option<String>,
    pub deleted_date: Option<String>,

    // Decrypted type-specific data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub login: Option<CipherLoginView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secure_note: Option<CipherSecureNote>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card: Option<CipherCardView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identity: Option<CipherIdentityView>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<CipherAttachment>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<CipherFieldView>,
}

/// Decrypted login view
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CipherLoginView {
    pub username: Option<String>,
    pub password: Option<String>,
    #[serde(default)]
    pub uris: Vec<CipherLoginUriView>,
    pub totp: Option<String>,
}

/// Decrypted URI view
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CipherLoginUriView {
    pub uri: Option<String>,
    #[serde(rename = "match")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_type: Option<UriMatchType>,
}

/// Decrypted card view
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CipherCardView {
    pub cardholder_name: Option<String>,
    pub number: Option<String>,
    pub brand: Option<String>,
    pub exp_month: Option<String>,
    pub exp_year: Option<String>,
    pub code: Option<String>,
}

/// Decrypted identity view
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CipherIdentityView {
    pub title: Option<String>,
    pub first_name: Option<String>,
    pub middle_name: Option<String>,
    pub last_name: Option<String>,
    pub address1: Option<String>,
    pub address2: Option<String>,
    pub address3: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub postal_code: Option<String>,
    pub country: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub ssn: Option<String>,
    pub username: Option<String>,
    pub passport_number: Option<String>,
    pub license_number: Option<String>,
}

/// Decrypted field view
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CipherFieldView {
    pub name: String,
    pub value: Option<String>,
    #[serde(rename = "type")]
    pub field_type: u8,
}
```

#### 2. Folder Model (crates/bw-core/src/models/vault/folder.rs)

```rust
use serde::{Deserialize, Serialize};

/// Encrypted folder
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Folder {
    /// Folder ID (UUID)
    pub id: String,

    /// Encrypted folder name (EncString)
    pub name: String,

    /// Revision date (ISO 8601)
    pub revision_date: String,
}

/// Decrypted folder view
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderView {
    pub id: String,
    pub name: String,
    pub revision_date: String,
}
```

#### 3. Collection Model (crates/bw-core/src/models/vault/collection.rs)

```rust
use serde::{Deserialize, Serialize};

/// Collection (shared folder within organization)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Collection {
    /// Collection ID (UUID)
    pub id: String,

    /// Organization ID
    pub organization_id: String,

    /// Encrypted collection name (EncString)
    pub name: String,

    /// External ID (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,

    /// Read only flag
    #[serde(default)]
    pub read_only: bool,
}

/// Decrypted collection view
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionView {
    pub id: String,
    pub organization_id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
    #[serde(default)]
    pub read_only: bool,
}
```

#### 4. Organization Model (crates/bw-core/src/models/vault/organization.rs)

```rust
use serde::{Deserialize, Serialize};

/// Organization (team/company)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Organization {
    /// Organization ID (UUID)
    pub id: String,

    /// Organization name (plain text)
    pub name: String,

    /// Status: 0=Invited, 1=Accepted, 2=Confirmed
    pub status: u8,

    /// Organization type: 0=Owner, 1=Admin, 2=User, 3=Manager
    #[serde(rename = "type")]
    pub org_type: u8,

    /// Whether user is enabled
    pub enabled: bool,

    /// Available features
    #[serde(default)]
    pub use_policies: bool,
    #[serde(default)]
    pub use_groups: bool,
    #[serde(default)]
    pub use_directory: bool,
    #[serde(default)]
    pub use_events: bool,
    #[serde(default)]
    pub use_totp: bool,
    #[serde(default)]
    pub use_api: bool,
    #[serde(default)]
    pub self_host: bool,

    /// Permissions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<OrganizationPermissions>,
}

/// Organization permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationPermissions {
    pub access_business_portal: bool,
    pub access_event_logs: bool,
    pub access_import_export: bool,
    pub access_reports: bool,
    pub manage_all_collections: bool,
    pub manage_assigned_collections: bool,
    pub manage_groups: bool,
    pub manage_policies: bool,
    pub manage_sso: bool,
    pub manage_users: bool,
    pub manage_reset_password: bool,
}
```

#### 5. Sync Response Model (crates/bw-core/src/models/vault/sync_response.rs)

```rust
use super::{Cipher, Folder, Collection, Organization};
use serde::{Deserialize, Serialize};

/// API sync endpoint response
///
/// Contains complete vault state from server.
/// Returned by GET /api/sync
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncResponse {
    /// Encrypted ciphers (vault items)
    #[serde(default)]
    pub ciphers: Vec<Cipher>,

    /// Encrypted folders
    #[serde(default)]
    pub folders: Vec<Folder>,

    /// Collections
    #[serde(default)]
    pub collections: Vec<Collection>,

    /// Organizations
    #[serde(default)]
    pub organizations: Vec<Organization>,

    /// Profile information (optional, not used in MVP)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<serde_json::Value>,

    /// Domains (optional, not used in MVP)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domains: Option<serde_json::Value>,
}

/// Vault data stored in local storage
///
/// Persisted to data.json after successful sync.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultData {
    /// Last sync timestamp (ISO 8601)
    pub last_sync: String,

    /// Encrypted ciphers
    #[serde(default)]
    pub ciphers: Vec<Cipher>,

    /// Encrypted folders
    #[serde(default)]
    pub folders: Vec<Folder>,

    /// Collections
    #[serde(default)]
    pub collections: Vec<Collection>,

    /// Organizations
    #[serde(default)]
    pub organizations: Vec<Organization>,
}
```

---

## Service Layer Architecture

### VaultService Design

The `VaultService` provides high-level vault operations and coordinates between storage, API client, and SDK.

#### Service Interface (crates/bw-core/src/services/vault/mod.rs)

```rust
use crate::models::vault::{
    Cipher, CipherView, Folder, FolderView, Collection, CollectionView,
    Organization, VaultData,
};
use crate::services::{api::BitwardenApiClient, storage::JsonFileStorage, sdk::Client};
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;

pub mod sync_service;
pub mod cipher_service;
pub mod search_service;
pub mod totp_service;
pub mod errors;

pub use errors::VaultError;
pub use sync_service::SyncService;
pub use cipher_service::CipherService;
pub use search_service::SearchService;
pub use totp_service::TotpService;

/// Main vault service coordinating all vault operations
pub struct VaultService {
    sync_service: SyncService,
    cipher_service: CipherService,
    search_service: SearchService,
    totp_service: TotpService,
}

impl VaultService {
    /// Create new vault service
    pub fn new(
        api_client: Arc<BitwardenApiClient>,
        storage: Arc<Mutex<JsonFileStorage>>,
        sdk_client: Arc<Client>,
    ) -> Self {
        let sync_service = SyncService::new(
            Arc::clone(&api_client),
            Arc::clone(&storage),
        );

        let cipher_service = CipherService::new(
            Arc::clone(&storage),
            Arc::clone(&sdk_client),
        );

        let search_service = SearchService::new();

        let totp_service = TotpService::new(Arc::clone(&sdk_client));

        Self {
            sync_service,
            cipher_service,
            search_service,
            totp_service,
        }
    }

    // Sync operations

    /// Sync vault from server
    pub async fn sync(&self, force: bool) -> Result<String, VaultError> {
        self.sync_service.sync(force).await
    }

    /// Get last sync timestamp
    pub async fn get_last_sync(&self) -> Result<Option<String>, VaultError> {
        self.sync_service.get_last_sync().await
    }

    // List operations

    /// List all items with optional filters
    pub async fn list_items(&self, filters: &ItemFilters) -> Result<Vec<CipherView>, VaultError> {
        let vault_data = self.get_vault_data().await?;
        let filtered = self.filter_ciphers(&vault_data.ciphers, filters);
        self.cipher_service.decrypt_ciphers(&filtered).await
    }

    /// List all folders
    pub async fn list_folders(&self, search: Option<&str>) -> Result<Vec<FolderView>, VaultError> {
        let vault_data = self.get_vault_data().await?;
        let mut folders = self.cipher_service.decrypt_folders(&vault_data.folders).await?;

        if let Some(search_term) = search {
            folders = self.search_service.filter_folders(folders, search_term);
        }

        Ok(folders)
    }

    /// List all collections
    pub async fn list_collections(
        &self,
        organization_id: Option<&str>,
        search: Option<&str>,
    ) -> Result<Vec<CollectionView>, VaultError> {
        let vault_data = self.get_vault_data().await?;
        let mut collections = self.cipher_service
            .decrypt_collections(&vault_data.collections)
            .await?;

        // Filter by organization
        if let Some(org_id) = organization_id {
            collections.retain(|c| c.organization_id == org_id);
        }

        // Filter by search
        if let Some(search_term) = search {
            collections = self.search_service.filter_collections(collections, search_term);
        }

        Ok(collections)
    }

    /// List all organizations
    pub async fn list_organizations(&self) -> Result<Vec<Organization>, VaultError> {
        let vault_data = self.get_vault_data().await?;
        Ok(vault_data.organizations)
    }

    // Get operations

    /// Get specific item by ID or search term
    pub async fn get_item(&self, id_or_search: &str) -> Result<CipherView, VaultError> {
        let vault_data = self.get_vault_data().await?;

        // Try to find by ID first
        let cipher = if let Some(cipher) = vault_data.ciphers.iter().find(|c| c.id == id_or_search) {
            cipher
        } else {
            // Search by name
            self.search_service.find_cipher_by_name(&vault_data.ciphers, id_or_search)
                .ok_or(VaultError::ItemNotFound)?
        };

        self.cipher_service.decrypt_cipher(cipher).await
    }

    /// Get specific field from item
    pub async fn get_field(&self, id_or_search: &str, field: FieldType) -> Result<String, VaultError> {
        let cipher_view = self.get_item(id_or_search).await?;
        self.extract_field(&cipher_view, field)
    }

    /// Generate TOTP code for item
    pub async fn get_totp(&self, id_or_search: &str) -> Result<String, VaultError> {
        let cipher_view = self.get_item(id_or_search).await?;

        let totp_secret = cipher_view
            .login
            .as_ref()
            .and_then(|l| l.totp.as_ref())
            .ok_or(VaultError::TotpNotConfigured)?;

        self.totp_service.generate_code(totp_secret).await
    }

    // Helper methods

    async fn get_vault_data(&self) -> Result<VaultData, VaultError> {
        let storage = self.sync_service.storage().lock().await;
        storage
            .get::<VaultData>("vaultData")
            .map_err(|_| VaultError::StorageError)?
            .ok_or(VaultError::NotSynced)
    }

    fn filter_ciphers(&self, ciphers: &[Cipher], filters: &ItemFilters) -> Vec<Cipher> {
        self.search_service.filter_ciphers(ciphers, filters)
    }

    fn extract_field(&self, cipher: &CipherView, field: FieldType) -> Result<String, VaultError> {
        match field {
            FieldType::Username => {
                cipher.login
                    .as_ref()
                    .and_then(|l| l.username.clone())
                    .ok_or(VaultError::FieldNotFound("username"))
            }
            FieldType::Password => {
                cipher.login
                    .as_ref()
                    .and_then(|l| l.password.clone())
                    .ok_or(VaultError::FieldNotFound("password"))
            }
            FieldType::Uri => {
                cipher.login
                    .as_ref()
                    .and_then(|l| l.uris.first())
                    .and_then(|u| u.uri.clone())
                    .ok_or(VaultError::FieldNotFound("uri"))
            }
            FieldType::Notes => {
                cipher.notes.clone()
                    .ok_or(VaultError::FieldNotFound("notes"))
            }
        }
    }
}

/// Item filter options for list operations
#[derive(Debug, Default, Clone)]
pub struct ItemFilters {
    pub organization_id: Option<String>,
    pub collection_id: Option<String>,
    pub folder_id: Option<String>,
    pub search: Option<String>,
    pub url: Option<String>,
    pub trash: bool,
}

/// Field types for extraction
#[derive(Debug, Clone, Copy)]
pub enum FieldType {
    Username,
    Password,
    Uri,
    Notes,
}
```

#### Sync Service (crates/bw-core/src/services/vault/sync_service.rs)

```rust
use super::errors::VaultError;
use crate::models::vault::{SyncResponse, VaultData};
use crate::services::{api::BitwardenApiClient, storage::JsonFileStorage};
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Service for vault synchronization operations
pub struct SyncService {
    api_client: Arc<BitwardenApiClient>,
    storage: Arc<Mutex<JsonFileStorage>>,
}

impl SyncService {
    pub fn new(
        api_client: Arc<BitwardenApiClient>,
        storage: Arc<Mutex<JsonFileStorage>>,
    ) -> Self {
        Self {
            api_client,
            storage,
        }
    }

    /// Sync vault from server
    ///
    /// # Arguments
    /// * `force` - Force full sync even if recently synced
    ///
    /// # Returns
    /// Last sync timestamp (ISO 8601 format)
    pub async fn sync(&self, force: bool) -> Result<String, VaultError> {
        // Check authentication
        if !self.api_client.is_authenticated().await {
            return Err(VaultError::NotAuthenticated);
        }

        // TODO: Implement smart sync logic (check last sync time if !force)

        // Fetch vault data from API
        let sync_response: SyncResponse = self.api_client
            .get_with_auth("/api/sync")
            .await
            .map_err(|e| VaultError::ApiError(e.to_string()))?;

        // Create vault data structure
        let now = chrono::Utc::now().to_rfc3339();
        let vault_data = VaultData {
            last_sync: now.clone(),
            ciphers: sync_response.ciphers,
            folders: sync_response.folders,
            collections: sync_response.collections,
            organizations: sync_response.organizations,
        };

        // Store in local storage (atomic operation)
        let mut storage = self.storage.lock().await;
        storage
            .set("vaultData", &vault_data)
            .await
            .map_err(|_| VaultError::StorageError)?;

        Ok(now)
    }

    /// Get last sync timestamp
    pub async fn get_last_sync(&self) -> Result<Option<String>, VaultError> {
        let storage = self.storage.lock().await;
        let vault_data: Option<VaultData> = storage
            .get("vaultData")
            .map_err(|_| VaultError::StorageError)?;

        Ok(vault_data.map(|v| v.last_sync))
    }

    pub fn storage(&self) -> &Arc<Mutex<JsonFileStorage>> {
        &self.storage
    }
}
```

#### Cipher Service (crates/bw-core/src/services/vault/cipher_service.rs)

```rust
use super::errors::VaultError;
use crate::models::vault::{
    Cipher, CipherView, Folder, FolderView, Collection, CollectionView,
    CipherLoginView, CipherLoginUriView, CipherCardView, CipherIdentityView,
    CipherFieldView,
};
use crate::services::{storage::JsonFileStorage, sdk::Client};
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Service for cipher decryption operations
///
/// Handles all SDK integration for decrypting vault items.
pub struct CipherService {
    storage: Arc<Mutex<JsonFileStorage>>,
    sdk_client: Arc<Client>,
}

impl CipherService {
    pub fn new(
        storage: Arc<Mutex<JsonFileStorage>>,
        sdk_client: Arc<Client>,
    ) -> Self {
        Self {
            storage,
            sdk_client,
        }
    }

    /// Decrypt a single cipher
    pub async fn decrypt_cipher(&self, cipher: &Cipher) -> Result<CipherView, VaultError> {
        // TODO: Use SDK to decrypt cipher
        // This is a placeholder - actual SDK integration required

        Ok(CipherView {
            id: cipher.id.clone(),
            organization_id: cipher.organization_id.clone(),
            folder_id: cipher.folder_id.clone(),
            cipher_type: cipher.cipher_type,
            name: self.decrypt_string(&cipher.name).await?,
            notes: if let Some(notes) = &cipher.notes {
                Some(self.decrypt_string(notes).await?)
            } else {
                None
            },
            favorite: cipher.favorite,
            collection_ids: cipher.collection_ids.clone(),
            revision_date: cipher.revision_date.clone(),
            creation_date: cipher.creation_date.clone(),
            deleted_date: cipher.deleted_date.clone(),
            login: if let Some(login) = &cipher.login {
                Some(self.decrypt_login(login).await?)
            } else {
                None
            },
            secure_note: cipher.secure_note.clone(),
            card: if let Some(card) = &cipher.card {
                Some(self.decrypt_card(card).await?)
            } else {
                None
            },
            identity: if let Some(identity) = &cipher.identity {
                Some(self.decrypt_identity(identity).await?)
            } else {
                None
            },
            attachments: cipher.attachments.clone(),
            fields: {
                let mut fields = Vec::new();
                for field in &cipher.fields {
                    fields.push(CipherFieldView {
                        name: self.decrypt_string(&field.name).await?,
                        value: if let Some(v) = &field.value {
                            Some(self.decrypt_string(v).await?)
                        } else {
                            None
                        },
                        field_type: field.field_type,
                    });
                }
                fields
            },
        })
    }

    /// Decrypt multiple ciphers
    pub async fn decrypt_ciphers(&self, ciphers: &[Cipher]) -> Result<Vec<CipherView>, VaultError> {
        let mut results = Vec::new();
        for cipher in ciphers {
            match self.decrypt_cipher(cipher).await {
                Ok(decrypted) => results.push(decrypted),
                Err(e) => {
                    // Log error but continue with other ciphers
                    eprintln!("Warning: Failed to decrypt cipher {}: {}", cipher.id, e);
                }
            }
        }
        Ok(results)
    }

    /// Decrypt folders
    pub async fn decrypt_folders(&self, folders: &[Folder]) -> Result<Vec<FolderView>, VaultError> {
        let mut results = Vec::new();
        for folder in folders {
            results.push(FolderView {
                id: folder.id.clone(),
                name: self.decrypt_string(&folder.name).await?,
                revision_date: folder.revision_date.clone(),
            });
        }
        Ok(results)
    }

    /// Decrypt collections
    pub async fn decrypt_collections(
        &self,
        collections: &[Collection],
    ) -> Result<Vec<CollectionView>, VaultError> {
        let mut results = Vec::new();
        for collection in collections {
            results.push(CollectionView {
                id: collection.id.clone(),
                organization_id: collection.organization_id.clone(),
                name: self.decrypt_string(&collection.name).await?,
                external_id: collection.external_id.clone(),
                read_only: collection.read_only,
            });
        }
        Ok(results)
    }

    // Private helper methods

    async fn decrypt_string(&self, enc_string: &str) -> Result<String, VaultError> {
        // TODO: Implement SDK decryption
        // For now, placeholder that returns the encrypted string
        // Real implementation will use:
        // self.sdk_client.decrypt_string(enc_string).await
        Ok(enc_string.to_string())
    }

    async fn decrypt_login(&self, login: &crate::models::vault::CipherLogin) -> Result<CipherLoginView, VaultError> {
        Ok(CipherLoginView {
            username: if let Some(u) = &login.username {
                Some(self.decrypt_string(u).await?)
            } else {
                None
            },
            password: if let Some(p) = &login.password {
                Some(self.decrypt_string(p).await?)
            } else {
                None
            },
            uris: {
                let mut uris = Vec::new();
                for uri in &login.uris {
                    uris.push(CipherLoginUriView {
                        uri: if let Some(u) = &uri.uri {
                            Some(self.decrypt_string(u).await?)
                        } else {
                            None
                        },
                        match_type: uri.match_type,
                    });
                }
                uris
            },
            totp: if let Some(t) = &login.totp {
                Some(self.decrypt_string(t).await?)
            } else {
                None
            },
        })
    }

    async fn decrypt_card(&self, card: &crate::models::vault::CipherCard) -> Result<CipherCardView, VaultError> {
        Ok(CipherCardView {
            cardholder_name: if let Some(n) = &card.cardholder_name {
                Some(self.decrypt_string(n).await?)
            } else {
                None
            },
            number: if let Some(n) = &card.number {
                Some(self.decrypt_string(n).await?)
            } else {
                None
            },
            brand: if let Some(b) = &card.brand {
                Some(self.decrypt_string(b).await?)
            } else {
                None
            },
            exp_month: if let Some(m) = &card.exp_month {
                Some(self.decrypt_string(m).await?)
            } else {
                None
            },
            exp_year: if let Some(y) = &card.exp_year {
                Some(self.decrypt_string(y).await?)
            } else {
                None
            },
            code: if let Some(c) = &card.code {
                Some(self.decrypt_string(c).await?)
            } else {
                None
            },
        })
    }

    async fn decrypt_identity(&self, identity: &crate::models::vault::CipherIdentity) -> Result<CipherIdentityView, VaultError> {
        // Similar pattern to decrypt_card - decrypt each optional field
        // Omitted for brevity but follows same pattern
        todo!("Implement identity decryption")
    }
}
```

#### Search Service (crates/bw-core/src/services/vault/search_service.rs)

```rust
use super::ItemFilters;
use crate::models::vault::{Cipher, FolderView, CollectionView};

/// Service for searching and filtering vault items
///
/// Provides efficient filtering without requiring full decryption.
pub struct SearchService;

impl SearchService {
    pub fn new() -> Self {
        Self
    }

    /// Filter ciphers based on criteria
    ///
    /// Returns encrypted ciphers (not decrypted yet).
    /// Filtering done on encrypted metadata (IDs, dates, structure).
    pub fn filter_ciphers(&self, ciphers: &[Cipher], filters: &ItemFilters) -> Vec<Cipher> {
        ciphers
            .iter()
            .filter(|cipher| {
                // Trash filter (exclude deleted by default)
                if filters.trash {
                    if cipher.deleted_date.is_none() {
                        return false;
                    }
                } else {
                    if cipher.deleted_date.is_some() {
                        return false;
                    }
                }

                // Organization filter
                if let Some(org_id) = &filters.organization_id {
                    if cipher.organization_id.as_ref() != Some(org_id) {
                        return false;
                    }
                }

                // Folder filter (including "no folder" as None)
                if let Some(folder_id) = &filters.folder_id {
                    if cipher.folder_id.as_ref() != Some(folder_id) {
                        return false;
                    }
                }

                // Collection filter
                if let Some(collection_id) = &filters.collection_id {
                    if !cipher.collection_ids.contains(collection_id) {
                        return false;
                    }
                }

                // Note: Search and URL filters require decryption, handled after

                true
            })
            .cloned()
            .collect()
    }

    /// Filter decrypted folders by search term
    pub fn filter_folders(&self, folders: Vec<FolderView>, search: &str) -> Vec<FolderView> {
        let search_lower = search.to_lowercase();
        folders
            .into_iter()
            .filter(|f| f.name.to_lowercase().contains(&search_lower))
            .collect()
    }

    /// Filter decrypted collections by search term
    pub fn filter_collections(
        &self,
        collections: Vec<CollectionView>,
        search: &str,
    ) -> Vec<CollectionView> {
        let search_lower = search.to_lowercase();
        collections
            .into_iter()
            .filter(|c| c.name.to_lowercase().contains(&search_lower))
            .collect()
    }

    /// Find cipher by name search (case-insensitive)
    ///
    /// Note: This requires the name to already be decrypted or
    /// we need to decrypt each cipher to search. For MVP, we'll
    /// decrypt all ciphers and search. Optimization: build search index.
    pub fn find_cipher_by_name<'a>(
        &self,
        ciphers: &'a [Cipher],
        search: &str,
    ) -> Option<&'a Cipher> {
        // For MVP: return first match by ID prefix
        // Real implementation will require decryption first
        ciphers.iter().find(|c| c.id.starts_with(search))
    }

    /// Search in decrypted cipher names (post-decryption filter)
    pub fn matches_search(&self, name: &str, notes: Option<&str>, search: &str) -> bool {
        let search_lower = search.to_lowercase();

        if name.to_lowercase().contains(&search_lower) {
            return true;
        }

        if let Some(n) = notes {
            if n.to_lowercase().contains(&search_lower) {
                return true;
            }
        }

        false
    }

    /// Match cipher URI against target URL
    pub fn matches_url(&self, uris: &[String], target_url: &str) -> bool {
        // Simplified URL matching for MVP
        // Full implementation would use UriMatchType and proper domain matching
        let target_lower = target_url.to_lowercase();

        uris.iter().any(|uri| {
            let uri_lower = uri.to_lowercase();
            uri_lower.contains(&target_lower) || target_lower.contains(&uri_lower)
        })
    }
}
```

#### TOTP Service (crates/bw-core/src/services/vault/totp_service.rs)

```rust
use super::errors::VaultError;
use crate::services::sdk::Client;
use anyhow::Result;
use std::sync::Arc;

/// Service for TOTP code generation
///
/// Uses SDK for all TOTP operations to ensure correctness.
pub struct TotpService {
    sdk_client: Arc<Client>,
}

impl TotpService {
    pub fn new(sdk_client: Arc<Client>) -> Self {
        Self { sdk_client }
    }

    /// Generate current TOTP code from secret
    ///
    /// # Arguments
    /// * `totp_secret` - TOTP secret string (otpauth:// URI or base32 secret)
    ///
    /// # Returns
    /// 6-digit TOTP code valid for current 30-second window
    pub async fn generate_code(&self, totp_secret: &str) -> Result<String, VaultError> {
        // TODO: Use SDK TOTP generation
        // Placeholder implementation
        // Real implementation:
        // self.sdk_client.generate_totp(totp_secret).await

        Ok("123456".to_string())
    }
}
```

#### Error Types (crates/bw-core/src/services/vault/errors.rs)

```rust
use thiserror::Error;

/// Vault service errors
#[derive(Debug, Error)]
pub enum VaultError {
    #[error("Not authenticated. Run 'bw login' first.")]
    NotAuthenticated,

    #[error("Vault not synced. Run 'bw sync' first.")]
    NotSynced,

    #[error("Item not found")]
    ItemNotFound,

    #[error("Field '{0}' not found on item")]
    FieldNotFound(&'static str),

    #[error("TOTP not configured for this item")]
    TotpNotConfigured,

    #[error("Storage error")]
    StorageError,

    #[error("API error: {0}")]
    ApiError(String),

    #[error("Decryption error: {0}")]
    DecryptionError(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),
}
```

---

## Command Implementation

### Sync Command (crates/bw-cli/src/commands/sync.rs)

```rust
use crate::output::Response;
use crate::GlobalArgs;
use bw_core::services::ServiceContainer;
use bw_core::services::vault::VaultService;
use clap::Args;

#[derive(Args)]
pub struct SyncCommand {
    /// Force full sync
    #[arg(long)]
    pub force: bool,

    /// Show last sync time only (no sync)
    #[arg(long)]
    pub last: bool,
}

pub async fn execute_sync(
    cmd: SyncCommand,
    global_args: &GlobalArgs,
) -> anyhow::Result<Response> {
    // Create service container
    let container = ServiceContainer::new(
        global_args.api_url.clone(),
        global_args.identity_url.clone(),
        global_args.data_dir.clone(),
        global_args.timeout,
    )?;

    // Create vault service
    let vault_service = VaultService::new(
        container.api_client(),
        container.storage(),
        Arc::new(container.sdk().clone()),
    );

    // Handle --last flag
    if cmd.last {
        match vault_service.get_last_sync().await? {
            Some(timestamp) => Ok(Response::success_message(timestamp)),
            None => Ok(Response::error("Never synced")),
        }
    } else {
        // Perform sync
        match vault_service.sync(cmd.force).await {
            Ok(timestamp) => Ok(Response::success_message(format!(
                "Syncing complete. Last sync: {}",
                timestamp
            ))),
            Err(e) => Ok(Response::error(e.to_string())),
        }
    }
}
```

### Vault List Commands (crates/bw-cli/src/commands/vault.rs - updated)

```rust
use crate::output::Response;
use crate::GlobalArgs;
use bw_core::services::ServiceContainer;
use bw_core::services::vault::{VaultService, ItemFilters};
use std::sync::Arc;

// ... (keep existing command definitions)

pub async fn execute_list(
    cmd: ListCommands,
    global_args: &GlobalArgs,
) -> anyhow::Result<Response> {
    let container = ServiceContainer::new(
        global_args.api_url.clone(),
        global_args.identity_url.clone(),
        global_args.data_dir.clone(),
        global_args.timeout,
    )?;

    let vault_service = VaultService::new(
        container.api_client(),
        container.storage(),
        Arc::new(container.sdk().clone()),
    );

    match cmd {
        ListCommands::Items(item_cmd) => {
            let filters = ItemFilters {
                organization_id: item_cmd.organizationid,
                collection_id: item_cmd.collectionid,
                folder_id: item_cmd.folderid,
                search: item_cmd.search,
                url: item_cmd.url,
                trash: item_cmd.trash,
            };

            match vault_service.list_items(&filters).await {
                Ok(items) => Ok(Response::success(items)),
                Err(e) => Ok(Response::error(e.to_string())),
            }
        }

        ListCommands::Folders(folder_cmd) => {
            match vault_service.list_folders(folder_cmd.search.as_deref()).await {
                Ok(folders) => Ok(Response::success(folders)),
                Err(e) => Ok(Response::error(e.to_string())),
            }
        }

        ListCommands::Collections(collection_cmd) => {
            match vault_service.list_collections(
                collection_cmd.organizationid.as_deref(),
                collection_cmd.search.as_deref(),
            ).await {
                Ok(collections) => Ok(Response::success(collections)),
                Err(e) => Ok(Response::error(e.to_string())),
            }
        }

        ListCommands::Organizations(_) => {
            match vault_service.list_organizations().await {
                Ok(orgs) => Ok(Response::success(orgs)),
                Err(e) => Ok(Response::error(e.to_string())),
            }
        }

        ListCommands::OrgCollections(_) | ListCommands::OrgMembers(_) => {
            Ok(Response::error("Not yet implemented"))
        }
    }
}

pub async fn execute_get(
    cmd: GetCommands,
    global_args: &GlobalArgs,
) -> anyhow::Result<Response> {
    let container = ServiceContainer::new(
        global_args.api_url.clone(),
        global_args.identity_url.clone(),
        global_args.data_dir.clone(),
        global_args.timeout,
    )?;

    let vault_service = VaultService::new(
        container.api_client(),
        container.storage(),
        Arc::new(container.sdk().clone()),
    );

    use bw_core::services::vault::FieldType;

    match cmd {
        GetCommands::Item(item_cmd) => {
            match vault_service.get_item(&item_cmd.id).await {
                Ok(item) => Ok(Response::success(item)),
                Err(e) => Ok(Response::error(e.to_string())),
            }
        }

        GetCommands::Username(username_cmd) => {
            match vault_service.get_field(&username_cmd.id, FieldType::Username).await {
                Ok(username) => {
                    if global_args.raw {
                        println!("{}", username);
                        Ok(Response::success_message(""))
                    } else {
                        Ok(Response::success(username))
                    }
                }
                Err(e) => Ok(Response::error(e.to_string())),
            }
        }

        GetCommands::Password(password_cmd) => {
            match vault_service.get_field(&password_cmd.id, FieldType::Password).await {
                Ok(password) => {
                    if global_args.raw {
                        println!("{}", password);
                        Ok(Response::success_message(""))
                    } else {
                        Ok(Response::success(password))
                    }
                }
                Err(e) => Ok(Response::error(e.to_string())),
            }
        }

        GetCommands::Uri(uri_cmd) => {
            match vault_service.get_field(&uri_cmd.id, FieldType::Uri).await {
                Ok(uri) => {
                    if global_args.raw {
                        println!("{}", uri);
                        Ok(Response::success_message(""))
                    } else {
                        Ok(Response::success(uri))
                    }
                }
                Err(e) => Ok(Response::error(e.to_string())),
            }
        }

        GetCommands::Totp(totp_cmd) => {
            match vault_service.get_totp(&totp_cmd.id).await {
                Ok(code) => {
                    if global_args.raw {
                        println!("{}", code);
                        Ok(Response::success_message(""))
                    } else {
                        Ok(Response::success(code))
                    }
                }
                Err(e) => Ok(Response::error(e.to_string())),
            }
        }

        _ => Ok(Response::error("Not yet implemented")),
    }
}
```

---

## SDK Integration Strategy

### SDK Decryption Flow

1. **Session Key Retrieval**: Get decryption key from secure storage
2. **EncString Parsing**: Parse EncString format ("2.base64|base64|base64")
3. **Decryption**: Use SDK crypto functions to decrypt
4. **Memory Safety**: Use `secrecy` crate to protect decrypted values

### Implementation Notes

The SDK integration details are placeholders in this plan because the actual SDK API needs to be explored during implementation. Key integration points:

1. **Cipher Decryption**: `cipher_service.rs` needs SDK calls
2. **TOTP Generation**: `totp_service.rs` needs SDK TOTP functions
3. **EncString Handling**: Create helper functions for EncString parsing

**Action Item for Implementer**: Review SDK documentation and implement actual SDK calls in `CipherService::decrypt_string()` and `TotpService::generate_code()`.

---

## Testing Strategy

### Unit Tests

**Test Files to Create**:

1. `crates/bw-core/src/models/vault/tests.rs`
   - Serialization/deserialization of all models
   - Validate camelCase JSON output
   - Test optional field handling

2. `crates/bw-core/src/services/vault/search_service_tests.rs`
   - Filter logic with various combinations
   - Search matching (case-insensitive, substring)
   - URL matching logic

3. `crates/bw-core/src/services/vault/sync_service_tests.rs`
   - Mock API responses
   - Storage operations
   - Error handling

### Integration Tests

**Test File**: `crates/bw-cli/tests/vault_integration_tests.rs`

```rust
#[tokio::test]
async fn test_sync_and_list_flow() {
    // Setup test environment
    // Mock API responses
    // Execute sync command
    // Execute list command
    // Verify output format
}

#[tokio::test]
async fn test_get_password_flow() {
    // Setup synced vault with test data
    // Execute get password command
    // Verify decryption and output
}
```

### Compatibility Tests

**Test File**: `crates/bw-cli/tests/typescript_compatibility_tests.rs`

Compare JSON output with TypeScript CLI using fixtures:

```rust
#[test]
fn test_cipher_view_format() {
    let cipher_view = create_test_cipher_view();
    let json = serde_json::to_string(&cipher_view).unwrap();

    // Load expected JSON from TypeScript CLI output
    let expected = load_fixture("typescript_cipher_view.json");

    // Compare structure (ignoring IDs and dates)
    assert_json_structure_matches(json, expected);
}
```

---

## Performance Considerations

### Optimization Strategies

1. **Lazy Decryption**: Only decrypt fields needed for display
   - List operations: decrypt name and folder only
   - Get operations: decrypt all fields

2. **Decryption Caching**: Cache decrypted ciphers during command execution
   - Use HashMap with cipher ID as key
   - Clear cache after command completes

3. **Parallel Decryption**: Decrypt multiple ciphers concurrently
   - Use `tokio::spawn` for parallel tasks
   - Batch size: 50 ciphers per batch

4. **Search Optimization** (Future):
   - Build in-memory search index on sync
   - Index: name → cipher ID mapping
   - Only decrypt ciphers that match search

### Performance Targets

- **Sync**: <10 seconds for 100 items
- **List**: <1 second for 1000 items
- **Get**: <500ms per item
- **TOTP**: <100ms per code generation

### Profiling Points

Add tracing instrumentation:

```rust
use tracing::{info, instrument};

#[instrument(skip(self))]
pub async fn decrypt_cipher(&self, cipher: &Cipher) -> Result<CipherView, VaultError> {
    let start = std::time::Instant::now();
    // ... decryption logic
    info!("Decryption took {:?}", start.elapsed());
    Ok(result)
}
```

---

## Error Handling & User Experience

### Error Message Guidelines

1. **Authentication Errors**: Clear next steps
   ```
   "Not authenticated. Run 'bw login' first."
   ```

2. **Sync Errors**: Helpful recovery steps
   ```
   "Vault not synced. Run 'bw sync' to download your vault."
   ```

3. **Not Found Errors**: Suggest alternatives
   ```
   "Item not found. Use 'bw list items' to see available items."
   ```

4. **Field Errors**: Explain what's missing
   ```
   "This item does not have a password field."
   ```

### Progress Indicators

For long operations (sync with 100+ items):

```rust
use indicatif::{ProgressBar, ProgressStyle};

let pb = ProgressBar::new(total_items as u64);
pb.set_style(
    ProgressStyle::default_bar()
        .template("[{elapsed_precise}] {bar:40} {pos}/{len} {msg}")
        .unwrap()
);

for (i, cipher) in ciphers.iter().enumerate() {
    // Process cipher
    pb.set_position(i as u64);
    pb.set_message(format!("Processing: {}", cipher.id));
}

pb.finish_with_message("Sync complete");
```

---

## Implementation Phasing

### Phase 1: MVP Core (Days 1-7)

**Day 1-2: Data Models**
- [ ] Create all vault data models (cipher, folder, collection, organization)
- [ ] Implement serialization tests
- [ ] Validate JSON output format

**Day 3-4: Sync Service**
- [ ] Implement `SyncService`
- [ ] API integration for `/api/sync`
- [ ] Storage integration
- [ ] Update sync command handler
- [ ] Test sync flow end-to-end

**Day 5-6: List Commands**
- [ ] Implement `SearchService` filtering
- [ ] Implement `CipherService` decryption (MVP: basic fields)
- [ ] Update list command handlers
- [ ] Test all list commands

**Day 7: Get Commands**
- [ ] Implement field extraction
- [ ] Implement basic TOTP (stub for MVP)
- [ ] Update get command handlers
- [ ] Test get operations

### Phase 2: SDK Integration & Polish (Days 8-10)

**Day 8: SDK Decryption**
- [ ] Implement real SDK decryption in `CipherService`
- [ ] Handle all cipher types
- [ ] Test decryption accuracy

**Day 9: TOTP & Advanced Features**
- [ ] Implement real TOTP generation
- [ ] Test TOTP codes against authenticator apps
- [ ] Implement URL matching for `--url` filter

**Day 10: Testing & Compatibility**
- [ ] Compatibility tests vs TypeScript CLI
- [ ] Performance profiling
- [ ] Bug fixes and polish

---

## Migration from TypeScript CLI

### Output Compatibility Checklist

- [ ] JSON field names use camelCase
- [ ] Optional fields omitted when null (use `skip_serializing_if`)
- [ ] Date formats use ISO 8601
- [ ] UUID fields are strings (not UUID types)
- [ ] Array fields default to empty array (not null)
- [ ] Enum values match numeric representation

### Known Differences to Document

1. **Attachment Downloads**: Not implemented in MVP (Phase 2)
2. **Organization Members**: Not implemented in MVP (Phase 2)
3. **Templates**: Not implemented in MVP (Phase 2)

---

## Security Considerations

### Memory Safety

1. **Use `secrecy` crate** for passwords and keys:
   ```rust
   use secrecy::{Secret, ExposeSecret, Zeroize};

   let password: Secret<String> = Secret::new(decrypted_password);
   // Use: password.expose_secret()
   // Auto-zeroized on drop
   ```

2. **Zeroize sensitive data** explicitly:
   ```rust
   use zeroize::Zeroize;

   let mut buffer = decrypted_data;
   buffer.zeroize(); // Clear memory
   ```

3. **Never log sensitive data**:
   ```rust
   // BAD
   tracing::debug!("Password: {}", password);

   // GOOD
   tracing::debug!("Password field present: {}", password.is_some());
   ```

### Validation

1. **Validate UUIDs** before queries:
   ```rust
   use uuid::Uuid;

   fn validate_id(id: &str) -> Result<(), VaultError> {
       Uuid::parse_str(id)
           .map_err(|_| VaultError::InvalidInput("Invalid ID format"))?;
       Ok(())
   }
   ```

2. **Sanitize search terms**:
   ```rust
   fn sanitize_search(term: &str) -> String {
       term.trim().to_lowercase()
   }
   ```

---

## Dependencies Required

### Cargo.toml Updates

Add to `workspace.dependencies`:

```toml
# Date/time handling
chrono = { version = "0.4", features = ["serde"] }

# TOTP (if not using SDK)
totp-lite = "2.0"
```

No new dependencies required beyond what's already in the workspace manifest.

---

## Documentation Requirements

### Code Documentation

1. **Module-level docs** for each service:
   ```rust
   //! Vault synchronization service
   //!
   //! Handles downloading vault data from Bitwarden API and caching locally.
   //!
   //! # Example
   //! ```rust,no_run
   //! let sync_service = SyncService::new(api_client, storage);
   //! let timestamp = sync_service.sync(false).await?;
   //! ```
   ```

2. **Function docs** with examples for public API

3. **Error docs** explaining when each error occurs

### User Documentation

Create `docs/vault-read-commands.md`:

1. Command usage examples
2. Filter combinations
3. Output format reference
4. Troubleshooting guide

---

## Acceptance Criteria Summary

### Sync Command
- [ ] `bw sync` downloads and caches vault data
- [ ] `bw sync --last` shows last sync timestamp
- [ ] `bw sync --force` forces full re-sync
- [ ] Requires authentication
- [ ] Shows progress for large vaults
- [ ] Handles API errors gracefully

### List Commands
- [ ] `bw list items` returns all items
- [ ] All filters work: folder, collection, organization, search, url, trash
- [ ] `bw list folders` returns folders
- [ ] `bw list collections` returns collections
- [ ] `bw list organizations` returns organizations
- [ ] Requires prior sync
- [ ] JSON output matches TypeScript CLI

### Get Commands
- [ ] `bw get item <id>` returns full item
- [ ] `bw get password <id>` extracts password
- [ ] `bw get username <id>` extracts username
- [ ] `bw get uri <id>` extracts first URI
- [ ] `bw get totp <id>` generates current code
- [ ] Search by name works if not UUID
- [ ] --raw flag outputs plain text

### General
- [ ] All operations require authentication
- [ ] List/get require prior sync
- [ ] Error messages are clear and actionable
- [ ] Output format exactly matches TypeScript CLI
- [ ] Performance meets targets
- [ ] All tests pass

---

## Next Steps for Implementer

1. **Start with Data Models**: Create all structs in `models/vault/`
2. **Test Serialization**: Ensure JSON output matches TypeScript CLI
3. **Implement Sync**: Get end-to-end sync working first
4. **Add Decryption**: Integrate SDK decryption (may need SDK team help)
5. **Implement List**: Add filtering and search
6. **Implement Get**: Add field extraction
7. **Add TOTP**: Integrate SDK TOTP generation
8. **Test Compatibility**: Compare output with TypeScript CLI
9. **Performance Tune**: Profile and optimize hot paths
10. **Polish UX**: Add progress indicators, improve error messages

---

## Appendix: TypeScript CLI Reference Files

For implementation reference, consult these TypeScript CLI files:

1. **Commands**:
   - `apps/cli/src/vault/commands/sync.command.ts`
   - `apps/cli/src/vault/commands/list.command.ts`
   - `apps/cli/src/vault/commands/get.command.ts`

2. **Services**:
   - `libs/common/src/services/sync/sync.service.ts`
   - `libs/common/src/services/cipher.service.ts`

3. **Models**:
   - `libs/common/src/models/domain/cipher.ts`
   - `libs/common/src/models/view/cipher.view.ts`

4. **API**:
   - `libs/common/src/models/response/sync.response.ts`

---

## Status: READY_FOR_IMPLEMENTATION

This implementation plan provides complete architectural design and detailed specifications for implementing vault read operations. All architectural decisions are documented, data models are fully specified, and the implementation path is clear.

**Estimated Effort**: 7-10 days for MVP (Phase 1)
**Complexity**: High (SDK integration, decryption, compatibility)
**Risk Level**: Medium (SDK learning curve, TOTP accuracy critical)

The implementer should proceed with Phase 1 (MVP Core) as outlined above.
