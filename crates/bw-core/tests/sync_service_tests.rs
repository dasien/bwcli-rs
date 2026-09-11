//! Integration tests for vault sync
//!
//! Covers two behaviors that were previously broken:
//! - organizations from the sync response's profile were never persisted, so
//!   `bw list organizations` always returned an empty list
//! - `--force` was ignored (the `force` parameter was `_force`), and every
//!   invocation performed a full download regardless of server state

use bitwarden_core::client::login_method::UserLoginMethod;
use bitwarden_crypto::Kdf;
use bw_core::models::vault::Organization;
use bw_core::services::{create_sdk_client_with_state, open_state};
use bw_core::services::sdk_session;
use bw_core::services::storage::{JsonFileStorage, Storage, StorageKey};
use bw_core::services::vault::SyncService;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const TEST_USER_ID: &str = "11111111-1111-4111-8111-111111111111";
const TEST_ORG_ID: &str = "22222222-2222-4222-8222-222222222222";

/// A structurally valid `UnsignedSharedKey` (RSA-OAEP-SHA1, type 4). It is not
/// wrapped to any real key, so it parses but would not decrypt — enough to prove
/// the key is *extracted* from the profile, which is the part that was missing.
fn org_key() -> String {
    format!("4.{}", "A".repeat(342))
}

/// A sync response carrying one organization on the profile and no vault items.
fn sync_response_with_org() -> serde_json::Value {
    serde_json::json!({
        "object": "sync",
        "profile": {
            "object": "profile",
            "id": TEST_USER_ID,
            "email": "test@example.com",
            "emailVerified": true,
            "premium": false,
            "securityStamp": "stamp",
            "organizations": [
                {
                    "object": "profileOrganization",
                    "id": TEST_ORG_ID,
                    "key": org_key(),
                    "name": "Acme Corp",
                    "enabled": true,
                    "status": 2,
                    "type": 0,
                    "usePolicies": true,
                    "useGroups": false,
                    "useDirectory": false,
                    "useEvents": true,
                    "useTotp": true,
                    "useApi": false,
                    "selfHost": false
                }
            ]
        },
        "folders": [],
        "collections": [],
        "ciphers": [],
        // A real server always sends this key. It is spelled out because the
        // SDK's `SendSyncHandler` treats an *absent* list as an error rather
        // than as "no sends" (`BUGLIST.md` S10), and an omission here would fail
        // the whole sync rather than this one handler.
        "sends": []
    })
}

async fn setup(
    server: &MockServer,
    last_sync: Option<&str>,
) -> (SyncService, Arc<Mutex<JsonFileStorage>>, tempfile::TempDir) {
    let temp_dir = tempfile::tempdir().unwrap();
    let mut storage = JsonFileStorage::new(Some(temp_dir.path().to_path_buf())).unwrap();

    storage
        .set(
            &StorageKey::GlobalActiveAccountId.format(None),
            &TEST_USER_ID.to_string(),
        )
        .await
        .unwrap();

    if let Some(ts) = last_sync {
        storage
            .set(
                &StorageKey::UserLastSync.format(Some(TEST_USER_ID)),
                &ts.to_string(),
            )
            .await
            .unwrap();
    }

    storage.flush().await.unwrap();

    let storage = Arc::new(Mutex::new(storage));

    // Sync talks to the server through the SDK's generated API clients, so the
    // SDK client must point at the mock server and hold a token in its own
    // state. (`https_only` is only enforced in release builds, so http:// works
    // here.)
    let registry = open_state(temp_dir.path().to_path_buf()).await.unwrap();
    let sdk = Arc::new(create_sdk_client_with_state(
        Some(server.uri()),
        Some(server.uri()),
        registry,
    ));

    // Far-future expiry, so the token handler attaches this token rather than
    // trying to renew it against the mock server.
    sdk_session::persist_tokens(
        &sdk,
        UserLoginMethod::Username {
            client_id: "cli".to_string(),
            email: "test@example.com".to_string(),
            kdf: Kdf::default_pbkdf2(),
        },
        "fake-access-token",
        Some("fake-refresh-token"),
        3600,
    )
    .await
    .unwrap();

    (
        SyncService::new(Arc::clone(&storage), sdk),
        storage,
        temp_dir,
    )
}

