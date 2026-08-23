//! The one-time `data.json` -> SQLite login carry-over.
//!
//! The fixtures here are shaped after real state, including its awkward parts:
//! `bw logout` nulls the token keys rather than removing them, the TypeScript
//! CLI writes `null` for fields it keeps only in memory, and an install can end
//! up with several accounts in the registry and no active one.

use bitwarden_core::client::login_method::UserLoginMethod;
use bitwarden_core::client::persisted_state::{
    ACCOUNT_CRYPTO_STATE, AUTHENTICATION_TOKENS, USER_EMAIL, USER_ID, USER_LOGIN_METHOD,
};
use bw_core::services::{create_sdk_client_with_state, open_state};
use bw_core::services::sdk_session;
use bw_core::services::state_import;
use bw_core::services::storage::{JsonFileStorage, Storage};
use std::path::Path;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::Mutex;

const USER: &str = "73d54ce4-b9f5-4bf1-a921-b3ad0104a632";
const OTHER_USER: &str = "88d2a7fb-411d-4a79-8914-af04011fa633";
const EMAIL: &str = "test@example.com";

/// A structurally valid `EncString` (AES-256-CBC-HMAC), so the parse is really
/// exercised. Deterministic bytes; it decrypts to nothing and guards nothing.
const PRIVATE_KEY: &str = "2.AAECAwQFBgcICQoLDA0ODw==|AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8=|AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8=";

async fn write_data_json(dir: &Path, entries: Vec<(String, serde_json::Value)>) -> Arc<Mutex<JsonFileStorage>> {
    let mut storage = JsonFileStorage::new(Some(dir.to_path_buf())).unwrap();
    for (key, value) in entries {
        storage.set(&key, &value).await.unwrap();
    }
    storage.flush().await.unwrap();
    Arc::new(Mutex::new(storage))
}

fn accounts(ids: &[&str]) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    for id in ids {
        map.insert(
            id.to_string(),
            serde_json::json!({ "email": EMAIL, "emailVerified": true }),
        );
    }
    serde_json::Value::Object(map)
}

/// A logged-in `data.json`: active account, tokens, private key, KDF config.
fn full_login() -> Vec<(String, serde_json::Value)> {
    vec![
        (
            "global_account_activeAccountId".into(),
            serde_json::json!(USER),
        ),
        ("global_account_accounts".into(), accounts(&[USER])),
        (
            format!("user_{USER}_token_accessToken"),
            serde_json::json!("stale-access-token"),
        ),
        (
            format!("user_{USER}_token_refreshToken"),
            serde_json::json!("the-refresh-token"),
        ),
        (
            format!("user_{USER}_crypto_privateKey"),
            serde_json::json!(PRIVATE_KEY),
        ),
        (
            format!("user_{USER}_kdfConfig_kdfConfig"),
            serde_json::json!({ "kdfType": 0, "iterations": 600000 }),
        ),
    ]
}

async fn client(dir: &Path) -> bitwarden_core::Client {
    let registry = open_state(dir.to_path_buf()).await.unwrap();
    create_sdk_client_with_state(None, None, registry)
}

#[tokio::test]
async fn carries_a_full_login_over() {
    let dir = TempDir::new().unwrap();
    let storage = write_data_json(dir.path(), full_login()).await;
    let client = client(dir.path()).await;

    assert!(!sdk_session::is_authenticated(&client).await);

    let imported = state_import::migrate(&client, &storage).await.unwrap();
    assert!(imported.identity);
    assert!(imported.crypto_state);
    assert!(imported.tokens);
    assert!(imported.login_method);

    let state = client.platform().state();
    assert_eq!(
        state.setting(USER_ID).unwrap().get().await.unwrap().unwrap().to_string(),
        USER
    );
    assert_eq!(
        state.setting(USER_EMAIL).unwrap().get().await.unwrap().unwrap(),
        EMAIL
    );
    assert!(
        state.setting(ACCOUNT_CRYPTO_STATE).unwrap().get().await.unwrap().is_some(),
        "the account private key must come over, or unlock cannot work"
    );
    assert!(sdk_session::is_authenticated(&client).await);
}

/// The carried-over token is recorded as already expired, so the first
/// authenticated request renews instead of sending a token the server has
/// almost certainly already rejected.
#[tokio::test]
async fn the_carried_over_token_is_treated_as_expired() {
    let dir = TempDir::new().unwrap();
    let storage = write_data_json(dir.path(), full_login()).await;
    let client = client(dir.path()).await;

    state_import::migrate(&client, &storage).await.unwrap();

    let tokens = client
        .platform()
        .state()
        .setting(AUTHENTICATION_TOKENS)
        .unwrap()
        .get()
        .await
        .unwrap()
        .unwrap();
    assert_eq!(tokens.refresh_token.as_deref(), Some("the-refresh-token"));
    assert!(
        tokens.expires_on <= chrono::Utc::now().timestamp(),
        "expected an already-expired token, got expires_on={}",
        tokens.expires_on
    );
}

