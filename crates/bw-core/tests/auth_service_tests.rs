//! Integration tests for AuthService
//!
//! Tests the authentication service with mock HTTP server and real storage

use bitwarden_core::client::persisted_state::USER_LOGIN_METHOD;
use bitwarden_crypto::{Kdf, MasterKey};
use bw_core::services::sdk_session;
use bw_core::services::{
    api::{BitwardenApiClient, Environment},
    auth::{AuthError, AuthService},
    storage::{JsonFileStorage, Storage, StorageKey},
};
use secrecy::Secret;
use std::num::NonZeroU32;
use std::sync::Arc;
use tempfile::tempdir;
use tokio::sync::Mutex;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_string_contains, method, path},
};

/// Build an unsigned JWT carrying the identity claims login now reads.
///
/// The CLI no longer calls `GET /accounts/profile`; it takes the user id and
/// email from the access token's own claims, so test tokens have to be
/// real-shaped JWTs.
fn test_jwt(sub: &str, email: &str) -> String {
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"HS256","typ":"JWT"}"#);
    let payload = serde_json::json!({
        "exp": 1_900_000_000u64,
        "sub": sub,
        "email": email,
        "scope": ["api", "offline_access"],
    })
    .to_string();
    format!("{header}.{}.signature", URL_SAFE_NO_PAD.encode(payload))
}

/// Test credentials - these are only used in tests, not real credentials
const TEST_EMAIL: &str = "test@example.com";
/// Must be a real UUID: the SDK parses it into a `UserId`.
const TEST_USER_ID: &str = "11111111-1111-4111-8111-111111111111";
const TEST_PASSWORD: &str = "test_password";
const TEST_KDF_ITERATIONS: u32 = 600000;

/// Generate a valid encrypted user key for testing
///
/// This creates a real encrypted user key that can be decrypted with the
/// given password/email/KDF combination.
/// `(encrypted_user_key, wrapped_private_key)` for a test account.
///
/// Login now initializes the SDK key store and mints the session key through
/// `bitwarden-unlock`, which needs the account's wrapped private key — so mocks
/// have to supply a real one, not just the user key.
fn test_account_keys(password: &str, email: &str, iterations: u32) -> (String, String) {
    let kdf = Kdf::PBKDF2 {
        iterations: NonZeroU32::new(iterations).unwrap(),
    };
    let master_key = MasterKey::derive(password, email, &kdf).expect("Failed to derive master key");
    let (user_key, encrypted_user_key) =
        master_key.make_user_key().expect("Failed to make user key");

    // `make_user_key` already hands back a `UserKey`.
    let key_pair = user_key.make_key_pair().expect("Failed to make key pair");

    (encrypted_user_key.to_string(), key_pair.private.to_string())
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
        BitwardenApiClient::new(environment, None)
            .expect("Failed to create API client"),
    );

    // Login and unlock now mint the session through the SDK, so the service
    // needs a client with state to write the sealed key into.
    let temp_state = temp_dir.path().to_path_buf();
    let sdk = Arc::new(
        bw_core::services::create_sdk_client_with_state(
            Some(api_url.clone()),
            Some(api_url.clone()),
            temp_state,
        )
        .await
        .unwrap(),
    );

    let auth_service = AuthService::new(Arc::clone(&storage), api_client, sdk);

    (auth_service, storage, temp_dir)
}

/// A second `AuthService` over the same state directory.
///
/// Each CLI invocation gets a fresh client, so login and unlock never share
/// one. Reusing a single client would try to initialize crypto twice and fail
/// with "Cryptography Initialization error" — an artifact of the test, not of
/// the code under test.
async fn another_invocation(
    dir: &std::path::Path,
    api_url: String,
) -> (AuthService, Arc<Mutex<JsonFileStorage>>) {
    let storage = Arc::new(Mutex::new(
        JsonFileStorage::new(Some(dir.to_path_buf())).expect("Failed to create test storage"),
    ));

    let environment = Environment::from_base_url(&api_url).expect("Failed to create environment");
    let api_client = Arc::new(
        BitwardenApiClient::new(environment, None)
            .expect("Failed to create API client"),
    );

    let sdk = Arc::new(
        bw_core::services::create_sdk_client_with_state(
            Some(api_url.clone()),
            Some(api_url),
            dir.to_path_buf(),
        )
        .await
        .unwrap(),
    );

    (
        AuthService::new(Arc::clone(&storage), api_client, sdk),
        storage,
    )
}

