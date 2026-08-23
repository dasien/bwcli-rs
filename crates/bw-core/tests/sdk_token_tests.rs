//! Token persistence and renewal, now owned by the SDK.
//!
//! These cover the wiring that replaced `TokenManager`. Each one corresponds to
//! something that was broken in the hand-rolled version:
//!
//! - tokens written by login must be readable by the *next* invocation
//!   (the old code forgot `storage.flush()`, so renewed tokens were lost)
//! - renewal must send `client_id`, which the old request model omitted, making
//!   every refresh fail `invalid_request`
//! - a renewed token must be persisted, not just used for the one request
//! - logout must clear the login method too, or an API-key login could be
//!   silently resurrected from the stored client secret

use bitwarden_core::client::login_method::UserLoginMethod;
use bitwarden_core::client::persisted_state::{
    AUTHENTICATION_TOKENS, USER_LOGIN_METHOD,
};
use bitwarden_crypto::Kdf;
use bw_core::services::{create_sdk_client_with_state, open_state};
use bw_core::services::sdk_session;
use std::path::Path;
use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// A client over `dir` pointed at `server`. Calling this twice with the same
/// `dir` models two CLI invocations sharing one state directory.
///
/// (`https_only` is only enforced in release builds, so http:// works here.)
async fn client_at(dir: &Path, server: &MockServer) -> bitwarden_core::Client {
    let registry = open_state(dir.to_path_buf()).await.expect("state");
    create_sdk_client_with_state(Some(server.uri()), Some(server.uri()), registry)
}

fn password_login() -> UserLoginMethod {
    UserLoginMethod::Username {
        client_id: "cli".to_string(),
        email: "test@example.com".to_string(),
        kdf: Kdf::default_pbkdf2(),
    }
}

/// `GET /folders`, which the generated client marks as requiring auth.
async fn mock_folders(server: &MockServer) {
    Mock::given(method("GET"))
        .and(path("/folders"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "object": "list",
            "data": []
        })))
        .mount(server)
        .await;
}

#[tokio::test]
async fn attaches_the_persisted_token_to_authenticated_requests() {
    let server = MockServer::start().await;
    mock_folders(&server).await;
    let dir = tempfile::tempdir().unwrap();

    {
        let client = client_at(dir.path(), &server).await;
        sdk_session::persist_tokens(&client, password_login(), "the-token", Some("refresh"), 3600)
            .await
            .unwrap();
    }

    // A *separate* client, as a later `bw` invocation would be.
    let client = client_at(dir.path(), &server).await;
    client
        .internal
        .get_api_configurations()
        .api_client
        .folders_api()
        .get_all()
        .await
        .expect("authenticated request should succeed");

    let requests = server.received_requests().await.unwrap();
    let folders = requests
        .iter()
        .find(|r| r.url.path() == "/folders")
        .expect("a request to /folders");
    assert_eq!(
        folders.headers.get("Authorization").unwrap(),
        "Bearer the-token",
        "the token handler should attach the persisted token"
    );
}