async fn stored_organizations(
    storage: &Arc<Mutex<JsonFileStorage>>,
) -> HashMap<String, Organization> {
    let s = storage.lock().await;
    s.get(&StorageKey::UserOrganizations.format(Some(TEST_USER_ID)))
        .unwrap()
        .unwrap_or_default()
}

/// Regression: organizations were parsed from the response but never written,
/// so `bw list organizations` was permanently empty.
#[tokio::test]
async fn sync_persists_organizations_from_profile() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/sync"))
        .respond_with(ResponseTemplate::new(200).set_body_json(sync_response_with_org()))
        .mount(&server)
        .await;

    let (service, storage, _tmp) = setup(&server, None).await;

    service.sync(false).await.expect("sync should succeed");

    let orgs = stored_organizations(&storage).await;
    assert_eq!(orgs.len(), 1, "expected the org to be persisted");

    let org = orgs.get(TEST_ORG_ID).expect("org keyed by id");
    assert_eq!(org.name, "Acme Corp");
    assert_eq!(org.status, 2, "Confirmed");
    assert_eq!(org.org_type, 0, "Owner");
    assert!(org.enabled);
    assert!(org.use_policies);
    assert!(!org.use_groups);
}

/// Regression: `force` was ignored and the full `/sync` download always ran.
/// With no server-side changes the download must be skipped.
///
/// `last_sync` is established by performing a real sync rather than seeded:
/// `SyncClient` keeps it as an SDK setting whose key (`LAST_SYNC`) is
/// `pub(crate)` in `bitwarden-sync`, so there is no public way to write it
/// (`BUGLIST.md` S11). Driving it through a sync is also the more honest test —
/// it exercises the same path a user does.
#[tokio::test]
async fn sync_is_skipped_when_server_reports_no_changes() {
    let server = MockServer::start().await;

    // Fixed in the past, so once the first sync stamps `last_sync` to "now" the
    // revision check must conclude there is nothing new.
    let server_revision = chrono::Utc::now() - chrono::Duration::hours(1);

    Mock::given(method("GET"))
        .and(path("/accounts/revision-date"))
        .respond_with(ResponseTemplate::new(200).set_body_json(server_revision.timestamp_millis()))
        .mount(&server)
        .await;

    // Exactly once: the first sync downloads (no stored `last_sync` yet, so the
    // revision check is skipped), the second must not.
    Mock::given(method("GET"))
        .and(path("/sync"))
        .respond_with(ResponseTemplate::new(200).set_body_json(sync_response_with_org()))
        .expect(1)
        .mount(&server)
        .await;

    let (service, _storage, _tmp) = setup(&server, None).await;

    let first = service.sync(false).await.expect("first sync should succeed");
    let second = service
        .sync(false)
        .await
        .expect("second sync should succeed");

    assert!(
        second >= first,
        "the skip path must still report a timestamp, not regress it: {first} -> {second}"
    );
    // `expect(1)` on the /sync mock is what proves the download was skipped; it
    // is verified when the server drops.
}

#[tokio::test]
async fn force_downloads_even_when_server_reports_no_changes() {
    let server = MockServer::start().await;

    let last_sync = chrono::Utc::now();
    let server_revision = last_sync - chrono::Duration::hours(1);

    Mock::given(method("GET"))
        .and(path("/accounts/revision-date"))
        .respond_with(ResponseTemplate::new(200).set_body_json(server_revision.timestamp_millis()))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/sync"))
        .respond_with(ResponseTemplate::new(200).set_body_json(sync_response_with_org()))
        .expect(1)
        .mount(&server)
        .await;

    let (service, storage, _tmp) = setup(&server, Some(&last_sync.to_rfc3339())).await;

    service.sync(true).await.expect("forced sync should succeed");

    assert_eq!(stored_organizations(&storage).await.len(), 1);
}

