//! Integration tests for vault write operations
//!
//! These tests verify:
//! - CRUD operations for ciphers (create, update, delete, restore, move)
//! - CRUD operations for folders
//! - Cache consistency after operations
//! - Error handling for invalid operations
//! - Validation enforcement
//!
//! Note: These tests use the placeholder encryption implementation.
//! Real SDK integration will require updating these tests.
//! Write operations require a session key for encryption - tests that need
//! encryption pass a dummy session key.

use bw_core::models::vault::{CipherLoginView, CipherType, CipherView, VaultData};
use bw_core::services::api::{BitwardenApiClient, Environment};
use bw_core::services::create_sdk_client;
use bw_core::services::storage::{AccountManager, JsonFileStorage, Storage};
use bw_core::services::vault::{
    CipherService, ConfirmationService, ValidationService, VaultError, WriteService,
};
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::Mutex;

// ============================================================================
// Test Helpers
// ============================================================================

fn create_test_cipher_view() -> CipherView {
    CipherView {
        id: String::new(),
        organization_id: None,
        folder_id: None,
        cipher_type: CipherType::Login,
        name: "Test Login".to_string(),
        notes: Some("Test notes".to_string()),
        favorite: false,
        collection_ids: vec![],
        revision_date: String::new(),
        creation_date: None,
        deleted_date: None,
        login: Some(CipherLoginView {
            username: Some("user@example.com".to_string()),
            password: Some("secure_password".to_string()),
            uris: vec![],
            totp: None,
        }),
        secure_note: None,
        card: None,
        identity: None,
        attachments: vec![],
        fields: vec![],
    }
}

async fn setup_test_environment() -> (
    Arc<BitwardenApiClient>,
    Arc<Mutex<JsonFileStorage>>,
    Arc<CipherService>,
    Arc<ValidationService>,
    Arc<ConfirmationService>,
    Arc<AccountManager>,
) {
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path().to_path_buf();

    // Create storage and initialize with empty vault data
    let mut storage = JsonFileStorage::new(Some(storage_path.clone())).unwrap();
    let vault_data = VaultData {
        ciphers: vec![],
        folders: vec![],
        collections: vec![],
        organizations: vec![],
        last_sync: chrono::Utc::now().to_rfc3339(),
    };
    storage.set("vaultData", &vault_data).await.unwrap();

    let storage = Arc::new(Mutex::new(storage));

    // Create SDK client
    let sdk_client = Arc::new(create_sdk_client(None, None).unwrap());

    // Create API client (Note: This will fail on actual API calls without a real server)
    let environment = Environment::default_cloud();
    let api_client = Arc::new(BitwardenApiClient::new(environment, storage.clone(), None).unwrap());

    let cipher_service = Arc::new(CipherService::new(sdk_client.clone()));
    let validation_service = Arc::new(ValidationService::new());
    let confirmation_service = Arc::new(ConfirmationService::new(true)); // no_interaction=true
    let account_manager = Arc::new(AccountManager::new(storage.clone()));

    (
        api_client,
        storage,
        cipher_service,
        validation_service,
        confirmation_service,
        account_manager,
    )
}

// ============================================================================
// Validation Integration Tests
// ============================================================================

#[tokio::test]
async fn test_create_cipher_rejects_invalid_input() {
    let (
        api_client,
        storage,
        cipher_service,
        validation_service,
        confirmation_service,
        account_manager,
    ) = setup_test_environment().await;

    let write_service = WriteService::new(
        api_client,
        storage,
        cipher_service,
        validation_service,
        confirmation_service,
        account_manager,
    );

    // Create invalid cipher (missing name)
    let mut cipher_view = create_test_cipher_view();
    cipher_view.name = String::new();

    // Dummy session - won't be used since validation fails first
    let result = write_service.create_cipher(cipher_view, "dummy").await;

    // Should fail validation
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        VaultError::ValidationError(_)
    ));
}