/// Renewal sends the `client_id` from the login method, so the carry-over has to
/// write one. Getting this wrong is the bug that broke every refresh before.
#[tokio::test]
async fn writes_a_login_method_renewal_can_use() {
    let dir = TempDir::new().unwrap();
    let storage = write_data_json(dir.path(), full_login()).await;
    let client = client(dir.path()).await;

    state_import::migrate(&client, &storage).await.unwrap();

    let method = client
        .platform()
        .state()
        .setting(USER_LOGIN_METHOD)
        .unwrap()
        .get()
        .await
        .unwrap()
        .unwrap();
    match method {
        UserLoginMethod::Username { client_id, email, .. } => {
            assert_eq!(client_id, "cli");
            assert_eq!(email, EMAIL);
        }
        other => panic!("expected a password login method, got {other:?}"),
    }
}

/// A refresh token alone is enough. An older install may never have written an
/// access token to disk at all, and the handler only needs the refresh token.
#[tokio::test]
async fn a_refresh_token_alone_is_enough() {
    let dir = TempDir::new().unwrap();
    let mut entries = full_login();
    entries.retain(|(k, _)| !k.ends_with("_token_accessToken"));
    let storage = write_data_json(dir.path(), entries).await;
    let client = client(dir.path()).await;

    let imported = state_import::migrate(&client, &storage).await.unwrap();
    assert!(imported.tokens);
    assert!(sdk_session::is_authenticated(&client).await);
}

/// `bw logout` nulls the token keys rather than removing them, so a logged-out
/// install must not look migratable.
#[tokio::test]
async fn nulled_tokens_are_not_a_login() {
    let dir = TempDir::new().unwrap();
    let mut entries = full_login();
    for (key, value) in entries.iter_mut() {
        if key.contains("_token_") {
            *value = serde_json::Value::Null;
        }
    }
    let storage = write_data_json(dir.path(), entries).await;
    let client = client(dir.path()).await;

    let imported = state_import::migrate(&client, &storage).await.unwrap();
    assert!(!imported.tokens, "nulled tokens are not a login");
    assert!(!sdk_session::is_authenticated(&client).await);
}

/// A cleared `activeAccountId` with exactly one token-holding account is still
/// recoverable, and refusing to look would be needlessly strict.
#[tokio::test]
async fn recovers_the_sole_token_holder_with_no_active_account() {
    let dir = TempDir::new().unwrap();
    let mut entries = full_login();
    entries.retain(|(k, _)| k != "global_account_activeAccountId");
    entries.push(("global_account_accounts".into(), accounts(&[USER, OTHER_USER])));
    let storage = write_data_json(dir.path(), entries).await;
    let client = client(dir.path()).await;

    let imported = state_import::migrate(&client, &storage).await.unwrap();
    assert!(imported.tokens);
    assert_eq!(
        client
            .platform()
            .state()
            .setting(USER_ID)
            .unwrap()
            .get()
            .await
            .unwrap()
            .unwrap()
            .to_string(),
        USER
    );
}

/// Two token-holding accounts and no active one: there is no right answer, so
/// do nothing rather than pick an account the user did not mean.
#[tokio::test]
async fn refuses_to_guess_between_two_logins() {
    let dir = TempDir::new().unwrap();
    let mut entries = full_login();
    entries.retain(|(k, _)| k != "global_account_activeAccountId");
    entries.push(("global_account_accounts".into(), accounts(&[USER, OTHER_USER])));
    entries.push((
        format!("user_{OTHER_USER}_token_refreshToken"),
        serde_json::json!("another-refresh-token"),
    ));
    let storage = write_data_json(dir.path(), entries).await;
    let client = client(dir.path()).await;

    assert!(state_import::migrate(&client, &storage).await.is_err());
    assert!(!sdk_session::is_authenticated(&client).await);
}

