use super::{environment::Environment, errors::ApiError, traits::ApiClient};
use anyhow::Result;
use async_trait::async_trait;
use reqwest::{Client as ReqwestClient, Request, Response, StatusCode, header};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// HTTP client for the endpoints the SDK does not cover.
///
/// This is **unauthenticated by design**. Every authenticated call now goes
/// through the SDK's generated clients, which carry its token handler; what is
/// left here is login, prelogin and the identity endpoints, none of which take a
/// bearer token. Adding an authenticated method here would reintroduce the split
/// token state this deliberately removed — reach for
/// `Client::internal::get_api_configurations()` instead.
///
/// Features:
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
}

impl BitwardenApiClient {
    /// Create new API client
    ///
    /// # Arguments
    /// * `environment` - Environment URLs configuration
    /// * `timeout_seconds` - Optional request timeout (default: 60s)
    ///
    /// # Configuration
    /// Reads from environment variables:
    /// - HTTP_PROXY / HTTPS_PROXY - Proxy server
    /// - NO_PROXY - Proxy bypass patterns
    pub fn new(environment: Environment, timeout_seconds: Option<u64>) -> Result<Self> {
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

        Ok(Self {
            http_client,
            environment,
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

    /// Execute a request and map its status to an error.
    async fn execute(&self, request: Request) -> Result<Response> {
        let response = self.http_client.execute(request).await?;
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
        let text = response.text().await?;
        Ok(describe_error_body(&text))
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
            .header(
                header::CONTENT_TYPE,
                "application/x-www-form-urlencoded; charset=utf-8",
            )
            .header(header::ACCEPT, "application/json")
            .form(body);

        // Add any extra headers
        if let Some(headers) = extra_headers {
            for (name, value) in headers {
                request_builder = request_builder.header(name, value);
            }
        }

        let request = request_builder.build()?;

        let response = self.execute(request).await?;
        deserialize_response(response, &url).await
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

        let response = self.execute(request).await?;
        deserialize_response(response, &url).await
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

        let response = self.execute(request).await?;
        deserialize_response(response, &url).await
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

        let response = self.execute(request).await?;
        deserialize_response(response, &url).await
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

        let response = self.execute(request).await?;
        deserialize_response(response, &url).await
    }

    fn environment(&self) -> &Environment {
        &self.environment
    }
}

async fn deserialize_response<R>(response: Response, url: &str) -> Result<R>
where
    R: for<'de> Deserialize<'de>,
{
    let body = response.text().await?;

    match serde_json::from_str::<R>(&body) {
        Ok(data) => Ok(data),
        Err(e) => {
            let shape = match serde_json::from_str::<serde_json::Value>(&body) {
                Ok(serde_json::Value::Object(map)) => {
                    let mut names: Vec<&str> = map.keys().map(String::as_str).collect();
                    names.sort_unstable();
                    format!("server sent fields: [{}]", names.join(", "))
                }
                Ok(other) => format!(
                    "server sent a JSON {} rather than an object",
                    match other {
                        serde_json::Value::Null => "null",
                        serde_json::Value::Bool(_) => "boolean",
                        serde_json::Value::Number(_) => "number",
                        serde_json::Value::String(_) => "string",
                        serde_json::Value::Array(_) => "array",
                        serde_json::Value::Object(_) => unreachable!(),
                    }
                ),
                Err(_) => format!("body was not valid JSON ({} bytes)", body.len()),
            };

            Err(anyhow::anyhow!(
                "could not parse the response from {url}: {e}; {shape}"
            ))
        }
    }
}

/// Turn an error response body into a human-readable message.
///
/// A free function so the parsing rules are unit-testable without a live
/// HTTP response.
fn describe_error_body(text: &str) -> String {
    // Every field is optional, so `from_str` succeeds even for a body that
    // contains none of them. That previously meant an "Unknown error" default
    // was returned and the raw-body fallback was unreachable, hiding the
    // server's actual complaint on every 400.
    #[derive(Deserialize)]
    struct ErrorResponse {
        #[serde(rename = "Message", alias = "message")]
        message: Option<String>,
        #[serde(rename = "error")]
        error: Option<String>,
        #[serde(rename = "error_description")]
        error_description: Option<String>,
        /// Per-field validation failures, which is what the API returns for
        /// most 400s.
        #[serde(rename = "validationErrors", alias = "ValidationErrors")]
        validation_errors: Option<std::collections::HashMap<String, Vec<String>>>,
    }

    if let Ok(err) = serde_json::from_str::<ErrorResponse>(text) {
        let validation = err.validation_errors.and_then(|map| {
            let mut parts: Vec<String> = map
                .into_iter()
                .map(|(field, msgs)| {
                    if field.is_empty() {
                        msgs.join("; ")
                    } else {
                        format!("{field}: {}", msgs.join("; "))
                    }
                })
                .collect();
            parts.sort();
            (!parts.is_empty()).then(|| parts.join(" | "))
        });

        if let Some(message) = err
            .message
            .or(err.error_description)
            .or(err.error)
            .or(validation)
        {
            return message;
        }
    }

    // Nothing recognizable: report the body so the failure is diagnosable
    // rather than an opaque "Unknown error".
    let text = text.trim();
    if text.is_empty() {
        return "empty response body".to_string();
    }

    const MAX: usize = 500;
    if text.len() > MAX {
        format!("unrecognized error body: {}\u{2026}", &text[..MAX])
    } else {
        format!("unrecognized error body: {text}")
    }
}

#[cfg(test)]
mod tests {
    use super::describe_error_body;

    #[test]
    fn prefers_the_message_field() {
        let body = r#"{"Message":"The item cannot be saved because it is out of date."}"#;
        assert_eq!(
            describe_error_body(body),
            "The item cannot be saved because it is out of date."
        );
    }

    #[test]
    fn reports_validation_errors() {
        let body = r#"{"validationErrors":{"Name":["Name is required."]}}"#;
        assert_eq!(describe_error_body(body), "Name: Name is required.");
    }

    #[test]
    fn reports_unkeyed_validation_errors() {
        let body = r#"{"validationErrors":{"":["Something is wrong."]}}"#;
        assert_eq!(describe_error_body(body), "Something is wrong.");
    }

    #[test]
    fn falls_back_to_the_oauth_error_fields() {
        let body = r#"{"error":"invalid_grant","error_description":"bad refresh token"}"#;
        assert_eq!(describe_error_body(body), "bad refresh token");
    }

    /// Regression: a JSON object with none of the known fields used to parse
    /// successfully into an all-`None` struct and report "Unknown error",
    /// hiding what the server actually said.
    #[test]
    fn surfaces_bodies_with_no_recognized_fields() {
        let body = r#"{"somethingElse":"details here"}"#;
        let msg = describe_error_body(body);
        assert!(msg.contains("details here"), "got: {msg}");
        assert!(!msg.contains("Unknown error"));
    }

    #[test]
    fn handles_non_json_and_empty_bodies() {
        assert!(describe_error_body("<html>502</html>").contains("502"));
        assert_eq!(describe_error_body("   "), "empty response body");
    }
}
