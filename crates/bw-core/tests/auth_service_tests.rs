//! Integration tests for AuthService
//!
//! Tests the authentication service with mock HTTP server and real storage

use bw_core::services::{
    api::{BitwardenApiClient, Environment},
    auth::{AuthError, AuthService},
    storage::{JsonFileStorage, Storage, StorageKey},
};
use bitwarden_crypto::{Kdf, MasterKey};
use secrecy::Secret;
use std::num::NonZeroU32;
use std::sync::Arc;
use tempfile::tempdir;
use tokio::sync::Mutex;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_string_contains, method, path},
};

/// Test credentials - these are only used in tests, not real credentials
const TEST_EMAIL: &str = "test@example.com";
const TEST_PASSWORD: &str = "test_password";
const TEST_KDF_ITERATIONS: u32 = 600000;

/// Generate a valid encrypted user key for testing
///
/// This creates a real encrypted user key that can be decrypted with the
/// given password/email/KDF combination.
fn generate_test_encrypted_user_key(password: &str, email: &str, iterations: u32) -> String {
    let kdf = Kdf::PBKDF2 {
        iterations: NonZeroU32::new(iterations).unwrap(),
    };
    let master_key = MasterKey::derive(password, email, &kdf).expect("Failed to derive master key");
    let (_user_key, encrypted_user_key) = master_key.make_user_key().expect("Failed to make user key");
    encrypted_user_key.to_string()
}

/// Helper to create test auth service with temp storage and mock API
/// Returns the TempDir to keep it alive for the duration of the test
async fn setup_test_auth_service(
    api_url: String,
) -> (AuthService, Arc<Mutex<JsonFileStorage>>, tempfile::TempDir) {
    let temp_dir = tempdir().unwrap();
    // JsonFileStorage expects a directory path, not a file path
    // It will create data.json inside this directory
    let storage_path = temp_dir.path().to_path_buf();

    let storage = Arc::new(Mutex::new(
        JsonFileStorage::new(Some(storage_path)).expect("Failed to create test storage"),
    ));

    let environment = Environment::from_base_url(&api_url).expect("Failed to create environment");
    let api_client = Arc::new(
        BitwardenApiClient::new(environment, Arc::clone(&storage), None)
            .expect("Failed to create API client"),
    );

    let auth_service = AuthService::new(Arc::clone(&storage), api_client);

    (auth_service, storage, temp_dir)
}

/// Setup standard mocks for password login tests
async fn setup_login_mocks(mock_server: &MockServer, encrypted_user_key: &str) {
    // Mock prelogin response (KDF config)
    Mock::given(method("POST"))
        .and(path("/identity/accounts/prelogin"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "kdf": 0,
            "kdfIterations": TEST_KDF_ITERATIONS,
        })))
        .mount(mock_server)
        .await;

    // Mock login response
    Mock::given(method("POST"))
        .and(path("/identity/connect/token"))
        .and(body_string_contains("grant_type=password"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "test_access_token",
            "expires_in": 3600,
            "token_type": "Bearer",
            "refresh_token": "test_refresh_token",
            "Key": encrypted_user_key,
            "Kdf": 0,
            "KdfIterations": TEST_KDF_ITERATIONS,
            "ResetMasterPassword": false,
        })))
        .mount(mock_server)
        .await;

    // Mock profile response
    Mock::given(method("GET"))
        .and(path("/api/accounts/profile"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "user_id_123",
            "name": "Test User",
            "email": TEST_EMAIL,
            "emailVerified": true,
            "premium": false,
            "securityStamp": "security_stamp_123",
        })))
        .mount(mock_server)
        .await;
}

#[tokio::test]
async fn test_login_with_password_success() {
    // Generate a valid encrypted user key for our test credentials
    let encrypted_user_key = generate_test_encrypted_user_key(TEST_PASSWORD, TEST_EMAIL, TEST_KDF_ITERATIONS);

    // Setup mock server
    let mock_server = MockServer::start().await;
    setup_login_mocks(&mock_server, &encrypted_user_key).await;

    // Create test service
    let (auth_service, storage, _temp_dir) = setup_test_auth_service(mock_server.uri()).await;

    // Execute login
    let result = auth_service
        .login_with_password(
            TEST_EMAIL,
            Secret::new(TEST_PASSWORD.to_string()),
            None,
            None,
        )
        .await;

    // Verify success
    assert!(result.is_ok(), "Login should succeed: {:?}", result.err());
    let login_result = result.unwrap();
    assert_eq!(login_result.email, TEST_EMAIL);
    assert_eq!(login_result.user_id, "user_id_123");
    assert!(!login_result.session_key.is_empty());

    // Verify storage persistence - check namespaced keys
    let storage = storage.lock().await;

    // Check access token is stored with namespaced key
    let access_token_key = StorageKey::UserAccessToken.format(Some("user_id_123"));
    let access_token: Option<String> = storage.get(&access_token_key).unwrap();
    assert!(access_token.is_some(), "Access token should be stored");

    // Check KDF config is stored with namespaced key
    let kdf_key = StorageKey::UserKdfConfig.format(Some("user_id_123"));
    let kdf_config: Option<serde_json::Value> = storage.get(&kdf_key).unwrap();
    assert!(kdf_config.is_some(), "KDF config should be stored");
}