/// Setup standard mocks for password login tests
/// A client over an existing state directory, i.e. the next `bw` invocation.
async fn next_invocation_client(dir: &std::path::Path) -> bitwarden_core::Client {
    bw_core::services::create_sdk_client_with_state(None, None, dir.to_path_buf())
        .await
        .unwrap()
}

async fn setup_login_mocks(
    mock_server: &MockServer,
    encrypted_user_key: &str,
    private_key: &str,
) {
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
            "access_token": test_jwt(TEST_USER_ID, TEST_EMAIL),
            "expires_in": 3600,
            "token_type": "Bearer",
            "refresh_token": "test_refresh_token",
            "Key": encrypted_user_key,
            "PrivateKey": private_key,
            "Kdf": 0,
            "KdfIterations": TEST_KDF_ITERATIONS,
            "ResetMasterPassword": false,
        })))
        .mount(mock_server)
        .await;

}

#[tokio::test]
async fn test_login_with_password_success() {
    // Generate a valid encrypted user key for our test credentials
    // One generation: `Key` and `PrivateKey` must be wrapped by the *same*
    // user key or crypto initialization fails.
    let (encrypted_user_key, private_key) =
        test_account_keys(TEST_PASSWORD, TEST_EMAIL, TEST_KDF_ITERATIONS);

    // Setup mock server
    let mock_server = MockServer::start().await;
    setup_login_mocks(&mock_server, &encrypted_user_key, &private_key).await;

    // Create test service
    let (auth_service, storage, temp_dir) = setup_test_auth_service(mock_server.uri()).await;

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
    assert_eq!(login_result.user_id, TEST_USER_ID);
    assert!(!login_result.session_key.is_empty());

    // Tokens must be readable by a *later* invocation, which is the only thing
    // that matters: that is how the token handler finds them. A fresh client
    // over the same directory models exactly that.
    assert!(
        sdk_session::is_authenticated(&next_invocation_client(temp_dir.path()).await).await,
        "the SDK should hold authentication tokens after login"
    );

    // Check KDF config is stored with namespaced key
    let storage = storage.lock().await;
    let kdf_key = StorageKey::UserKdfConfig.format(Some(TEST_USER_ID));
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
            "access_token": test_jwt(TEST_USER_ID, "api@example.com"),
            "expires_in": 3600,
            "token_type": "Bearer",
            "refresh_token": "api_key_refresh_token",
            "Kdf": 0,
            "KdfIterations": TEST_KDF_ITERATIONS,
            "ResetMasterPassword": false,
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
    assert_eq!(login_result.user_id, TEST_USER_ID);
    // API-key login returns no user key, so there is nothing to seal and no
    // session to mint. It previously handed out a session key that could not
    // unlock anything.
    assert!(
        login_result.session_key.is_empty(),
        "api-key login should not mint a session key"
    );
}

