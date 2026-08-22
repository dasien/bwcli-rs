//! Integration tests for the `Repository<Send>` adapter.
//!
//! The SDK's send CRUD reads and writes through this adapter. Without it
//! registered, the SDK silently falls back to an in-memory database that is
//! empty at the start of every CLI invocation — so `bw send list` would always
//! return nothing even after a sync.

use bitwarden_core::UserId;
use bitwarden_core::key_management::account_cryptographic_state::WrappedAccountCryptographicState;
use bitwarden_core::key_management::crypto::{InitUserCryptoMethod, InitUserCryptoRequest};
use bitwarden_crypto::{Kdf, SymmetricCryptoKey, SymmetricKeyAlgorithm, UserKey};
use bitwarden_send::{AuthType, Send, SendClientExt, SendId, SendTextView, SendType, SendView};
use bitwarden_state::repository::Repository;
use bw_core::services::storage::{AccountManager, JsonFileStorage, Storage, StorageKey};
use bw_core::services::{Client, JsonSendRepository, create_sdk_client};
use chrono::{Duration, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

const TEST_USER_ID: &str = "11111111-1111-4111-8111-111111111111";

async fn unlocked_client() -> Client {
    let client = create_sdk_client(None, None).unwrap();
    let user_key = SymmetricCryptoKey::make(SymmetricKeyAlgorithm::Aes256CbcHmac);
    let key_pair = UserKey::new(user_key.clone()).make_key_pair().unwrap();

    client
        .crypto()
        .initialize_user_crypto(InitUserCryptoRequest {
            user_id: Some(UserId::new_v4()),
            kdf_params: Kdf::PBKDF2 {
                iterations: std::num::NonZeroU32::new(600_000).unwrap(),
            },
            email: "test@example.com".to_string(),
            account_cryptographic_state: WrappedAccountCryptographicState::V1 {
                private_key: key_pair.private,
            },
            method: InitUserCryptoMethod::DecryptedKey {
                decrypted_user_key: user_key.to_base64().to_string(),
            },
            upgrade_token: None,
        })
        .await
        .unwrap();

    client
}

async fn setup() -> (
    JsonSendRepository,
    Arc<Mutex<JsonFileStorage>>,
    tempfile::TempDir,
) {
    let temp_dir = tempfile::tempdir().unwrap();
    let storage = Arc::new(Mutex::new(
        JsonFileStorage::new(Some(temp_dir.path().to_path_buf())).unwrap(),
    ));

    let account_manager = Arc::new(AccountManager::new(Arc::clone(&storage)));
    account_manager
        .register_account(TEST_USER_ID, "test@example.com")
        .await
        .unwrap();
    account_manager
        .set_active_user_id(TEST_USER_ID)
        .await
        .unwrap();

    let repo = JsonSendRepository::new(Arc::clone(&storage), account_manager);
    (repo, storage, temp_dir)
}

/// Build a real encrypted Send by round-tripping a view through the SDK.
async fn make_send(client: &Client, name: &str) -> (SendId, Send) {
    let view = SendView {
        id: None,
        access_id: None,
        name: name.to_string(),
        notes: None,
        key: None,
        new_password: None,
        has_password: false,
        r#type: SendType::Text,
        file: None,
        text: Some(SendTextView {
            text: Some(format!("secret for {name}")),
            hidden: false,
        }),
        max_access_count: None,
        access_count: 0,
        disabled: false,
        hide_email: false,
        revision_date: Utc::now(),
        deletion_date: Utc::now() + Duration::days(7),
        expiration_date: None,
        emails: vec![],
        auth_type: AuthType::None,
    };

    let send = client.sends().encrypt(view).expect("encrypt send");
    (SendId::new_v4(), send)
}

#[tokio::test]
async fn set_then_get_round_trips() {
    let client = unlocked_client().await;
    let (repo, _storage, _tmp) = setup().await;
    let (id, send) = make_send(&client, "First").await;

    repo.set(id, send).await.unwrap();

    let loaded = repo.get(id).await.unwrap().expect("send should be stored");
    let view = client.sends().decrypt(loaded).unwrap();
    assert_eq!(view.name, "First");
}

#[tokio::test]
async fn get_returns_none_for_unknown_id() {
    let (repo, _storage, _tmp) = setup().await;

    assert!(repo.get(SendId::new_v4()).await.unwrap().is_none());
}

#[tokio::test]
async fn list_returns_everything_stored() {
    let client = unlocked_client().await;
    let (repo, _storage, _tmp) = setup().await;

    for name in ["A", "B", "C"] {
        let (id, send) = make_send(&client, name).await;
        repo.set(id, send).await.unwrap();
    }

    assert_eq!(repo.list().await.unwrap().len(), 3);
}

#[tokio::test]
async fn remove_deletes_only_the_target() {
    let client = unlocked_client().await;
    let (repo, _storage, _tmp) = setup().await;

    let (keep_id, keep) = make_send(&client, "Keep").await;
    let (drop_id, drop) = make_send(&client, "Drop").await;
    repo.set(keep_id, keep).await.unwrap();
    repo.set(drop_id, drop).await.unwrap();

    repo.remove(drop_id).await.unwrap();

    assert!(repo.get(drop_id).await.unwrap().is_none());
    assert!(repo.get(keep_id).await.unwrap().is_some());
    assert_eq!(repo.list().await.unwrap().len(), 1);
}

#[tokio::test]
async fn replace_all_swaps_the_contents() {
    let client = unlocked_client().await;
    let (repo, _storage, _tmp) = setup().await;

    let (old_id, old) = make_send(&client, "Old").await;
    repo.set(old_id, old).await.unwrap();

    let (new_id, new) = make_send(&client, "New").await;
    repo.replace_all(vec![(new_id, new)]).await.unwrap();

    assert!(repo.get(old_id).await.unwrap().is_none());
    assert!(repo.get(new_id).await.unwrap().is_some());
}

#[tokio::test]
async fn remove_all_empties_the_repository() {
    let client = unlocked_client().await;
    let (repo, _storage, _tmp) = setup().await;

    let (id, send) = make_send(&client, "Only").await;
    repo.set(id, send).await.unwrap();

    repo.remove_all().await.unwrap();

    assert!(repo.list().await.unwrap().is_empty());
}

/// Sends must land under the TypeScript CLI's storage key so the state file
/// stays interoperable.
#[tokio::test]
async fn sends_are_stored_under_the_typescript_cli_key() {
    let client = unlocked_client().await;
    let (repo, storage, _tmp) = setup().await;

    let (id, send) = make_send(&client, "Keyed").await;
    repo.set(id, send).await.unwrap();

    let expected_key = StorageKey::UserSends.format(Some(TEST_USER_ID));
    assert_eq!(expected_key, format!("user_{TEST_USER_ID}_send_sends"));

    let s = storage.lock().await;
    let stored: HashMap<String, Send> = s.get(&expected_key).unwrap().expect("key should exist");

    assert!(stored.contains_key(&id.to_string()));
}

/// Persistence across adapter instances is the whole point: each CLI
/// invocation constructs a fresh one.
#[tokio::test]
async fn data_survives_a_new_repository_instance() {
    let client = unlocked_client().await;
    let (repo, storage, _tmp) = setup().await;

    let (id, send) = make_send(&client, "Persisted").await;
    repo.set(id, send).await.unwrap();

    let account_manager = Arc::new(AccountManager::new(Arc::clone(&storage)));
    let reopened = JsonSendRepository::new(Arc::clone(&storage), account_manager);

    assert!(reopened.get(id).await.unwrap().is_some());
}
