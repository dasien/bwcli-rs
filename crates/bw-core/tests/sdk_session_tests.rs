//! Session lifecycle through `bitwarden-unlock`, with real crypto.
//!
//! Replaces the old `keystore_init_tests`, which covered the hand-rolled
//! `KeyService`. The property under test is unchanged and is the one that
//! matters: after unlocking, vault crypto actually works. It used to be dead
//! code at runtime — nothing populated the SDK key store at all.

use bitwarden_core::Client;
use bitwarden_crypto::{Kdf, MasterKey, SymmetricCryptoKey};
use bitwarden_vault::{FolderView, VaultClientExt};
use bw_core::services::create_sdk_client_with_state;
use bw_core::services::sdk_session;
use chrono::Utc;
use std::num::NonZeroU32;
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;

const TEST_USER_ID: &str = "11111111-1111-4111-8111-111111111111";
const TEST_EMAIL: &str = "test@example.com";

/// A client over `dir`. Calling this twice models two CLI invocations sharing
/// one state directory, which is exactly how login-then-unlock behaves.
async fn client_in(dir: &Path) -> Client {
    create_sdk_client_with_state(None, None, dir.to_path_buf())
        .await
        .expect("client with state")
}

fn kdf() -> Kdf {
    Kdf::PBKDF2 {
        iterations: NonZeroU32::new(600_000).unwrap(),
    }
}

/// A user key plus the account private key wrapped by it, as the server would
/// hand back at login.
fn account_keys() -> (SymmetricCryptoKey, bitwarden_crypto::EncString) {
    let master_key = MasterKey::derive("test-password", TEST_EMAIL, &kdf()).unwrap();
    let (user_key, _encrypted) = master_key.make_user_key().unwrap();
    let key_pair = user_key.make_key_pair().unwrap();
    (user_key.0, key_pair.private)
}

/// Do what login does: initialize crypto, persist state, mint a session key.
async fn establish(client: &Client) -> String {
    let (user_key, private_key) = account_keys();

    sdk_session::initialize_crypto(
        client,
        TEST_USER_ID,
        TEST_EMAIL,
        kdf(),
        private_key.clone(),
        &user_key,
    )
    .await
    .expect("initialize crypto");

    sdk_session::persist_account_state(
        client,
        TEST_USER_ID,
        TEST_EMAIL,
        "https://api.bitwarden.com",
        "https://identity.bitwarden.com",
        private_key,
    )
    .await
    .expect("persist account state");

    sdk_session::mint_session_key(client)
        .await
        .expect("mint session key")
}

fn cipher_view(name: &str) -> bitwarden_vault::CipherView {
    use bitwarden_vault::{CipherRepromptType, CipherType, LoginView};

    bitwarden_vault::CipherView {
        id: Some(bitwarden_vault::CipherId::new_v4()),
        organization_id: None,
        folder_id: None,
        collection_ids: vec![],
        key: None,
        name: name.to_string(),
        notes: None,
        r#type: CipherType::Login,
        login: Some(LoginView {
            username: None,
            password: None,
            password_revision_date: None,
            uris: None,
            totp: None,
            autofill_on_page_load: None,
            fido2_credentials: None,
        }),
        identity: None,
        card: None,
        secure_note: None,
        ssh_key: None,
        bank_account: None,
        drivers_license: None,
        passport: None,
        favorite: false,
        reprompt: CipherRepromptType::None,
        organization_use_totp: false,
        edit: true,
        permissions: None,
        view_password: true,
        local_data: None,
        attachments: None,
        attachment_decryption_failures: None,
        fields: None,
        password_history: None,
        creation_date: Utc::now(),
        deleted_date: None,
        revision_date: Utc::now(),
        archived_date: None,
    }
}

fn folder(name: &str) -> FolderView {
    FolderView {
        id: None,
        name: name.to_string(),
        revision_date: Utc::now(),
    }
}