/// Running twice must not change anything the second time — that is what makes
/// it safe to call on every startup instead of gating it behind a flag.
#[tokio::test]
async fn is_idempotent() {
    let dir = TempDir::new().unwrap();
    let storage = write_data_json(dir.path(), full_login()).await;

    {
        let client = client(dir.path()).await;
        state_import::migrate(&client, &storage).await.unwrap();
    }

    // Stand in for a later, real login so we can prove it is not clobbered.
    {
        let client = client(dir.path()).await;
        sdk_session::persist_tokens(
            &client,
            UserLoginMethod::Username {
                client_id: "cli".into(),
                email: EMAIL.into(),
                kdf: bitwarden_crypto::Kdf::default_pbkdf2(),
            },
            "fresh-access-token",
            Some("fresh-refresh-token"),
            3600,
        )
        .await
        .unwrap();
    }

    let client = client(dir.path()).await;
    state_import::migrate_if_needed(&client, &storage).await;

    let tokens = client
        .platform()
        .state()
        .setting(AUTHENTICATION_TOKENS)
        .unwrap()
        .get()
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        tokens.access_token, "fresh-access-token",
        "a second run must not overwrite a newer login"
    );
}

/// Nothing to carry over is the normal case for a fresh install, and must be
/// quiet rather than an error the user sees.
#[tokio::test]
async fn an_empty_data_json_is_a_no_op() {
    let dir = TempDir::new().unwrap();
    let storage = write_data_json(dir.path(), vec![]).await;
    let client = client(dir.path()).await;

    assert!(state_import::migrate(&client, &storage).await.is_err());
    state_import::migrate_if_needed(&client, &storage).await;
    assert!(!sdk_session::is_authenticated(&client).await);
}

/// Self-hosted URLs must come over, or renewal would target Bitwarden cloud
/// with a self-hosted refresh token.
#[tokio::test]
async fn carries_self_hosted_urls_over() {
    let dir = TempDir::new().unwrap();
    let mut entries = full_login();
    entries.push((
        "global_config_byServer".into(),
        serde_json::json!({
            "https://vault.example.com": {
                "environment": {
                    "api": "https://vault.example.com/api",
                    "identity": "https://vault.example.com/identity"
                }
            }
        }),
    ));
    let storage = write_data_json(dir.path(), entries).await;
    let client = client(dir.path()).await;

    let imported = state_import::migrate(&client, &storage).await.unwrap();
    assert!(imported.base_urls);

    let urls = client
        .platform()
        .state()
        .setting(bitwarden_core::client::persisted_state::BASE_URLS)
        .unwrap()
        .get()
        .await
        .unwrap()
        .unwrap();
    assert_eq!(urls.api_url, "https://vault.example.com/api");
    assert_eq!(urls.identity_url, "https://vault.example.com/identity");
}

/// When SDK state already names a user — the normal case for an install that
/// got its crypto state migrated but not its tokens — that user decides which
/// `data.json` account to read. Two accounts holding tokens is then no longer
/// ambiguous.
#[tokio::test]
async fn an_already_known_user_id_breaks_the_tie() {
    let dir = TempDir::new().unwrap();
    let mut entries = full_login();
    entries.retain(|(k, _)| k != "global_account_activeAccountId");
    entries.push(("global_account_accounts".into(), accounts(&[USER, OTHER_USER])));
    entries.push((
        format!("user_{OTHER_USER}_token_refreshToken"),
        serde_json::json!("another-refresh-token"),
    ));
    let storage = write_data_json(dir.path(), entries).await;

    // Stand in for state that step 1 and 2 already migrated.
    {
        let client = client(dir.path()).await;
        client
            .platform()
            .state()
            .setting(USER_ID)
            .unwrap()
            .update(USER.parse().unwrap())
            .await
            .unwrap();
    }

    let client = client(dir.path()).await;
    let imported = state_import::migrate(&client, &storage).await.unwrap();
    assert!(imported.tokens);

    let tokens = client
        .platform()
        .state()
        .setting(AUTHENTICATION_TOKENS)
        .unwrap()
        .get()
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        tokens.refresh_token.as_deref(),
        Some("the-refresh-token"),
        "must take the known user's token, not the other account's"
    );
}

/// The known user id must *override* `activeAccountId`, not defer to it.
/// Carrying account B's tokens onto account A's cryptographic state would be a
/// worse outcome than not migrating at all.
#[tokio::test]
async fn the_known_user_id_wins_over_a_conflicting_active_account() {
    let dir = TempDir::new().unwrap();
    let mut entries = full_login();
    entries.push((
        "global_account_activeAccountId".into(),
        serde_json::json!(OTHER_USER),
    ));
    entries.push(("global_account_accounts".into(), accounts(&[USER, OTHER_USER])));
    entries.push((
        format!("user_{OTHER_USER}_token_refreshToken"),
        serde_json::json!("wrong-account-token"),
    ));
    let storage = write_data_json(dir.path(), entries).await;

    {
        let client = client(dir.path()).await;
        client
            .platform()
            .state()
            .setting(USER_ID)
            .unwrap()
            .update(USER.parse().unwrap())
            .await
            .unwrap();
    }

    let client = client(dir.path()).await;
    state_import::migrate(&client, &storage).await.unwrap();

    let tokens = client
        .platform()
        .state()
        .setting(AUTHENTICATION_TOKENS)
        .unwrap()
        .get()
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        tokens.refresh_token.as_deref(),
        Some("the-refresh-token"),
        "identity mix-up: took the active account's token instead of the known user's"
    );
}

