//! Integration tests for vault sync
//!
//! Covers two behaviors that were previously broken:
//! - organizations from the sync response's profile were never persisted, so
//!   `bw list organizations` always returned an empty list
//! - `--force` was ignored (the `force` parameter was `_force`), and every
//!   invocation performed a full download regardless of server state

use bw_core::models::vault::Organization;
use bw_core::services::api::{BitwardenApiClient, Environment};
use bw_core::services::storage::{JsonFileStorage, Storage, StorageKey};
use bw_core::services::vault::SyncService;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const TEST_USER_ID: &str = "11111111-1111-4111-8111-111111111111";
const TEST_ORG_ID: &str = "22222222-2222-4222-8222-222222222222";

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
        "ciphers": []
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

    // `is_authenticated` only checks for a stored access token.
    storage
        .set(
            &StorageKey::UserAccessToken.format(Some(TEST_USER_ID)),
            &"fake-access-token".to_string(),
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
    let environment = Environment::from_base_url(&server.uri()).unwrap();
    let api_client = Arc::new(
        BitwardenApiClient::new(environment, Arc::clone(&storage), None).unwrap(),
    );

    (
        SyncService::new(api_client, Arc::clone(&storage)),
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
        .and(path("/api/sync"))
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
#[tokio::test]
async fn sync_is_skipped_when_server_reports_no_changes() {
    let server = MockServer::start().await;

    let last_sync = chrono::Utc::now();
    let server_revision = last_sync - chrono::Duration::hours(1);

    Mock::given(method("GET"))
        .and(path("/api/accounts/revision-date"))
        .respond_with(ResponseTemplate::new(200).set_body_json(server_revision.timestamp_millis()))
        .mount(&server)
        .await;

    // Must not be called.
    Mock::given(method("GET"))
        .and(path("/api/sync"))
        .respond_with(ResponseTemplate::new(200).set_body_json(sync_response_with_org()))
        .expect(0)
        .mount(&server)
        .await;

    let (service, _storage, _tmp) = setup(&server, Some(&last_sync.to_rfc3339())).await;

    let returned = service.sync(false).await.expect("sync should succeed");

    assert_eq!(
        returned,
        last_sync.to_rfc3339(),
        "should report the existing timestamp when nothing changed"
    );
}

#[tokio::test]
async fn force_downloads_even_when_server_reports_no_changes() {
    let server = MockServer::start().await;

    let last_sync = chrono::Utc::now();
    let server_revision = last_sync - chrono::Duration::hours(1);

    Mock::given(method("GET"))
        .and(path("/api/accounts/revision-date"))
        .respond_with(ResponseTemplate::new(200).set_body_json(server_revision.timestamp_millis()))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/api/sync"))
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
        .and(path("/api/accounts/revision-date"))
        .respond_with(ResponseTemplate::new(200).set_body_json(server_revision.timestamp_millis()))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/api/sync"))
        .respond_with(ResponseTemplate::new(200).set_body_json(sync_response_with_org()))
        .expect(1)
        .mount(&server)
        .await;

    let (service, storage, _tmp) = setup(&server, Some(&last_sync.to_rfc3339())).await;

    service.sync(false).await.expect("sync should succeed");

    assert_eq!(stored_organizations(&storage).await.len(), 1);
}

/// A negative revision timestamp is how the server signals a deleted account.
#[tokio::test]
async fn negative_revision_date_reports_deleted_account() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/accounts/revision-date"))
        .respond_with(ResponseTemplate::new(200).set_body_json(-1i64))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/api/sync"))
        .respond_with(ResponseTemplate::new(200).set_body_json(sync_response_with_org()))
        .expect(0)
        .mount(&server)
        .await;

    let (service, _storage, _tmp) =
        setup(&server, Some(&chrono::Utc::now().to_rfc3339())).await;

    let err = service
        .sync(false)
        .await
        .expect_err("a deleted account should be reported");

    assert!(
        err.to_string().contains("no longer exists"),
        "unexpected error: {err}"
    );
}