#[tokio::test]
async fn login_mints_a_session_key() {
    let dir = tempfile::tempdir().unwrap();
    let session = establish(&client_in(dir.path()).await).await;

    assert!(!session.is_empty());
    // The session key is a key envelope, not the old 64-byte enc+MAC blob.
    bitwarden_unlock::SessionKey::from_str(&session).expect("round-trips");
}

/// The payoff: a *fresh* client can unlock from the session key alone and do
/// real vault crypto.
#[tokio::test]
async fn a_new_invocation_can_unlock_and_decrypt() {
    let dir = tempfile::tempdir().unwrap();
    let session = establish(&client_in(dir.path()).await).await;

    let next = client_in(dir.path()).await;
    sdk_session::unlock_with_session(&next, &session)
        .await
        .expect("unlock with the session key");

    let encrypted = next
        .vault()
        .folders()
        .encrypt(folder("Personal"))
        .expect("encrypt");
    let decrypted = next.vault().folders().decrypt(encrypted).expect("decrypt");

    assert_eq!(decrypted.name, "Personal");
}

/// Regression guard: without unlocking, the key store is empty. This used to be
/// the CLI's actual runtime state, and decryption silently produced blanks.
#[tokio::test]
async fn a_new_invocation_cannot_decrypt_before_unlocking() {
    let dir = tempfile::tempdir().unwrap();
    establish(&client_in(dir.path()).await).await;

    let next = client_in(dir.path()).await;

    assert!(
        next.vault().folders().encrypt(folder("Personal")).is_err(),
        "an un-unlocked client must refuse vault crypto"
    );
}

/// `bw lock` deletes the sealed key, so outstanding sessions stop working.
#[tokio::test]
async fn locking_invalidates_the_session_key() {
    let dir = tempfile::tempdir().unwrap();
    let first = client_in(dir.path()).await;
    let session = establish(&first).await;

    sdk_session::invalidate_session(&first).await.expect("lock");

    let next = client_in(dir.path()).await;
    assert!(
        sdk_session::unlock_with_session(&next, &session)
            .await
            .is_err(),
        "a session key must not work after lock"
    );
}

#[tokio::test]
async fn a_malformed_session_key_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    establish(&client_in(dir.path()).await).await;

    let next = client_in(dir.path()).await;
    let err = sdk_session::unlock_with_session(&next, "not-a-session-key")
        .await
        .expect_err("should reject");

    assert!(
        format!("{err:#}").contains("not a valid session key"),
        "unexpected error: {err:#}"
    );
}

/// A non-UUID user id used to be silently dropped, which then failed deep in the
/// SDK with "Unable to initialize local user data key".
#[tokio::test]
async fn a_non_uuid_user_id_is_rejected_up_front() {
    let dir = tempfile::tempdir().unwrap();
    let client = client_in(dir.path()).await;
    let (user_key, private_key) = account_keys();

    let err = sdk_session::initialize_crypto(
        &client,
        "not-a-uuid",
        TEST_EMAIL,
        kdf(),
        private_key,
        &user_key,
    )
    .await
    .expect_err("should reject");

    assert!(
        format!("{err:#}").contains("not a valid user id"),
        "unexpected error: {err:#}"
    );
}

/// Regression: `UnlockClient::unlock` restores keys but does not set the
/// client's user id. Decryption worked, so reads looked fine, while *encryption*
/// failed with "Client User Id has not been set" — breaking every create and
/// edit. `unlock_with_session` now sets it from persisted state.
#[tokio::test]
async fn unlocking_sets_the_user_id_so_encryption_works() {
    let dir = tempfile::tempdir().unwrap();
    let session = establish(&client_in(dir.path()).await).await;

    let next = client_in(dir.path()).await;
    sdk_session::unlock_with_session(&next, &session)
        .await
        .expect("unlock");

    assert!(
        next.internal.get_user_id().is_some(),
        "the client must know its user id after unlocking"
    );

    // The operation that actually regressed: encryption records who encrypted
    // the item, so it needs the user id.
    next.vault()
        .ciphers()
        .encrypt(cipher_view("Encrypt me"))
        .await
        .expect("encryption should work after unlocking");
}