#[tokio::test]
async fn test_unlock_success() {
    // Generate a valid encrypted user key for our test credentials
    // One generation: `Key` and `PrivateKey` must be wrapped by the *same*
    // user key or crypto initialization fails.
    let (encrypted_user_key, private_key) =
        test_account_keys(TEST_PASSWORD, TEST_EMAIL, TEST_KDF_ITERATIONS);

    let mock_server = MockServer::start().await;
    setup_login_mocks(&mock_server, &encrypted_user_key, &private_key).await;

    let (auth_service, _storage, _temp_dir) = setup_test_auth_service(mock_server.uri()).await;

    // First login
    let password = Secret::new(TEST_PASSWORD.to_string());
    let login_result = auth_service
        .login_with_password(TEST_EMAIL, password.clone(), None, None)
        .await;
    assert!(
        login_result.is_ok(),
        "Login should succeed: {:?}",
        login_result.err()
    );

    // Unlock happens in a separate CLI invocation, so use a fresh service over
    // the same state directory.
    let (unlock_service, _storage2) =
        another_invocation(_temp_dir.path(), mock_server.uri()).await;
    let unlock_result = unlock_service.unlock(password).await;

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
    // One generation: `Key` and `PrivateKey` must be wrapped by the *same*
    // user key or crypto initialization fails.
    let (encrypted_user_key, private_key) =
        test_account_keys(TEST_PASSWORD, TEST_EMAIL, TEST_KDF_ITERATIONS);

    let mock_server = MockServer::start().await;
    setup_login_mocks(&mock_server, &encrypted_user_key, &private_key).await;

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
    assert!(
        login_result.is_ok(),
        "Login should succeed: {:?}",
        login_result.err()
    );

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
    // One generation: `Key` and `PrivateKey` must be wrapped by the *same*
    // user key or crypto initialization fails.
    let (encrypted_user_key, private_key) =
        test_account_keys(TEST_PASSWORD, TEST_EMAIL, TEST_KDF_ITERATIONS);

    let mock_server = MockServer::start().await;
    setup_login_mocks(&mock_server, &encrypted_user_key, &private_key).await;

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
    assert!(
        login_result.is_ok(),
        "Login should succeed: {:?}",
        login_result.err()
    );

    // Lock should succeed
    let result = auth_service.lock().await;
    assert!(result.is_ok(), "Lock should succeed: {:?}", result.err());
}

#[tokio::test]
async fn test_logout_success() {
    // Generate a valid encrypted user key for our test credentials
    // One generation: `Key` and `PrivateKey` must be wrapped by the *same*
    // user key or crypto initialization fails.
    let (encrypted_user_key, private_key) =
        test_account_keys(TEST_PASSWORD, TEST_EMAIL, TEST_KDF_ITERATIONS);

    let mock_server = MockServer::start().await;
    setup_login_mocks(&mock_server, &encrypted_user_key, &private_key).await;

    let (auth_service, _storage, temp_dir) = setup_test_auth_service(mock_server.uri()).await;

    // Login first
    let login_result = auth_service
        .login_with_password(
            TEST_EMAIL,
            Secret::new(TEST_PASSWORD.to_string()),
            None,
            None,
        )
        .await;
    assert!(
        login_result.is_ok(),
        "Login should succeed: {:?}",
        login_result.err()
    );

    assert!(
        sdk_session::is_authenticated(&next_invocation_client(temp_dir.path()).await).await,
        "the SDK should hold authentication tokens after login"
    );

    // Execute logout
    let logout_result = auth_service.logout().await;
    assert!(
        logout_result.is_ok(),
        "Logout should succeed: {:?}",
        logout_result.err()
    );

    // Logout must clear the tokens *and* the login method — an API-key login
    // method left behind would let the token handler mint fresh tokens from the
    // stored client secret.
    let after = next_invocation_client(temp_dir.path()).await;
    assert!(
        !sdk_session::is_authenticated(&after).await,
        "authentication tokens should be gone after logout"
    );
    assert!(
        after
            .platform()
            .state()
            .setting(USER_LOGIN_METHOD)
            .unwrap()
            .get()
            .await
            .unwrap()
            .is_none(),
        "the login method should be gone after logout"
    );
}

#[tokio::test]
async fn test_session_key_format() {
    // Generate a valid encrypted user key for our test credentials
    // One generation: `Key` and `PrivateKey` must be wrapped by the *same*
    // user key or crypto initialization fails.
    let (encrypted_user_key, private_key) =
        test_account_keys(TEST_PASSWORD, TEST_EMAIL, TEST_KDF_ITERATIONS);

    let mock_server = MockServer::start().await;
    setup_login_mocks(&mock_server, &encrypted_user_key, &private_key).await;

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

    // The session key is now minted by `bitwarden-unlock`, so it is a
    // SymmetricKeyEnvelope key rather than the old 64-byte enc+MAC blob.
    // Assert the property that matters: BW_SESSION round-trips back into a
    // usable SessionKey.
    assert!(
        !result.session_key.is_empty(),
        "login should mint a session key"
    );

    use std::str::FromStr;
    bitwarden_unlock::SessionKey::from_str(&result.session_key)
        .expect("BW_SESSION should parse back into a session key");
}