/// A newer server revision means we download without needing `--force`.
#[tokio::test]
async fn sync_downloads_when_server_has_newer_revision() {
    let server = MockServer::start().await;

    let last_sync = chrono::Utc::now() - chrono::Duration::hours(2);
    let server_revision = chrono::Utc::now();

    Mock::given(method("GET"))
        .and(path("/accounts/revision-date"))
        .respond_with(ResponseTemplate::new(200).set_body_json(server_revision.timestamp_millis()))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/sync"))
        .respond_with(ResponseTemplate::new(200).set_body_json(sync_response_with_org()))
        .expect(1)
        .mount(&server)
        .await;

    let (service, storage, _tmp) = setup(&server, Some(&last_sync.to_rfc3339())).await;

    service.sync(false).await.expect("sync should succeed");

    assert_eq!(stored_organizations(&storage).await.len(), 1);
}

/// A negative revision timestamp is how the server signals a deleted account.
/// A negative account revision date means the account is gone server-side.
///
/// The revision check only runs once a `last_sync` exists, so this syncs
/// normally first, then swaps the server's answer for a negative timestamp.
#[tokio::test]
async fn negative_revision_date_reports_deleted_account() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/accounts/revision-date"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json((chrono::Utc::now() - chrono::Duration::hours(1)).timestamp_millis()),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/sync"))
        .respond_with(ResponseTemplate::new(200).set_body_json(sync_response_with_org()))
        .mount(&server)
        .await;

    let (service, _storage, _tmp) = setup(&server, None).await;
    service
        .sync(false)
        .await
        .expect("the first sync establishes last_sync");

    // Now the account disappears.
    server.reset().await;
    Mock::given(method("GET"))
        .and(path("/accounts/revision-date"))
        .respond_with(ResponseTemplate::new(200).set_body_json(-1_i64))
        .mount(&server)
        .await;

    let err = service
        .sync(false)
        .await
        .expect_err("a deleted account should be reported");

    assert!(
        err.to_string().to_lowercase().contains("deleted"),
        "error should name the cause, got: {err}"
    );
}

#[test]
fn the_sync_response_yields_organization_keys() {
    let response: bitwarden_api_api::models::SyncResponseModel =
        serde_json::from_value(sync_response_with_org()).unwrap();

    let profile = response.profile.as_deref().unwrap();
    let orgs = profile.organizations.as_ref().unwrap();

    let keys = Organization::keys_from_api(orgs);

    assert_eq!(orgs.iter().filter_map(Organization::from_api).count(), 1);
    assert_eq!(
        keys.len(),
        1,
        "the profile's organization key must be extracted"
    );
    assert!(keys.contains_key(&TEST_ORG_ID.parse().unwrap()));
}

/// An unreadable organization key must not fail the whole sync: that
/// organization's items stay undecryptable, which is no worse than before and far
/// better than refusing to sync anything.
#[test]
fn an_unreadable_organization_key_is_skipped_not_fatal() {
    let mut body = sync_response_with_org();
    body["profile"]["organizations"][0]["key"] = serde_json::json!("not-a-key");
    let response: bitwarden_api_api::models::SyncResponseModel =
        serde_json::from_value(body).unwrap();

    let profile = response.profile.as_deref().unwrap();
    let orgs = profile.organizations.as_ref().unwrap();

    assert_eq!(
        orgs.iter().filter_map(Organization::from_api).count(),
        1,
        "the organization is still listed"
    );
    assert!(
        Organization::keys_from_api(orgs).is_empty(),
        "an unreadable key is skipped, not fatal"
    );
}

/// An organization key that cannot be *unwrapped* (no private key in the store,
/// which is any locked or partially unlocked vault) must not fail the sync
/// either. `sync` is how a bad local state gets repaired, so it has to survive
/// one. This is the flaw the pre-existing sync tests caught when the load was
/// first written as fatal.
#[tokio::test]
async fn a_key_that_cannot_be_unwrapped_does_not_fail_the_sync() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/sync"))
        .respond_with(ResponseTemplate::new(200).set_body_json(sync_response_with_org()))
        .mount(&server)
        .await;

    let (service, storage, _tmp) = setup(&server, None).await;

    // The fixture's key is well-formed but wrapped to nothing, and the test
    // client has no private key, so unwrapping must fail.
    service.sync(true).await.expect("sync should still succeed");


    assert_eq!(
        stored_organizations(&storage).await.len(),
        1,
        "the organization is still recorded for `bw list organizations`"
    );
}