#[tokio::test]
async fn renews_an_expired_token_and_persists_the_result() {
    let server = MockServer::start().await;
    mock_folders(&server).await;

    Mock::given(method("POST"))
        .and(path("/connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "renewed-token",
            "expires_in": 3600,
            "refresh_token": "next-refresh",
            "token_type": "Bearer",
            "scope": "api offline_access"
        })))
        .mount(&server)
        .await;

    let dir = tempfile::tempdir().unwrap();

    {
        let client = client_at(dir.path(), &server).await;
        // `expires_in: 0` puts expiry inside the handler's 5-minute renewal
        // margin, so the next authenticated request must renew first.
        sdk_session::persist_tokens(&client, password_login(), "stale-token", Some("refresh"), 0)
            .await
            .unwrap();
    }

    let client = client_at(dir.path(), &server).await;
    client
        .internal
        .get_api_configurations()
        .api_client
        .folders_api()
        .get_all()
        .await
        .expect("request should succeed after renewal");

    let requests = server.received_requests().await.unwrap();

    let renewal = requests
        .iter()
        .find(|r| r.url.path() == "/connect/token")
        .expect("the handler should have renewed the token");
    let body = String::from_utf8_lossy(&renewal.body);
    assert!(
        body.contains("client_id=cli"),
        "renewal must send the client id; omitting it made every refresh fail \
         with invalid_request. body: {body}"
    );
    assert!(body.contains("grant_type=refresh_token"), "body: {body}");

    let folders = requests
        .iter()
        .find(|r| r.url.path() == "/folders")
        .expect("a request to /folders");
    assert_eq!(
        folders.headers.get("Authorization").unwrap(),
        "Bearer renewed-token",
        "the request should carry the renewed token, not the stale one"
    );

    // The renewed token must outlive this process, or the next invocation
    // renews again from a refresh token the server has already rotated.
    let stored = client_at(dir.path(), &server)
        .await
        .platform()
        .state()
        .setting(AUTHENTICATION_TOKENS)
        .unwrap()
        .get()
        .await
        .unwrap()
        .expect("tokens should still be stored");
    assert_eq!(stored.access_token, "renewed-token");
    assert_eq!(stored.refresh_token.as_deref(), Some("next-refresh"));
}

#[tokio::test]
async fn an_api_key_login_renews_without_a_refresh_token() {
    let server = MockServer::start().await;
    mock_folders(&server).await;

    Mock::given(method("POST"))
        .and(path("/connect/token"))
        .and(body_string_contains("grant_type=client_credentials"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "api-key-token",
            "expires_in": 3600,
            "token_type": "Bearer",
            "scope": "api"
        })))
        .mount(&server)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let client = client_at(dir.path(), &server).await;

    // API-key tokens are re-minted from the stored credentials rather than
    // refreshed, which is why the login method has to carry them.
    sdk_session::persist_tokens(
        &client,
        UserLoginMethod::ApiKey {
            client_id: "user.1234".to_string(),
            client_secret: "the-secret".to_string(),
            email: "test@example.com".to_string(),
            kdf: Kdf::default_pbkdf2(),
        },
        "stale-token",
        None,
        0,
    )
    .await
    .unwrap();

    client
        .internal
        .get_api_configurations()
        .api_client
        .folders_api()
        .get_all()
        .await
        .expect("request should succeed after re-minting the token");

    let requests = server.received_requests().await.unwrap();
    let folders = requests
        .iter()
        .find(|r| r.url.path() == "/folders")
        .expect("a request to /folders");
    assert_eq!(
        folders.headers.get("Authorization").unwrap(),
        "Bearer api-key-token"
    );
}

#[tokio::test]
async fn clearing_tokens_removes_the_login_method_too() {
    let server = MockServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let client = client_at(dir.path(), &server).await;

    sdk_session::persist_tokens(&client, password_login(), "the-token", Some("refresh"), 3600)
        .await
        .unwrap();
    assert!(sdk_session::is_authenticated(&client).await);

    sdk_session::clear_tokens(&client).await.unwrap();

    assert!(
        !sdk_session::is_authenticated(&client).await,
        "tokens should be gone"
    );
    assert!(
        client
            .platform()
            .state()
            .setting(USER_LOGIN_METHOD)
            .unwrap()
            .get()
            .await
            .unwrap()
            .is_none(),
        "the login method must go too: for an API-key login it holds the client \
         secret, which the token handler would use to mint fresh tokens"
    );
}

/// An expired token still counts as authenticated: the handler renews it
/// transparently, so treating it as logged-out would make `bw sync` fail an hour
/// after login rather than just refreshing.
#[tokio::test]
async fn an_expired_token_still_counts_as_authenticated() {
    let server = MockServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let client = client_at(dir.path(), &server).await;

    sdk_session::persist_tokens(&client, password_login(), "stale", Some("refresh"), 0)
        .await
        .unwrap();

    assert!(sdk_session::is_authenticated(&client).await);
}

#[tokio::test]
async fn no_login_means_not_authenticated() {
    let server = MockServer::start().await;
    let dir = tempfile::tempdir().unwrap();

    assert!(!sdk_session::is_authenticated(&client_at(dir.path(), &server).await).await);
}
