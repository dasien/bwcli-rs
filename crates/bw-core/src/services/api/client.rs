use super::{
    environment::Environment, errors::ApiError, token_manager::TokenManager, traits::ApiClient,
};
use crate::models::api::token::{TokenRefreshRequest, TokenResponse};
use crate::services::storage::JsonFileStorage;
use anyhow::Result;
use async_trait::async_trait;
use reqwest::{Client as ReqwestClient, Request, Response, StatusCode, header};
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

/// Bitwarden API client implementation
///
/// Features:
/// - Automatic token refresh on 401 responses
/// - Connection pooling (via reqwest)
/// - Proxy support from environment variables
/// - TLS with certificate validation (rustls)
/// - Configurable timeouts
/// - Custom User-Agent header
pub struct BitwardenApiClient {
    /// HTTP client (reused across requests for connection pooling)
    http_client: ReqwestClient,

    /// Environment URLs
    environment: Environment,

    /// Token manager for authentication
    token_manager: Arc<TokenManager>,

    /// Storage for configuration
    storage: Arc<Mutex<JsonFileStorage>>,
}

impl BitwardenApiClient {
    /// Create new API client
    ///
    /// # Arguments
    /// * `environment` - Environment URLs configuration
    /// * `storage` - Storage for token/config access
    /// * `timeout_seconds` - Optional request timeout (default: 60s)
    ///
    /// # Configuration
    /// Reads from environment variables:
    /// - HTTP_PROXY / HTTPS_PROXY - Proxy server
    /// - NO_PROXY - Proxy bypass patterns
    pub fn new(
        environment: Environment,
        storage: Arc<Mutex<JsonFileStorage>>,
        timeout_seconds: Option<u64>,
    ) -> Result<Self> {
        let timeout = Duration::from_secs(timeout_seconds.unwrap_or(60));

        // Build HTTP client with all features
        let mut default_headers = header::HeaderMap::new();
        default_headers.insert(
            header::HeaderName::from_static("bitwarden-client-name"),
            header::HeaderValue::from_static("cli"),
        );
        default_headers.insert(
            header::HeaderName::from_static("bitwarden-client-version"),
            header::HeaderValue::from_static(env!("CARGO_PKG_VERSION")),
        );

        let http_client = ReqwestClient::builder()
            .timeout(timeout)
            .connect_timeout(Duration::from_secs(30))
            .user_agent(format!(
                "Bitwarden_CLI/{} (Rust)",
                env!("CARGO_PKG_VERSION")
            ))
            .default_headers(default_headers)
            .use_rustls_tls()
            .build()
            .map_err(|e| ApiError::Configuration(format!("Failed to create HTTP client: {}", e)))?;

        let token_manager = Arc::new(TokenManager::new(Arc::clone(&storage)));

        Ok(Self {
            http_client,
            environment,
            token_manager,
            storage,
        })
    }

    /// Build full URL from path
    ///
    /// When `use_identity` is true and path starts with `/identity/`, the prefix
    /// is stripped since the identity base URL is the identity server root.
    fn build_url(&self, path: &str, use_identity: bool) -> String {
        let base = if use_identity {
            self.environment.identity_url()
        } else {
            self.environment.api_url()
        };

        // Strip service prefix from path if already in base URL
        let path = path.trim_start_matches('/');
        let path = if use_identity {
            path.trim_start_matches("identity/")
        } else {
            path
        };
        format!("{}/{}", base, path)
    }

    /// Execute request with automatic retry on token refresh
    async fn execute_with_retry(
        &self,
        mut request: Request,
        requires_auth: bool,
    ) -> Result<Response> {
        // Try request first time
        let response = self
            .http_client
            .execute(
                request
                    .try_clone()
                    .ok_or_else(|| anyhow::anyhow!("Failed to clone request"))?,
            )
            .await?;

        // Check for 401 Unauthorized (expired token)
        if requires_auth && response.status() == StatusCode::UNAUTHORIZED {
            // Attempt token refresh
            let new_token = self
                .token_manager
                .refresh_access_token(|refresh_req| async {
                    self.post_token_refresh(refresh_req).await
                })
                .await?;

            // Update request with new token
            request.headers_mut().insert(
                header::AUTHORIZATION,
                header::HeaderValue::from_str(&format!("Bearer {}", new_token.expose_secret()))
                    .unwrap(),
            );

            // Retry request with new token
            let response = self.http_client.execute(request).await?;
            return self.process_response(response).await;
        }

        self.process_response(response).await
    }