#[tokio::test]
async fn test_login_with_password_invalid_credentials() {
    let mock_server = MockServer::start().await;

    // Mock prelogin response
    Mock::given(method("POST"))
        .and(path("/identity/accounts/prelogin"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "kdf": 0,
            "kdfIterations": TEST_KDF_ITERATIONS,
        })))
        .mount(&mock_server)
        .await;

    // Mock login failure (401)
    Mock::given(method("POST"))
        .and(path("/identity/connect/token"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "error": "invalid_grant",
            "error_description": "Username or password is incorrect"
        })))
        .mount(&mock_server)
        .await;

    let (auth_service, _storage, _temp_dir) = setup_test_auth_service(mock_server.uri()).await;

    // Execute login with wrong password
    let result = auth_service
        .login_with_password(
            TEST_EMAIL,
            Secret::new("wrong_password".to_string()),
            None,
            None,
        )
        .await;

    // Verify error
    assert!(result.is_err());
    match result.unwrap_err() {
        AuthError::InvalidCredentials { message: _ } => {
            // Expected error type
        }
        other => panic!("Expected InvalidCredentials error, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_login_with_api_key_success() {
    let mock_server = MockServer::start().await;

    // Mock API key login response (no Key field - API key login doesn't return encrypted user key)
    Mock::given(method("POST"))
        .and(path("/identity/connect/token"))
        .and(body_string_contains("grant_type=client_credentials"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "api_key_access_token",
            "expires_in": 3600,
            "token_type": "Bearer",
            "refresh_token": "api_key_refresh_token",
            "Kdf": 0,
            "KdfIterations": TEST_KDF_ITERATIONS,
            "ResetMasterPassword": false,
        })))
        .mount(&mock_server)
        .await;

    // Mock profile response
    Mock::given(method("GET"))
        .and(path("/api/accounts/profile"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "api_user_id",
            "name": "API User",
            "email": "api@example.com",
            "emailVerified": true,
            "premium": true,
            "securityStamp": "api_security_stamp",
        })))
        .mount(&mock_server)
        .await;

    let (auth_service, _storage, _temp_dir) = setup_test_auth_service(mock_server.uri()).await;

    // Execute API key login
    let result = auth_service
        .login_with_api_key(
            "user.api_client_id",
            Secret::new("api_client_secret".to_string()),
        )
        .await;

    // Verify success
    assert!(
        result.is_ok(),
        "API key login should succeed: {:?}",
        result.err()
    );
    let login_result = result.unwrap();
    assert_eq!(login_result.email, "api@example.com");
    assert_eq!(login_result.user_id, "api_user_id");
    assert!(!login_result.session_key.is_empty());
}

#[tokio::test]
async fn test_unlock_success() {
    // Generate a valid encrypted user key for our test credentials
    let encrypted_user_key = generate_test_encrypted_user_key(TEST_PASSWORD, TEST_EMAIL, TEST_KDF_ITERATIONS);

    let mock_server = MockServer::start().await;
    setup_login_mocks(&mock_server, &encrypted_user_key).await;

    let (auth_service, _storage, _temp_dir) = setup_test_auth_service(mock_server.uri()).await;

    // First login
    let password = Secret::new(TEST_PASSWORD.to_string());
    let login_result = auth_service
        .login_with_password(TEST_EMAIL, password.clone(), None, None)
        .await;
    assert!(login_result.is_ok(), "Login should succeed: {:?}", login_result.err());

    // Now test unlock with the same password
    let unlock_result = auth_service.unlock(password).await;

    // Verify unlock success
    assert!(
        unlock_result.is_ok(),
        "Unlock should succeed: {:?}",
        unlock_result.err()
    );
    let unlock_data = unlock_result.unwrap();
    assert!(!unlock_data.session_key.is_empty());
}

