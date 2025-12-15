# Code Changes Detail

## SdkVaultBridge Implementation

### Core Structure

The `SdkVaultBridge` wraps an `Arc<Client>` (the SDK client) and provides vault operations:

```rust
pub struct SdkVaultBridge {
    client: Arc<Client>,
}
```

### Key Methods

#### `initialize_crypto`

Initializes SDK crypto state with user credentials:

```rust
pub async fn initialize_crypto(
    &self,
    user_key: &SymmetricCryptoKey,  // Decrypted from protected storage
    email: &str,                      // User's email
    kdf_config: &KdfConfig,          // PBKDF2 or Argon2 config
    private_key: Option<&str>,        // Encrypted RSA private key
) -> Result<(), VaultError>
```

Implementation notes:
- Uses `InitUserCryptoMethod::DecryptedKey` to avoid re-deriving keys
- Converts user key to base64 string for SDK compatibility
- Handles missing private key with dummy value (for basic operations)

#### `is_crypto_initialized`

Checks if SDK crypto is ready:

```rust
pub fn is_crypto_initialized(&self) -> bool
```

Uses internal SDK API to check KeyStore state.

### Type Conversion Functions

#### JSON-Based (for encrypted types)

```rust
// CLI -> SDK
fn cli_cipher_to_sdk_via_json(cli: &Cipher) -> Result<bitwarden_vault::Cipher, VaultError>
fn cli_folder_to_sdk_via_json(cli: &Folder) -> Result<bitwarden_vault::Folder, VaultError>
fn cli_collection_to_sdk_via_json(cli: &Collection) -> Result<sdk_collection::Collection, VaultError>

// SDK -> CLI
fn sdk_cipher_to_cli_via_json(sdk: &bitwarden_vault::Cipher) -> Result<Cipher, VaultError>
```

#### Direct Field Mapping (for view types)

```rust
// SDK View -> CLI View
fn sdk_cipher_view_to_cli(sdk: &bitwarden_vault::CipherView) -> Result<CipherView, VaultError>
fn sdk_login_view_to_cli(sdk: &bitwarden_vault::LoginView) -> CipherLoginView
fn sdk_card_view_to_cli(sdk: &bitwarden_vault::CardView) -> CipherCardView
fn sdk_identity_view_to_cli(sdk: &bitwarden_vault::IdentityView) -> CipherIdentityView
fn sdk_field_view_to_cli(sdk: &bitwarden_vault::FieldView) -> CipherFieldView
fn sdk_folder_view_to_cli(sdk: &bitwarden_vault::FolderView) -> Result<FolderView, VaultError>
fn sdk_collection_view_to_cli(sdk: &sdk_collection::CollectionView) -> Result<CollectionView, VaultError>

// CLI View -> SDK View (for encryption)
fn cli_cipher_view_to_sdk(cli: &CipherView) -> Result<bitwarden_vault::CipherView, VaultError>
fn cli_login_view_to_sdk(cli: &CipherLoginView) -> bitwarden_vault::LoginView
fn cli_card_view_to_sdk(cli: &CipherCardView) -> bitwarden_vault::CardView
fn cli_identity_view_to_sdk(cli: &CipherIdentityView) -> bitwarden_vault::IdentityView
fn cli_field_view_to_sdk(cli: &CipherFieldView) -> bitwarden_vault::FieldView
fn cli_login_uri_view_to_sdk(cli: &CipherLoginUriView) -> bitwarden_vault::LoginUriView
```

## Dependency Changes

### workspace Cargo.toml

```diff
 # Bitwarden SDK (path dependencies from ../sdk-internal/)
 bitwarden-core = { path = "../sdk-internal/crates/bitwarden-core", version = "=1.0.0" }
 bitwarden-crypto = { path = "../sdk-internal/crates/bitwarden-crypto", version = "=1.0.0" }
 bitwarden-auth = { path = "../sdk-internal/crates/bitwarden-auth", version = "=1.0.0" }
 bitwarden-vault = { path = "../sdk-internal/crates/bitwarden-vault", version = "=1.0.0" }
+bitwarden-collections = { path = "../sdk-internal/crates/bitwarden-collections", version = "=1.0.0" }
 bitwarden-generators = { path = "../sdk-internal/crates/bitwarden-generators", version = "=1.0.0" }
```

### bw-core/Cargo.toml

```diff
 # Bitwarden SDK
 bitwarden-core = { workspace = true, features = ["internal"] }
 bitwarden-crypto.workspace = true
 bitwarden-auth.workspace = true
 bitwarden-vault.workspace = true
+bitwarden-collections.workspace = true
 bitwarden-generators.workspace = true
```

## Error Type Addition

### errors.rs

```diff
 #[error("IO error: {0}")]
 IoError(String),
+
+#[error("SDK crypto initialization failed: {0}")]
+CryptoInitFailed(String),
```

## Module Export Changes

### vault/mod.rs

```diff
 pub mod cipher_service;
 pub mod confirmation_service;
 pub mod errors;
+pub mod sdk_bridge;
 pub mod search_service;
 pub mod sync_service;
 pub mod totp_service;
 pub mod validation_service;
 pub mod write_service;

 pub use cipher_service::CipherService;
 pub use confirmation_service::ConfirmationService;
 pub use errors::VaultError;
+pub use sdk_bridge::SdkVaultBridge;
 pub use search_service::{ItemFilters, SearchService};
```

## Usage Example

```rust
use bw_core::services::vault::SdkVaultBridge;
use bitwarden_core::Client;

// Create SDK client
let client = Client::new(None);
let bridge = SdkVaultBridge::new(Arc::new(client));

// Initialize crypto (must be done before vault operations)
bridge.initialize_crypto(
    &user_key,           // From protected storage
    "user@example.com",  // From account info
    &kdf_config,         // From storage
    Some(&private_key),  // From storage (optional)
).await?;

// Now vault operations work
let decrypted = bridge.decrypt_cipher(&encrypted_cipher)?;
let encrypted = bridge.encrypt_cipher(&cipher_view)?;
```

## Integration Pattern

The recommended pattern for integrating with existing services:

```rust
// In CipherService or VaultService
impl CipherService {
    pub async fn decrypt_ciphers_with_sdk(
        &self,
        ciphers: &[Cipher],
        user_key: &SymmetricCryptoKey,
        account_info: &AccountInfo,
    ) -> Result<Vec<CipherView>, VaultError> {
        // Initialize SDK if needed
        if !self.sdk_bridge.is_crypto_initialized() {
            let kdf_config = self.load_kdf_config()?;
            let private_key = self.load_private_key()?;
            self.sdk_bridge.initialize_crypto(
                user_key,
                &account_info.email,
                &kdf_config,
                private_key.as_deref(),
            ).await?;
        }

        // Use SDK for decryption
        self.sdk_bridge.decrypt_ciphers(ciphers)
    }
}
```