    /// Process response and map errors
    async fn process_response(&self, response: Response) -> Result<Response> {
        let status = response.status();

        match status {
            s if s.is_success() => Ok(response),
            StatusCode::UNAUTHORIZED => Err(ApiError::Authentication {
                message: "Authentication required".to_string(),
                hint: "Run 'bw login' to authenticate again".to_string(),
            }
            .into()),
            StatusCode::FORBIDDEN => Err(ApiError::Authentication {
                message: "Access forbidden".to_string(),
                hint: "Check your permissions or run 'bw login' again".to_string(),
            }
            .into()),
            StatusCode::NOT_FOUND => {
                let url = response.url().path();
                Err(ApiError::NotFound {
                    resource: url.to_string(),
                }
                .into())
            }
            StatusCode::TOO_MANY_REQUESTS => {
                let retry_after = response
                    .headers()
                    .get(header::RETRY_AFTER)
                    .and_then(|h| h.to_str().ok())
                    .and_then(|s| s.parse().ok());

                Err(ApiError::rate_limit_error(retry_after).into())
            }
            s if s.is_client_error() => {
                let message = self.extract_error_message(response).await?;
                Err(ApiError::Client { status: s, message }.into())
            }
            s if s.is_server_error() => {
                let message = self.extract_error_message(response).await?;
                Err(ApiError::server_error(s, message).into())
            }
            s => Err(ApiError::Client {
                status: s,
                message: "Unexpected status code".to_string(),
            }
            .into()),
        }
    }

    /// Extract error message from response body
    async fn extract_error_message(&self, response: Response) -> Result<String> {
        // Try to parse Bitwarden error response format
        #[derive(Deserialize)]
        struct ErrorResponse {
            #[serde(rename = "Message")]
            message: Option<String>,
            #[serde(rename = "error")]
            error: Option<String>,
            #[serde(rename = "error_description")]
            error_description: Option<String>,
        }

        let text = response.text().await?;

        if let Ok(err_response) = serde_json::from_str::<ErrorResponse>(&text) {
            return Ok(err_response
                .message
                .or(err_response.error_description)
                .or(err_response.error)
                .unwrap_or_else(|| "Unknown error".to_string()));
        }

        // Fallback to raw text
        Ok(text)
    }

    /// Post token refresh request (special handling for identity endpoint)
    async fn post_token_refresh(&self, request: TokenRefreshRequest) -> Result<TokenResponse> {
        let url = format!("{}/connect/token", self.environment.identity_url());

        let response = self
            .http_client
            .post(&url)
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .form(&request)
            .send()
            .await?;

        let response = self.process_response(response).await?;
        let token_response: TokenResponse = response.json().await?;

        Ok(token_response)
    }

    /// Post form-encoded data (for OAuth2 endpoints)
    ///
    /// OAuth2 token endpoints require application/x-www-form-urlencoded encoding,
    /// not JSON. This method handles that requirement.
    ///
    /// # Arguments
    /// * `path` - API path (automatically determines if identity vs API endpoint)
    /// * `body` - Request body that will be form-encoded
    /// * `extra_headers` - Optional extra headers to include (e.g., Auth-Email for password login)
    ///
    /// # Returns
    /// Deserialized response of type R
    pub async fn post_form<T, R>(
        &self,
        path: &str,
        body: &T,
        extra_headers: Option<Vec<(&str, String)>>,
    ) -> Result<R>
    where
        T: Serialize + Send + Sync,
        R: for<'de> Deserialize<'de>,
    {
        // Identity endpoints (like /connect/token) need identity URL
        let use_identity = path.contains("/identity/") || path.contains("/connect/");
        let url = self.build_url(path, use_identity);

        let mut request_builder = self
            .http_client
            .post(&url)
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded; charset=utf-8")
            .header(header::ACCEPT, "application/json")
            .form(body);

        // Add any extra headers
        if let Some(headers) = extra_headers {
            for (name, value) in headers {
                request_builder = request_builder.header(name, value);
            }
        }

        let request = request_builder.build()?;

        let response = self.execute_with_retry(request, false).await?;
        let data: R = response.json().await?;

        Ok(data)
    }

    /// Post JSON to identity server endpoint
    ///
    /// # Arguments
    /// * `path` - Path relative to identity server root
    /// * `body` - Request body that will be JSON-encoded
    pub async fn post_identity<T, R>(&self, path: &str, body: &T) -> Result<R>
    where
        T: Serialize + Send + Sync,
        R: for<'de> Deserialize<'de>,
    {
        let url = self.build_url(path, true);

        let request = self
            .http_client
            .post(&url)
            .header(header::CONTENT_TYPE, "application/json")
            .json(body)
            .build()?;

        let response = self.execute_with_retry(request, false).await?;
        let data: R = response.json().await?;

        Ok(data)
    }

    /// Post form-encoded data to identity server endpoint
    ///
    /// Used for OAuth2 token requests which require form encoding.
    ///
    /// # Arguments
    /// * `path` - Path relative to identity server root
    /// * `form_data` - Key-value pairs for form encoding
    pub async fn post_identity_form<R>(&self, path: &str, form_data: &[(&str, String)]) -> Result<R>
    where
        R: for<'de> Deserialize<'de>,
    {
        let url = self.build_url(path, true);

        let request = self
            .http_client
            .post(&url)
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .form(form_data)
            .build()?;

        let response = self.execute_with_retry(request, false).await?;
        let data: R = response.json().await?;

        Ok(data)
    }