/// A known user with no token in `data.json` has nothing to carry over, and
/// must not silently fall back to some other account's login.
#[tokio::test]
async fn a_known_user_with_no_token_does_not_fall_back() {
    let dir = TempDir::new().unwrap();
    let entries = vec![
        ("global_account_accounts".into(), accounts(&[USER, OTHER_USER])),
        (
            format!("user_{OTHER_USER}_token_refreshToken"),
            serde_json::json!("another-refresh-token"),
        ),
    ];
    let storage = write_data_json(dir.path(), entries).await;

    {
        let client = client(dir.path()).await;
        client
            .platform()
            .state()
            .setting(USER_ID)
            .unwrap()
            .update(USER.parse().unwrap())
            .await
            .unwrap();
    }

    let client = client(dir.path()).await;
    assert!(state_import::migrate(&client, &storage).await.is_err());
    assert!(!sdk_session::is_authenticated(&client).await);
}

/// After migrating, `data.json` must point at the account. It still namespaces
/// sends, collections, organizations and the last-sync time by user, so a null
/// `activeAccountId` would leave the login migrated but unfindable.
#[tokio::test]
async fn adopts_the_migrated_account_as_active() {
    let dir = TempDir::new().unwrap();
    let mut entries = full_login();
    entries.retain(|(k, _)| k != "global_account_activeAccountId");
    let storage = write_data_json(dir.path(), entries).await;
    let client = client(dir.path()).await;

    state_import::migrate(&client, &storage).await.unwrap();

    let active: serde_json::Value = storage
        .lock()
        .await
        .get("global_account_activeAccountId")
        .unwrap()
        .unwrap();
    assert_eq!(active, serde_json::json!(USER));
}

/// Adopting the account must not blank out registry details it already has.
/// `register_account` replaces the whole entry, so calling it with a partial
/// picture would lose the email or name that was there.
#[tokio::test]
async fn adopting_the_account_preserves_existing_registry_details() {
    let dir = TempDir::new().unwrap();
    let mut entries = full_login();
    entries.retain(|(k, _)| k != "global_account_activeAccountId" && k != "global_account_accounts");
    entries.push((
        "global_account_accounts".into(),
        serde_json::json!({
            USER: { "email": "kept@example.com", "emailVerified": true, "name": "Kept Name" }
        }),
    ));
    let storage = write_data_json(dir.path(), entries).await;
    let client = client(dir.path()).await;

    state_import::migrate(&client, &storage).await.unwrap();

    let accounts: serde_json::Value = storage
        .lock()
        .await
        .get("global_account_accounts")
        .unwrap()
        .unwrap();
    let entry = &accounts[USER];
    assert_eq!(entry["email"], "kept@example.com");
    assert_eq!(entry["name"], "Kept Name");
}

/// The email can be missing from `data.json` but present in SDK state (the
/// crypto state migrated, the tokens did not). Take it from whichever store has
/// it, and leave both consistent.
#[tokio::test]
async fn takes_the_email_from_sdk_state_when_data_json_has_none() {
    let dir = TempDir::new().unwrap();
    let mut entries = full_login();
    entries.retain(|(k, _)| k != "global_account_accounts");
    entries.push((
        "global_account_accounts".into(),
        serde_json::json!({ USER: { "email": "", "emailVerified": false } }),
    ));
    let storage = write_data_json(dir.path(), entries).await;

    {
        let client = client(dir.path()).await;
        client
            .platform()
            .state()
            .setting(USER_EMAIL)
            .unwrap()
            .update("from-sdk@example.com".to_string())
            .await
            .unwrap();
    }

    let client = client(dir.path()).await;
    state_import::migrate(&client, &storage).await.unwrap();

    let accounts: serde_json::Value = storage
        .lock()
        .await
        .get("global_account_accounts")
        .unwrap()
        .unwrap();
    assert_eq!(accounts[USER]["email"], "from-sdk@example.com");
}