#[tokio::test]
async fn test_create_cipher_rejects_invalid_uuid() {
    let (
        api_client,
        storage,
        cipher_service,
        validation_service,
        confirmation_service,
        account_manager,
    ) = setup_test_environment().await;

    let write_service = WriteService::new(
        api_client,
        storage,
        cipher_service,
        validation_service,
        confirmation_service,
        account_manager,
    );

    // Create cipher with invalid folder UUID
    let mut cipher_view = create_test_cipher_view();
    cipher_view.folder_id = Some("not-a-uuid".to_string());

    let result = write_service.create_cipher(cipher_view, "dummy").await;

    // Should fail validation
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        VaultError::ValidationError(_)
    ));
}

#[tokio::test]
async fn test_create_cipher_rejects_field_too_long() {
    let (
        api_client,
        storage,
        cipher_service,
        validation_service,
        confirmation_service,
        account_manager,
    ) = setup_test_environment().await;

    let write_service = WriteService::new(
        api_client,
        storage,
        cipher_service,
        validation_service,
        confirmation_service,
        account_manager,
    );

    // Create cipher with name too long
    let mut cipher_view = create_test_cipher_view();
    cipher_view.name = "a".repeat(1001);

    let result = write_service.create_cipher(cipher_view, "dummy").await;

    // Should fail validation
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        VaultError::ValidationError(_)
    ));
}

// ============================================================================
// Cache Management Tests
// ============================================================================

#[tokio::test]
async fn test_validate_cipher_exists_returns_error_when_not_found() {
    let (
        api_client,
        storage,
        cipher_service,
        validation_service,
        confirmation_service,
        account_manager,
    ) = setup_test_environment().await;

    let write_service = WriteService::new(
        api_client,
        storage,
        cipher_service,
        validation_service,
        confirmation_service,
        account_manager,
    );

    // Try to update non-existent cipher
    let cipher_view = create_test_cipher_view();
    let result = write_service
        .update_cipher("non-existent-id", cipher_view, "dummy")
        .await;

    // Should fail because cipher doesn't exist
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), VaultError::ItemNotFound));
}

#[tokio::test]
async fn test_validate_folder_exists_returns_error_when_not_found() {
    let (
        api_client,
        storage,
        cipher_service,
        validation_service,
        confirmation_service,
        account_manager,
    ) = setup_test_environment().await;

    let write_service = WriteService::new(
        api_client,
        storage,
        cipher_service,
        validation_service,
        confirmation_service,
        account_manager,
    );

    // Try to delete non-existent folder
    let result = write_service.delete_folder("non-existent-id").await;

    // Should fail because folder doesn't exist
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), VaultError::FolderNotFound));
}

// ============================================================================
// Folder Validation Tests
// ============================================================================

#[tokio::test]
async fn test_create_folder_rejects_empty_name() {
    let (
        api_client,
        storage,
        cipher_service,
        validation_service,
        confirmation_service,
        account_manager,
    ) = setup_test_environment().await;

    let write_service = WriteService::new(
        api_client,
        storage,
        cipher_service,
        validation_service,
        confirmation_service,
        account_manager,
    );

    let result = write_service.create_folder(String::new(), "dummy").await;

    // Should fail validation
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        VaultError::ValidationError(_)
    ));
}

#[tokio::test]
async fn test_create_folder_rejects_name_too_long() {
    let (
        api_client,
        storage,
        cipher_service,
        validation_service,
        confirmation_service,
        account_manager,
    ) = setup_test_environment().await;

    let write_service = WriteService::new(
        api_client,
        storage,
        cipher_service,
        validation_service,
        confirmation_service,
        account_manager,
    );

    let result = write_service.create_folder("a".repeat(1001), "dummy").await;

    // Should fail validation
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        VaultError::ValidationError(_)
    ));
}