    /// GET with explicit authorization token
    ///
    /// Used when we have a token but haven't stored it yet (during login flow).
    ///
    /// # Arguments
    /// * `path` - API path
    /// * `access_token` - Bearer token for authorization
    pub async fn get_authenticated<R>(&self, path: &str, access_token: &str) -> Result<R>
    where
        R: for<'de> Deserialize<'de>,
    {
        let url = self.build_url(path, false);

        let request = self
            .http_client
            .get(&url)
            .header(header::AUTHORIZATION, format!("Bearer {}", access_token))
            .build()?;

        let response = self.execute_with_retry(request, false).await?;
        let data: R = response.json().await?;

        Ok(data)
    }
}

#[async_trait]
impl ApiClient for BitwardenApiClient {
    async fn get<T>(&self, path: &str) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let url = self.build_url(path, false);
        let request = self.http_client.get(&url).build()?;

        let response = self.execute_with_retry(request, false).await?;
        let data: T = response.json().await?;

        Ok(data)
    }

    async fn get_with_auth<T>(&self, path: &str) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let url = self.build_url(path, false);

        // Get access token
        let token = self
            .token_manager
            .get_access_token()
            .await?
            .ok_or_else(|| ApiError::Authentication {
                message: "Not authenticated".to_string(),
                hint: "Run 'bw login' to authenticate".to_string(),
            })?;

        let request = self
            .http_client
            .get(&url)
            .header(
                header::AUTHORIZATION,
                format!("Bearer {}", token.expose_secret()),
            )
            .build()?;

        let response = self.execute_with_retry(request, true).await?;
        let data: T = response.json().await?;

        Ok(data)
    }

    async fn post<T, R>(&self, path: &str, body: &T) -> Result<R>
    where
        T: Serialize + Send + Sync,
        R: for<'de> Deserialize<'de>,
    {
        // Identity endpoints need identity URL
        let use_identity = path.contains("/identity/") || path.contains("/connect/");
        let url = self.build_url(path, use_identity);

        let request = self
            .http_client
            .post(&url)
            .header(header::CONTENT_TYPE, "application/json")
            .json(body)
            .build()?;

        let response = self.execute_with_retry(request, false).await?;
        let data: R = response.json().await?;

        Ok(data)
    }

    async fn post_with_auth<T, R>(&self, path: &str, body: &T) -> Result<R>
    where
        T: Serialize + Send + Sync,
        R: for<'de> Deserialize<'de>,
    {
        let url = self.build_url(path, false);

        let token = self
            .token_manager
            .get_access_token()
            .await?
            .ok_or_else(|| ApiError::Authentication {
                message: "Not authenticated".to_string(),
                hint: "Run 'bw login' to authenticate".to_string(),
            })?;

        let request = self
            .http_client
            .post(&url)
            .header(header::CONTENT_TYPE, "application/json")
            .header(
                header::AUTHORIZATION,
                format!("Bearer {}", token.expose_secret()),
            )
            .json(body)
            .build()?;

        let response = self.execute_with_retry(request, true).await?;
        let data: R = response.json().await?;

        Ok(data)
    }

    async fn put_with_auth<T, R>(&self, path: &str, body: &T) -> Result<R>
    where
        T: Serialize + Send + Sync,
        R: for<'de> Deserialize<'de>,
    {
        let url = self.build_url(path, false);

        let token = self
            .token_manager
            .get_access_token()
            .await?
            .ok_or_else(|| ApiError::Authentication {
                message: "Not authenticated".to_string(),
                hint: "Run 'bw login' to authenticate".to_string(),
            })?;

        let request = self
            .http_client
            .put(&url)
            .header(header::CONTENT_TYPE, "application/json")
            .header(
                header::AUTHORIZATION,
                format!("Bearer {}", token.expose_secret()),
            )
            .json(body)
            .build()?;

        let response = self.execute_with_retry(request, true).await?;
        let data: R = response.json().await?;

        Ok(data)
    }

    async fn delete_with_auth(&self, path: &str) -> Result<()> {
        let url = self.build_url(path, false);

        let token = self
            .token_manager
            .get_access_token()
            .await?
            .ok_or_else(|| ApiError::Authentication {
                message: "Not authenticated".to_string(),
                hint: "Run 'bw login' to authenticate".to_string(),
            })?;

        let request = self
            .http_client
            .delete(&url)
            .header(
                header::AUTHORIZATION,
                format!("Bearer {}", token.expose_secret()),
            )
            .build()?;

        let _response = self.execute_with_retry(request, true).await?;

        Ok(())
    }

    fn environment(&self) -> &Environment {
        &self.environment
    }

    async fn is_authenticated(&self) -> bool {
        self.token_manager
            .get_access_token()
            .await
            .ok()
            .flatten()
            .is_some()
    }
}
