# SDK API Reference for Vault Migration

## VaultClient API

### Client Extension

```rust
use bitwarden_vault::VaultClientExt;

// Access vault client from initialized SDK client
let vault_client = client.vault();
```

### CiphersClient Methods

```rust
// Get ciphers client
let ciphers_client = client.vault().ciphers();

// Encrypt a cipher view (creates EncryptionContext)
pub fn encrypt(&self, cipher_view: CipherView) -> Result<EncryptionContext, EncryptError>;

// Decrypt a single cipher
pub fn decrypt(&self, cipher: Cipher) -> Result<CipherView, DecryptError>;

// Decrypt multiple ciphers to list view
pub fn decrypt_list(&self, ciphers: Vec<Cipher>) -> Result<Vec<CipherListView>, DecryptError>;

// Decrypt with failure handling
pub fn decrypt_list_with_failures(&self, ciphers: Vec<Cipher>) -> DecryptCipherListResult;
```

### FoldersClient Methods

```rust
// Get folders client
let folders_client = client.vault().folders();

// Note: FoldersClient doesn't expose direct decrypt methods
// Use KeyStore for folder decryption
```

### KeyStore Decryption (for Folders/Collections)

```rust
use bitwarden_crypto::{Decryptable, IdentifyKey};
use bitwarden_vault::Folder;

// Get key store from client
let key_store = client.internal.get_key_store();

// Decrypt folders
let folder_views: Result<Vec<FolderView>, _> = key_store.decrypt_list(&folders);

// Decrypt collections
let collection_views: Result<Vec<CollectionView>, _> = key_store.decrypt_list(&collections);
```

## SDK Type Definitions

### Cipher (Encrypted)

```rust
// From bitwarden_vault::cipher::cipher
pub struct Cipher {
    pub id: Option<CipherId>,
    pub organization_id: Option<OrganizationId>,
    pub folder_id: Option<FolderId>,
    pub collection_ids: Vec<CollectionId>,
    pub key: Option<EncString>,
    pub name: EncString,
    pub notes: Option<EncString>,
    pub r#type: CipherType,
    pub login: Option<Login>,
    pub identity: Option<Identity>,
    pub card: Option<Card>,
    pub secure_note: Option<SecureNote>,
    pub ssh_key: Option<SshKey>,
    pub favorite: bool,
    pub reprompt: CipherRepromptType,
    pub organization_use_totp: bool,
    pub edit: bool,
    pub permissions: Option<CipherPermissions>,
    pub view_password: bool,
    pub local_data: Option<LocalData>,
    pub attachments: Option<Vec<Attachment>>,
    pub fields: Option<Vec<Field>>,
    pub password_history: Option<Vec<PasswordHistory>>,
    pub creation_date: DateTime<Utc>,
    pub deleted_date: Option<DateTime<Utc>>,
    pub revision_date: DateTime<Utc>,
    pub archived_date: Option<DateTime<Utc>>,
    pub data: Option<Box<RawValue>>,
}
```

### CipherView (Decrypted)

```rust
// From bitwarden_vault::cipher::cipher
pub struct CipherView {
    pub id: Option<CipherId>,
    pub organization_id: Option<OrganizationId>,
    pub folder_id: Option<FolderId>,
    pub collection_ids: Vec<CollectionId>,
    pub key: Option<EncString>,
    pub name: String,          // Decrypted
    pub notes: Option<String>, // Decrypted
    pub r#type: CipherType,
    pub login: Option<LoginView>,
    pub identity: Option<IdentityView>,
    pub card: Option<CardView>,
    pub secure_note: Option<SecureNoteView>,
    pub ssh_key: Option<SshKeyView>,
    pub favorite: bool,
    pub reprompt: CipherRepromptType,
    pub organization_use_totp: bool,
    pub edit: bool,
    pub permissions: Option<CipherPermissions>,
    pub view_password: bool,
    pub local_data: Option<LocalData>,
    pub attachments: Option<Vec<AttachmentView>>,
    pub fields: Option<Vec<FieldView>>,
    pub password_history: Option<Vec<PasswordHistoryView>>,
    pub creation_date: DateTime<Utc>,
    pub deleted_date: Option<DateTime<Utc>>,
    pub revision_date: DateTime<Utc>,
    pub archived_date: Option<DateTime<Utc>>,
}
```

### CipherListView (Decrypted, Lightweight)

