//! Integration tests for SDK key store initialization
//!
//! The CLI derives and stores the user key itself, so the SDK client starts
//! every process with an empty key store. `KeyService::initialize_client_crypto`
//! is what loads the user key into it; without that call every
//! `client.vault()` crypto operation fails.
//!
//! These tests exercise that wiring end to end with real crypto (no mocks):
//! a genuine user key, a genuine RSA key pair wrapped by it, and a real
//! session-key-protected storage entry.

use bitwarden_crypto::{SymmetricCryptoKey, SymmetricKeyAlgorithm, UserKey};
use bitwarden_vault::{FolderView, VaultClientExt};
use bw_core::models::state::{KdfConfig, KdfType};
use bw_core::services::storage::{
    AccountManager, JsonFileStorage, Storage, StorageKey, encrypt_user_key, format_session_key,
    generate_session_key, make_protected_key, user_key_protected_storage_key,
};
use bw_core::services::{Client, KeyService, create_sdk_client};
use chrono::Utc;
use std::sync::Arc;
use tokio::sync::Mutex;

const TEST_USER_ID: &str = "11111111-1111-4111-8111-111111111111";
const TEST_EMAIL: &str = "test@example.com";

/// A fully populated fixture representing a logged-in, unlocked account.
struct Fixture {
    key_service: KeyService,
    client: Client,
    session_str: String,
    user_key: SymmetricCryptoKey,
    _temp_dir: tempfile::TempDir,
}

/// Build storage that looks like a successful `bw login` + `bw unlock`.
///
/// `with_private_key` controls whether `user_{id}_crypto_privateKey` is
/// persisted, so tests can cover the pre-existing-login case where it is
/// absent.
async fn setup(with_private_key: bool) -> Fixture {
    let temp_dir = tempfile::tempdir().unwrap();
    let storage = Arc::new(Mutex::new(
        JsonFileStorage::new(Some(temp_dir.path().to_path_buf())).unwrap(),
    ));

    let account_manager = Arc::new(AccountManager::new(Arc::clone(&storage)));
    account_manager
        .register_account(TEST_USER_ID, TEST_EMAIL)
        .await
        .unwrap();
    account_manager.set_active_user_id(TEST_USER_ID).await.unwrap();

    // Real user key, and a real RSA key pair wrapped by it. The SDK requires
    // the wrapped private key as the account cryptographic state.
    let user_key = SymmetricCryptoKey::make(SymmetricKeyAlgorithm::Aes256CbcHmac);
    let key_pair = UserKey::new(user_key.clone()).make_key_pair().unwrap();

    // Seal the user key under a session key, exactly as `bw unlock` does.
    let session_key = generate_session_key();
    let session_str = format_session_key(&session_key);
    let protected_user_key = encrypt_user_key(&user_key, &session_key).unwrap();

    {
        let mut s = storage.lock().await;
        s.set(
            &make_protected_key(&user_key_protected_storage_key(TEST_USER_ID)),
            &protected_user_key,
        )
        .await
        .unwrap();

        s.set(
            &StorageKey::UserKdfConfig.format(Some(TEST_USER_ID)),
            &KdfConfig {
                kdf_type: KdfType::PBKDF2SHA256,
                iterations: Some(600_000),
                memory: None,
                parallelism: None,
            },
        )
        .await
        .unwrap();

        if with_private_key {
            s.set(
                &StorageKey::UserPrivateKey.format(Some(TEST_USER_ID)),
                &key_pair.private.to_string(),
            )
            .await
            .unwrap();
        }

        s.flush().await.unwrap();
    }

    Fixture {
        key_service: KeyService::new(Arc::clone(&storage), account_manager),
        client: create_sdk_client(None, None).unwrap(),
        session_str,
        user_key,
        _temp_dir: temp_dir,
    }
}

/// Regression guard: this is the bug. A freshly created SDK client has an
/// empty key store, so vault crypto fails no matter what is in storage.
#[tokio::test]
async fn vault_crypto_fails_before_key_store_is_initialized() {
    let f = setup(true).await;

    let result = f.client.vault().folders().encrypt(FolderView {
        id: None,
        name: "Personal".to_string(),
        revision_date: Utc::now(),
    });

    assert!(
        result.is_err(),
        "expected an uninitialized key store to reject encryption; if this now \
         succeeds the SDK is seeding a user key on its own and this test is \
         no longer a meaningful guard"
    );
}

#[tokio::test]
async fn initialize_client_crypto_loads_the_user_key() {
    let f = setup(true).await;

    f.key_service
        .initialize_client_crypto(&f.client, &f.session_str)
        .await
        .expect("key store initialization should succeed");

    // The key store should now hold exactly the user key we sealed.
    let loaded = f
        .client
        .crypto()
        .get_user_encryption_key()
        .await
        .expect("user key should be readable from the key store");

    assert_eq!(loaded.to_string(), f.user_key.to_base64().to_string());
}

/// The actual payoff: the vault crypto path works after initialization.
#[tokio::test]
async fn vault_crypto_round_trips_after_initialization() {
    let f = setup(true).await;

    f.key_service
        .initialize_client_crypto(&f.client, &f.session_str)
        .await
        .unwrap();

    let encrypted = f
        .client
        .vault()
        .folders()
        .encrypt(FolderView {
            id: None,
            name: "Personal".to_string(),
            revision_date: Utc::now(),
        })
        .expect("encrypt should succeed with an initialized key store");

    let decrypted = f
        .client
        .vault()
        .folders()
        .decrypt(encrypted)
        .expect("decrypt should succeed with an initialized key store");

    assert_eq!(decrypted.name, "Personal");
}

/// Accounts that logged in before the private key was persisted must fail with
/// an actionable error rather than an opaque crypto failure.
#[tokio::test]
async fn missing_private_key_reports_actionable_error() {
    let f = setup(false).await;

    let err = f
        .key_service
        .initialize_client_crypto(&f.client, &f.session_str)
        .await
        .expect_err("initialization should fail without the account private key");

    assert!(
        matches!(err, bw_core::services::KeyServiceError::PrivateKeyNotFound),
        "expected PrivateKeyNotFound, got: {err:?}"
    );
}

#[tokio::test]
async fn invalid_session_key_is_rejected() {
    let f = setup(true).await;

    let err = f
        .key_service
        .initialize_client_crypto(&f.client, "not-a-valid-session-key")
        .await
        .expect_err("initialization should reject a malformed session key");

    assert!(
        matches!(
            err,
            bw_core::services::KeyServiceError::InvalidSessionKey(_)
                | bw_core::services::KeyServiceError::DecryptionFailed(_)
        ),
        "expected a session-key error, got: {err:?}"
    );
}