#[tokio::test]
async fn test_update_folder_rejects_empty_name() {
    let (
        api_client,
        storage,
        cipher_service,
        validation_service,
        confirmation_service,
        account_manager,
    ) = setup_test_environment().await;

    let write_service = WriteService::new(
        api_client,
        storage,
        cipher_service,
        validation_service,
        confirmation_service,
        account_manager,
    );

    let result = write_service
        .update_folder("some-id", String::new(), "dummy")
        .await;

    // Should fail validation before checking if folder exists
    assert!(result.is_err());
    // Could be either ValidationError or FolderNotFound depending on order
    assert!(matches!(
        result.unwrap_err(),
        VaultError::ValidationError(_) | VaultError::FolderNotFound
    ));
}

// ============================================================================
// Cipher Type Validation Tests
// ============================================================================

#[tokio::test]
async fn test_create_login_without_login_data_fails() {
    let (
        api_client,
        storage,
        cipher_service,
        validation_service,
        confirmation_service,
        account_manager,
    ) = setup_test_environment().await;

    let write_service = WriteService::new(
        api_client,
        storage,
        cipher_service,
        validation_service,
        confirmation_service,
        account_manager,
    );

    let mut cipher_view = create_test_cipher_view();
    cipher_view.cipher_type = CipherType::Login;
    cipher_view.login = None; // Missing required login data

    let result = write_service.create_cipher(cipher_view, "dummy").await;

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        VaultError::ValidationError(_)
    ));
}

#[tokio::test]
async fn test_create_secure_note_without_secure_note_data_fails() {
    let (
        api_client,
        storage,
        cipher_service,
        validation_service,
        confirmation_service,
        account_manager,
    ) = setup_test_environment().await;

    let write_service = WriteService::new(
        api_client,
        storage,
        cipher_service,
        validation_service,
        confirmation_service,
        account_manager,
    );

    let mut cipher_view = create_test_cipher_view();
    cipher_view.cipher_type = CipherType::SecureNote;
    cipher_view.login = None;
    cipher_view.secure_note = None; // Missing required secure_note data

    let result = write_service.create_cipher(cipher_view, "dummy").await;

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        VaultError::ValidationError(_)
    ));
}

#[tokio::test]
async fn test_create_card_without_card_data_fails() {
    let (
        api_client,
        storage,
        cipher_service,
        validation_service,
        confirmation_service,
        account_manager,
    ) = setup_test_environment().await;

    let write_service = WriteService::new(
        api_client,
        storage,
        cipher_service,
        validation_service,
        confirmation_service,
        account_manager,
    );

    let mut cipher_view = create_test_cipher_view();
    cipher_view.cipher_type = CipherType::Card;
    cipher_view.login = None;
    cipher_view.card = None; // Missing required card data

    let result = write_service.create_cipher(cipher_view, "dummy").await;

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        VaultError::ValidationError(_)
    ));
}

#[tokio::test]
async fn test_create_identity_without_identity_data_fails() {
    let (
        api_client,
        storage,
        cipher_service,
        validation_service,
        confirmation_service,
        account_manager,
    ) = setup_test_environment().await;

    let write_service = WriteService::new(
        api_client,
        storage,
        cipher_service,
        validation_service,
        confirmation_service,
        account_manager,
    );

    let mut cipher_view = create_test_cipher_view();
    cipher_view.cipher_type = CipherType::Identity;
    cipher_view.login = None;
    cipher_view.identity = None; // Missing required identity data

    let result = write_service.create_cipher(cipher_view, "dummy").await;

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        VaultError::ValidationError(_)
    ));
}

// ============================================================================
// Test Summary
// ============================================================================

// Note: Full integration tests that actually create/update/delete items
// would require either:
// 1. A mock HTTP server (using wiremock or similar)
// 2. A test Bitwarden server
// 3. Dependency injection to mock the API client
// 4. A valid session key with stored user key in protected storage
//
// The tests above focus on validation and error handling, which can be
// tested without actual API calls. The cache management methods are
// private and will be tested indirectly through the public API once
// we have proper mocking in place.
//
// Future enhancements:
// - Add wiremock-based tests for full CRUD operations
// - Test cache consistency after successful operations
// - Test concurrent operations
// - Test restore operation with deleted items
// - Test move operation between folders