```rust
// From bitwarden_vault::cipher::cipher
pub struct CipherListView {
    pub id: Option<CipherId>,
    pub organization_id: Option<OrganizationId>,
    pub folder_id: Option<FolderId>,
    pub collection_ids: Vec<CollectionId>,
    pub key: Option<EncString>,
    pub name: String,
    pub subtitle: String,      // Computed from login username/card brand/etc.
    pub r#type: CipherType,
    pub favorite: bool,
    pub reprompt: CipherRepromptType,
    pub organization_use_totp: bool,
    pub edit: bool,
    pub permissions: Option<CipherPermissions>,
    pub view_password: bool,
    pub attachments: u32,      // Count only
    pub creation_date: DateTime<Utc>,
    pub deleted_date: Option<DateTime<Utc>>,
    pub revision_date: DateTime<Utc>,
    pub archived_date: Option<DateTime<Utc>>,
}
```

### Folder (Encrypted)

```rust
// From bitwarden_vault::folder::folder_models
pub struct Folder {
    pub id: Option<FolderId>,
    pub name: EncString,
    pub revision_date: DateTime<Utc>,
}
```

### FolderView (Decrypted)

```rust
// From bitwarden_vault::folder::folder_models
pub struct FolderView {
    pub id: Option<FolderId>,
    pub name: String,          // Decrypted
    pub revision_date: DateTime<Utc>,
}
```

### Collection (Encrypted)

```rust
// From bitwarden_collections::collection
pub struct Collection {
    pub id: Option<CollectionId>,
    pub organization_id: OrganizationId,
    pub name: EncString,
    pub external_id: Option<String>,
    pub hide_passwords: bool,
    pub read_only: bool,
    pub manage: bool,
    pub default_user_collection_email: Option<String>,
    pub r#type: CollectionType,
}
```

### CollectionView (Decrypted)

```rust
// From bitwarden_collections::collection
pub struct CollectionView {
    pub id: Option<CollectionId>,
    pub organization_id: OrganizationId,
    pub name: String,          // Decrypted
    pub external_id: Option<String>,
    pub hide_passwords: bool,
    pub read_only: bool,
    pub manage: bool,
    pub r#type: CollectionType,
}
```

## Crypto Initialization API

### InitUserCryptoRequest

```rust
// From bitwarden_core::key_management::crypto
pub struct InitUserCryptoRequest {
    pub user_id: Option<UserId>,
    pub kdf_params: Kdf,
    pub email: String,
    pub private_key: EncString,
    pub signing_key: Option<EncString>,
    pub security_state: Option<SignedSecurityState>,
    pub method: InitUserCryptoMethod,
}
```

### InitUserCryptoMethod

```rust
// From bitwarden_core::key_management::crypto
pub enum InitUserCryptoMethod {
    Password {
        password: String,
        user_key: EncString,
    },
    MasterPasswordUnlock {
        password: String,
        master_password_unlock: MasterPasswordUnlockData,
    },
    DecryptedKey {
        decrypted_user_key: String,  // Base64 encoded
    },
    Pin {
        pin: String,
        pin_protected_user_key: EncString,
    },
    // ... other methods
}
```

### CryptoClient Methods

```rust
// From bitwarden_core::key_management::crypto_client
let crypto_client = client.crypto();

// Initialize user crypto state
pub async fn initialize_user_crypto(
    &self,
    req: InitUserCryptoRequest,
) -> Result<(), EncryptionSettingsError>;

// Initialize org crypto state (after user crypto)
pub async fn initialize_org_crypto(
    &self,
    req: InitOrgCryptoRequest,
) -> Result<(), EncryptionSettingsError>;

// Get user encryption key (for storage)
pub async fn get_user_encryption_key(&self) -> Result<B64, CryptoClientError>;
```

## EncryptionContext

Returned by `CiphersClient::encrypt()`:

```rust
pub struct EncryptionContext {
    pub cipher: Cipher,
    pub encrypted_for: UserId,
}
```

## UUID Newtypes

All IDs are UUID newtypes with these methods:

```rust
impl CipherId {
    pub fn new(id: Uuid) -> Self;
    pub fn to_string(&self) -> String;  // For storage keys
}

// Parse from string
let id: CipherId = "uuid-string".parse()?;
```

## Serde Configuration

All SDK types use:

```rust
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]  // Some types
```

This matches CLI storage format - no conversion needed.