#[tokio::test]
async fn test_unlock_not_logged_in() {
    let mock_server = MockServer::start().await;
    let (auth_service, _storage, _temp_dir) = setup_test_auth_service(mock_server.uri()).await;

    // Try to unlock without logging in first
    let result = auth_service
        .unlock(Secret::new("password".to_string()))
        .await;

    // Should fail with NotLoggedIn error
    assert!(result.is_err());
    match result.unwrap_err() {
        AuthError::NotLoggedIn => {
            // Expected error type
        }
        other => panic!("Expected NotLoggedIn error, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_unlock_wrong_password() {
    // Generate encrypted user key with the correct password
    let encrypted_user_key = generate_test_encrypted_user_key(TEST_PASSWORD, TEST_EMAIL, TEST_KDF_ITERATIONS);

    let mock_server = MockServer::start().await;
    setup_login_mocks(&mock_server, &encrypted_user_key).await;

    let (auth_service, _storage, _temp_dir) = setup_test_auth_service(mock_server.uri()).await;

    // Login with correct password
    let login_result = auth_service
        .login_with_password(
            TEST_EMAIL,
            Secret::new(TEST_PASSWORD.to_string()),
            None,
            None,
        )
        .await;
    assert!(login_result.is_ok(), "Login should succeed: {:?}", login_result.err());

    // Try to unlock with wrong password
    let unlock_result = auth_service
        .unlock(Secret::new("wrong_password".to_string()))
        .await;

    // Should fail with InvalidPassword error
    assert!(unlock_result.is_err());
    match unlock_result.unwrap_err() {
        AuthError::InvalidPassword => {
            // Expected error type
        }
        other => panic!("Expected InvalidPassword error, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_lock() {
    // Generate a valid encrypted user key for our test credentials
    let encrypted_user_key = generate_test_encrypted_user_key(TEST_PASSWORD, TEST_EMAIL, TEST_KDF_ITERATIONS);

    let mock_server = MockServer::start().await;
    setup_login_mocks(&mock_server, &encrypted_user_key).await;

    let (auth_service, _storage, _temp_dir) = setup_test_auth_service(mock_server.uri()).await;

    // Login first (lock requires being logged in)
    let login_result = auth_service
        .login_with_password(
            TEST_EMAIL,
            Secret::new(TEST_PASSWORD.to_string()),
            None,
            None,
        )
        .await;
    assert!(login_result.is_ok(), "Login should succeed: {:?}", login_result.err());

    // Lock should succeed
    let result = auth_service.lock().await;
    assert!(result.is_ok(), "Lock should succeed: {:?}", result.err());
}

#[tokio::test]
async fn test_logout_success() {
    // Generate a valid encrypted user key for our test credentials
    let encrypted_user_key = generate_test_encrypted_user_key(TEST_PASSWORD, TEST_EMAIL, TEST_KDF_ITERATIONS);

    let mock_server = MockServer::start().await;
    setup_login_mocks(&mock_server, &encrypted_user_key).await;

    let (auth_service, storage, _temp_dir) = setup_test_auth_service(mock_server.uri()).await;

    // Login first
    let login_result = auth_service
        .login_with_password(
            TEST_EMAIL,
            Secret::new(TEST_PASSWORD.to_string()),
            None,
            None,
        )
        .await;
    assert!(login_result.is_ok(), "Login should succeed: {:?}", login_result.err());

    // Verify data is stored
    {
        let storage = storage.lock().await;
        let access_token_key = StorageKey::UserAccessToken.format(Some("user_id_123"));
        let token: Option<String> = storage.get(&access_token_key).unwrap();
        assert!(token.is_some(), "Access token should be stored after login");
    }

    // Execute logout
    let logout_result = auth_service.logout().await;
    assert!(logout_result.is_ok(), "Logout should succeed: {:?}", logout_result.err());

    // Verify tokens are cleared (set to null)
    {
        let storage = storage.lock().await;
        let access_token_key = StorageKey::UserAccessToken.format(Some("user_id_123"));
        let token: Option<serde_json::Value> = storage.get(&access_token_key).unwrap();
        // Token should be null (not removed, set to null per TypeScript CLI behavior)
        assert!(
            token == Some(serde_json::Value::Null) || token.is_none(),
            "Access token should be cleared after logout, got: {:?}",
            token
        );
    }
}

#[tokio::test]
async fn test_session_key_format() {
    // Generate a valid encrypted user key for our test credentials
    let encrypted_user_key = generate_test_encrypted_user_key(TEST_PASSWORD, TEST_EMAIL, TEST_KDF_ITERATIONS);

    let mock_server = MockServer::start().await;
    setup_login_mocks(&mock_server, &encrypted_user_key).await;

    let (auth_service, _storage, _temp_dir) = setup_test_auth_service(mock_server.uri()).await;

    let result = auth_service
        .login_with_password(
            TEST_EMAIL,
            Secret::new(TEST_PASSWORD.to_string()),
            None,
            None,
        )
        .await
        .expect("Login should succeed");

    // Verify session key is valid base64 (should decode successfully)
    use base64::Engine;
    let decoded = base64::engine::general_purpose::STANDARD.decode(&result.session_key);
    assert!(decoded.is_ok(), "Session key should be valid base64");

    // Session key should be 64 bytes (512 bits)
    assert_eq!(decoded.unwrap().len(), 64, "Session key should be 64 bytes");
}
