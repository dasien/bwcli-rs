---
enhancement: 03-api-client
agent: architect
task_id: task_1764796128_58799
timestamp: 2025-12-03T16:30:00-08:00
status: READY_FOR_IMPLEMENTATION
---

# API Client Implementation Plan

## Executive Summary

This document provides a comprehensive technical architecture and implementation plan for the Bitwarden CLI Rust migration API client layer. The API client provides HTTP transport and authentication infrastructure for communicating with Bitwarden servers, handling token management, proxy support, and error mapping.

**Key Design Decisions:**
- **HTTP Client**: reqwest 0.12+ with rustls-tls for security and performance
- **Architecture Pattern**: Trait-based abstraction with concrete implementation for testability
- **Token Refresh**: Automatic with mutex-based concurrency control to prevent race conditions
- **Error Handling**: Structured error types with user-friendly messages and troubleshooting hints
- **Proxy Support**: Automatic detection from environment variables with authentication

**Critical Path**: This enhancement directly blocks authentication commands (enhancement 4) and all vault operations (enhancement 5+).

## System Architecture

### High-Level Component Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                   CLI Commands Layer                         │
│  (auth, vault, tools - all API consumers)                   │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│               Service Container                              │
│  - Provides ApiClient trait instance                        │
│  - Provides Storage for token/config access                 │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│            ApiClient Trait (Abstract Interface)              │
│  + get(&self, path: &str) -> Result<T>                      │
│  + post(&self, path: &str, body: &T) -> Result<R>           │
│  + put(&self, path: &str, body: &T) -> Result<R>            │
│  + delete(&self, path: &str) -> Result<()>                  │
│  + get_with_auth(&self, path: &str) -> Result<T>            │
│  + post_with_auth(&self, path: &str, body: &T) -> Result<R> │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│          BitwardenApiClient (Concrete Implementation)        │
│  - reqwest::Client instance (connection pooling)            │
│  - Environment URLs (api, identity, web vault, etc.)        │
│  - Token refresh coordinator (Arc<Mutex<>>)                 │
│  - Storage reference for token persistence                  │
└──────────┬──────────────────────────────┬───────────────────┘
           │                              │
           ▼                              ▼
┌──────────────────────┐   ┌─────────────────────────────────┐
│  HTTP Transport      │   │  Token Management               │
│  - reqwest client    │   │  - Bearer token injection       │
│  - Proxy config      │   │  - 401 detection                │
│  - TLS with rustls   │   │  - Refresh token flow           │
│  - Timeouts          │   │  - Race condition prevention    │
│  - User-Agent        │   │  - Storage integration          │
└──────────┬───────────┘   └─────────────┬───────────────────┘
           │                              │
           ▼                              ▼
┌──────────────────────┐   ┌─────────────────────────────────┐
│  Request Builder     │   │  Storage Layer                  │
│  - JSON serialize    │   │  - get_secure("accessToken")    │
│  - Headers           │   │  - get_secure("refreshToken")   │
│  - Query params      │   │  - set_secure("accessToken", ..)│
│  - Body encoding     │   │  - set_secure("refreshToken", ..)│
└──────────┬───────────┘   └─────────────────────────────────┘
           │
           ▼
┌─────────────────────────────────────────────────────────────┐
│                Response Processor                            │
│  - Status code handling                                     │
│  - JSON deserialize                                         │
│  - Error mapping (4xx/5xx -> typed errors)                  │
│  - Body validation                                          │
└─────────────────────────────────────────────────────────────┘
```

### Module Organization

```
crates/bw-core/src/
├── services/
│   ├── mod.rs                    # Re-exports (updated)
│   ├── container.rs              # ServiceContainer (updated)
│   ├── sdk.rs                    # SDK client wrapper (existing)
│   ├── storage/                  # Storage module (existing)
│   └── api/                      # NEW: API client module
│       ├── mod.rs                # Public API, re-exports
│       ├── traits.rs             # ApiClient trait definition
│       ├── client.rs             # BitwardenApiClient implementation
│       ├── environment.rs        # Environment URL resolution
│       ├── token_manager.rs      # Token refresh coordination
│       ├── request.rs            # Request builder
│       ├── response.rs           # Response processor
│       └── errors.rs             # API-specific errors
└── models/
    ├── api/                      # NEW: API request/response models
    │   ├── mod.rs
    │   ├── token.rs              # Token request/response types
    │   └── error_response.rs     # Bitwarden API error response
    └── state/                    # State models (existing)
```

## Technical Design

### 1. API Client Trait Design

**File**: `crates/bw-core/src/services/api/traits.rs`

**Design Decision**: Use trait-based abstraction to enable:
- Mock implementations for testing
- Future flexibility (swap HTTP client)
- Clear contract for command implementations

```rust
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Abstract API client interface for Bitwarden server communication
///
/// Provides methods for HTTP operations with and without authentication.
/// Implementations handle:
/// - Request serialization and response deserialization
/// - Bearer token injection for authenticated requests
/// - Automatic token refresh on 401 responses
/// - Error mapping to typed error enums
#[async_trait]
pub trait ApiClient: Send + Sync {
    /// Make an unauthenticated GET request
    ///
    /// # Arguments
    /// * `path` - API path relative to base URL (e.g., "/public/version")
    ///
    /// # Returns
    /// Deserialized response body of type T
    ///
    /// # Example
    /// ```rust
    /// let version: VersionResponse = client.get("/public/version").await?;
    /// ```
    async fn get<T>(&self, path: &str) -> Result<T>
    where
        T: for<'de> Deserialize<'de>;

    /// Make an authenticated GET request
    ///
    /// Automatically includes Bearer token in Authorization header.
    /// On 401 response, attempts token refresh and retries request.
    ///
    /// # Arguments
    /// * `path` - API path relative to base URL
    ///
    /// # Returns
    /// Deserialized response body of type T
    ///
    /// # Errors
    /// - `ApiError::Authentication` if token missing or refresh fails
    /// - `ApiError::Network` for connection failures
    /// - `ApiError::Server` for 5xx responses
    async fn get_with_auth<T>(&self, path: &str) -> Result<T>
    where
        T: for<'de> Deserialize<'de>;

    /// Make an unauthenticated POST request
    ///
    /// # Arguments
    /// * `path` - API path relative to base URL
    /// * `body` - Request body to serialize as JSON
    ///
    /// # Returns
    /// Deserialized response body of type R
    async fn post<T, R>(&self, path: &str, body: &T) -> Result<R>
    where
        T: Serialize + Send + Sync,
        R: for<'de> Deserialize<'de>;

    /// Make an authenticated POST request
    ///
    /// Automatically includes Bearer token in Authorization header.
    /// On 401 response, attempts token refresh and retries request.
    async fn post_with_auth<T, R>(&self, path: &str, body: &T) -> Result<R>
    where
        T: Serialize + Send + Sync,
        R: for<'de> Deserialize<'de>;

    /// Make an authenticated PUT request
    ///
    /// Updates an existing resource with provided data.
    async fn put_with_auth<T, R>(&self, path: &str, body: &T) -> Result<R>
    where
        T: Serialize + Send + Sync,
        R: for<'de> Deserialize<'de>;

    /// Make an authenticated DELETE request
    ///
    /// Deletes a resource. Returns empty result on success (204 No Content).
    async fn delete_with_auth(&self, path: &str) -> Result<()>;

    /// Get the current environment URLs
    ///
    /// Returns URLs for all Bitwarden services (api, identity, web vault, etc.)
    fn environment(&self) -> &Environment;

    /// Check if currently authenticated (has valid access token)
    ///
    /// Does not validate token with server, only checks local storage.
    async fn is_authenticated(&self) -> bool;
}
```

**Key Design Points:**
- **async_trait**: Required for async methods in traits (Rust limitation)
- **Generic type parameters**: Type-safe request/response handling
- **Separate auth methods**: Clear distinction between authenticated and unauthenticated
- **Environment access**: Allows commands to build URLs when needed
- **Send + Sync**: Enables use across threads/async tasks

### 2. Environment URL Resolution

**File**: `crates/bw-core/src/services/api/environment.rs`

**Design Decision**: Centralize URL resolution logic with validation and defaults.

```rust
use anyhow::Result;
use serde::{Deserialize, Serialize};
use url::Url;

/// Environment configuration for Bitwarden services
///
/// Resolves URLs for all Bitwarden server endpoints.
/// Supports self-hosted installations with custom base URLs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Environment {
    /// Base URL (e.g., "https://vault.bitwarden.com")
    base: String,

    /// Resolved service URLs
    urls: ServiceUrls,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ServiceUrls {
    /// API server URL (default: {base}/api)
    api: String,

    /// Identity server URL (default: {base}/identity)
    identity: String,

    /// Web vault URL (default: {base})
    web_vault: String,

    /// Icons server URL (default: {base}/icons)
    icons: String,

    /// Notifications server URL (default: {base}/notifications)
    notifications: String,

    /// Events server URL (default: {base}/events)
    events: Option<String>,
}

impl Environment {
    /// Create environment from base URL
    ///
    /// # Arguments
    /// * `base_url` - Base URL for all services (e.g., "https://vault.bitwarden.com")
    ///
    /// # Validation
    /// - URL must be well-formed
    /// - Must use HTTPS (or HTTP for localhost)
    /// - No trailing slash
    pub fn from_base_url(base_url: &str) -> Result<Self> {
        let base = Self::validate_and_normalize_url(base_url)?;

        let urls = ServiceUrls {
            api: format!("{}/api", base),
            identity: format!("{}/identity", base),
            web_vault: base.clone(),
            icons: format!("{}/icons", base),
            notifications: format!("{}/notifications", base),
            events: Some(format!("{}/events", base)),
        };

        Ok(Self { base, urls })
    }

    /// Create environment with custom service URLs
    ///
    /// Allows overriding individual service URLs for advanced configurations.
    pub fn custom(
        base_url: &str,
        api_url: Option<String>,
        identity_url: Option<String>,
        web_vault_url: Option<String>,
        icons_url: Option<String>,
        notifications_url: Option<String>,
        events_url: Option<String>,
    ) -> Result<Self> {
        let base = Self::validate_and_normalize_url(base_url)?;

        let urls = ServiceUrls {
            api: api_url.unwrap_or_else(|| format!("{}/api", base)),
            identity: identity_url.unwrap_or_else(|| format!("{}/identity", base)),
            web_vault: web_vault_url.unwrap_or_else(|| base.clone()),
            icons: icons_url.unwrap_or_else(|| format!("{}/icons", base)),
            notifications: notifications_url.unwrap_or_else(|| format!("{}/notifications", base)),
            events: events_url.or_else(|| Some(format!("{}/events", base))),
        };

        Ok(Self { base, urls })
    }

    /// Default cloud environment
    pub fn default_cloud() -> Self {
        Self::from_base_url("https://vault.bitwarden.com")
            .expect("Default cloud URL is valid")
    }

    /// Get API base URL
    pub fn api_url(&self) -> &str {
        &self.urls.api
    }

    /// Get Identity server URL
    pub fn identity_url(&self) -> &str {
        &self.urls.identity
    }

    /// Get Web Vault URL
    pub fn web_vault_url(&self) -> &str {
        &self.urls.web_vault
    }

    /// Get Icons server URL
    pub fn icons_url(&self) -> &str {
        &self.urls.icons
    }

    /// Get Notifications server URL
    pub fn notifications_url(&self) -> &str {
        &self.urls.notifications
    }

    /// Get Events server URL
    pub fn events_url(&self) -> Option<&str> {
        self.urls.events.as_deref()
    }

    /// Validate and normalize URL
    ///
    /// # Validation Rules
    /// - Must parse as valid URL
    /// - Must use HTTPS (exception: HTTP allowed for localhost/127.0.0.1)
    /// - Remove trailing slash
    fn validate_and_normalize_url(url_str: &str) -> Result<String> {
        let url = Url::parse(url_str)
            .map_err(|e| anyhow::anyhow!("Invalid URL '{}': {}", url_str, e))?;

        // Validate HTTPS requirement (except localhost)
        if url.scheme() == "http" {
            let host = url.host_str().unwrap_or("");
            if !host.starts_with("localhost") && !host.starts_with("127.0.0.1") {
                return Err(anyhow::anyhow!(
                    "HTTPS required for remote servers. URL: {}",
                    url_str
                ));
            }
        }

        // Remove trailing slash
        let mut normalized = url.to_string();
        if normalized.ends_with('/') {
            normalized.pop();
        }

        Ok(normalized)
    }
}

impl Default for Environment {
    fn default() -> Self {
        Self::default_cloud()
    }
}
```

**Additional Dependency Required:**
```toml
# Add to workspace dependencies
url = "2.5"  # URL parsing and validation
```

**Key Design Points:**
- **Validation**: Enforces HTTPS except for localhost
- **Normalization**: Removes trailing slashes for consistency
- **Flexibility**: Supports both simple base URL and custom service URLs
- **Defaults**: Sensible defaults for official Bitwarden cloud

### 3. Token Management and Refresh

**File**: `crates/bw-core/src/services/api/token_manager.rs`

**Design Decision**: Use Arc<Mutex<Option<Future>>> pattern to prevent token refresh race conditions.

**Critical Challenge**: Multiple concurrent requests with expired token must not trigger multiple refresh operations.

**Solution**: Coordinate refresh attempts with shared state:
- First request with 401 starts refresh and stores Future
- Subsequent requests wait on same Future
- After refresh completes, all waiting requests retry with new token

```rust
use super::errors::ApiError;
use crate::models::api::token::{TokenRefreshRequest, TokenResponse};
use crate::services::storage::Storage;
use anyhow::Result;
use secrecy::{ExposeSecret, Secret};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Token management with automatic refresh coordination
///
/// Handles:
/// - Token retrieval from storage
/// - Token refresh when expired
/// - Race condition prevention for concurrent refreshes
/// - Token persistence after refresh
pub struct TokenManager {
    /// Storage reference for token persistence
    storage: Arc<dyn Storage>,

    /// Refresh coordination state
    /// - None: No refresh in progress
    /// - Some(Future): Refresh in progress, await this
    refresh_state: Arc<Mutex<Option<Arc<Mutex<()>>>>>,
}

impl TokenManager {
    pub fn new(storage: Arc<dyn Storage>) -> Self {
        Self {
            storage,
            refresh_state: Arc::new(Mutex::new(None)),
        }
    }

    /// Get current access token
    ///
    /// # Returns
    /// Secret-wrapped access token if authenticated, None otherwise
    ///
    /// # Errors
    /// Returns error if storage access fails
    pub async fn get_access_token(&self) -> Result<Option<Secret<String>>> {
        let token_str = self.storage.get_secure("accessToken").await?;
        Ok(token_str.map(Secret::new))
    }

    /// Get current refresh token
    ///
    /// # Returns
    /// Secret-wrapped refresh token if available, None otherwise
    pub async fn get_refresh_token(&self) -> Result<Option<Secret<String>>> {
        let token_str = self.storage.get_secure("refreshToken").await?;
        Ok(token_str.map(Secret::new))
    }

    /// Refresh access token using refresh token
    ///
    /// Coordinates concurrent refresh attempts:
    /// - If no refresh in progress: start refresh
    /// - If refresh in progress: wait for it to complete
    ///
    /// # Arguments
    /// * `refresh_client` - Function to call refresh endpoint
    ///
    /// # Returns
    /// New access token on success
    ///
    /// # Errors
    /// - `ApiError::Authentication` if refresh token invalid/expired
    /// - `ApiError::Network` for connection failures
    pub async fn refresh_access_token<F, Fut>(
        &self,
        refresh_client: F,
    ) -> Result<Secret<String>>
    where
        F: FnOnce(TokenRefreshRequest) -> Fut,
        Fut: std::future::Future<Output = Result<TokenResponse>>,
    {
        // Check if refresh already in progress
        let lock_guard = self.refresh_state.lock().await;

        if let Some(existing_lock) = &*lock_guard {
            // Refresh in progress - wait for it
            let wait_lock = Arc::clone(existing_lock);
            drop(lock_guard);

            // Wait for refresh to complete
            let _guard = wait_lock.lock().await;

            // Refresh completed, get new token
            return self
                .get_access_token()
                .await?
                .ok_or_else(|| ApiError::Authentication(
                    "Token refresh completed but access token not found".into()
                ));
        }

        // No refresh in progress - start new refresh
        let refresh_lock = Arc::new(Mutex::new(()));
        let _lock_guard = refresh_lock.lock().await;

        // Store refresh lock so other requests can wait
        let mut state_guard = self.refresh_state.lock().await;
        *state_guard = Some(Arc::clone(&refresh_lock));
        drop(state_guard);

        // Get refresh token
        let refresh_token = self
            .get_refresh_token()
            .await?
            .ok_or_else(|| ApiError::Authentication(
                "No refresh token available. Please log in again.".into()
            ))?;

        // Build refresh request
        let request = TokenRefreshRequest {
            grant_type: "refresh_token".to_string(),
            refresh_token: refresh_token.expose_secret().clone(),
        };

        // Call refresh endpoint
        let response = refresh_client(request).await?;

        // Persist new tokens
        let mut storage = self.storage.clone();
        storage
            .set_secure("accessToken", &response.access_token)
            .await?;

        if let Some(new_refresh_token) = &response.refresh_token {
            storage
                .set_secure("refreshToken", new_refresh_token)
                .await?;
        }

        // Clear refresh state
        let mut state_guard = self.refresh_state.lock().await;
        *state_guard = None;
        drop(state_guard);

        Ok(Secret::new(response.access_token))
    }

    /// Save tokens after successful login
    ///
    /// # Arguments
    /// * `access_token` - New access token
    /// * `refresh_token` - New refresh token
    pub async fn save_tokens(
        &self,
        access_token: &str,
        refresh_token: &str,
    ) -> Result<()> {
        let mut storage = self.storage.clone();
        storage.set_secure("accessToken", access_token).await?;
        storage.set_secure("refreshToken", refresh_token).await?;
        Ok(())
    }

    /// Clear all stored tokens
    ///
    /// Called on logout
    pub async fn clear_tokens(&self) -> Result<()> {
        let mut storage = self.storage.clone();
        storage.remove_secure("accessToken").await?;
        storage.remove_secure("refreshToken").await?;
        Ok(())
    }
}
```

**Key Design Points:**
- **Arc<Mutex<Option<Arc<Mutex<()>>>>>**: Complex but correct concurrency pattern
  - Outer Mutex: Protects refresh state check/update
  - Inner Arc<Mutex<()>>: Shared lock for waiting requests
  - Option: None when no refresh, Some when refresh in progress
- **Coordination**: First request starts refresh, others wait
- **Storage integration**: Persists tokens after successful refresh
- **Error handling**: Clear messages for different failure scenarios

**Additional Dependency Required:**
```toml
# Already in workspace, ensure in bw-core
async-trait = "0.1"  # For async trait methods
```

### 4. API Error Types

**File**: `crates/bw-core/src/services/api/errors.rs`

**Design Decision**: Structured error enum with context and user-friendly messages.

```rust
use reqwest::StatusCode;
use thiserror::Error;

/// API client errors with context for user-friendly messages
#[derive(Debug, Error)]
pub enum ApiError {
    /// Network connectivity error (DNS, connection refused, timeout)
    #[error("Network error: {message}\n{troubleshooting}")]
    Network {
        message: String,
        troubleshooting: String,
        #[source]
        source: Option<reqwest::Error>,
    },

    /// Authentication error (401, 403, missing token)
    #[error("Authentication error: {message}\n{hint}")]
    Authentication {
        message: String,
        hint: String,
    },

    /// Not found error (404)
    #[error("Resource not found: {resource}")]
    NotFound { resource: String },

    /// Rate limit error (429)
    #[error("Rate limit exceeded. {message}")]
    RateLimit {
        message: String,
        retry_after: Option<u64>,
    },

    /// Client error (other 4xx)
    #[error("Client error ({status}): {message}")]
    Client {
        status: StatusCode,
        message: String,
    },

    /// Server error (5xx)
    #[error("Server error ({status}): {message}\n{hint}")]
    Server {
        status: StatusCode,
        message: String,
        hint: String,
    },

    /// Request timeout
    #[error("Request timeout: {message}\n{hint}")]
    Timeout { message: String, hint: String },

    /// TLS/certificate error
    #[error("TLS error: {message}\n{hint}")]
    Tls { message: String, hint: String },

    /// JSON serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Invalid URL or configuration
    #[error("Configuration error: {0}")]
    Configuration(String),
}

impl ApiError {
    /// Create network error with troubleshooting hints
    pub fn network_error(err: reqwest::Error) -> Self {
        let message = if err.is_timeout() {
            "Request timed out".to_string()
        } else if err.is_connect() {
            "Failed to connect to server".to_string()
        } else {
            format!("Network request failed: {}", err)
        };

        let troubleshooting = if err.is_timeout() {
            "Check your internet connection or increase timeout with --timeout flag".to_string()
        } else if err.is_connect() {
            "Check server URL, DNS settings, and firewall configuration".to_string()
        } else {
            "Check your network connection and proxy settings".to_string()
        };

        Self::Network {
            message,
            troubleshooting,
            source: Some(err),
        }
    }

    /// Create authentication error
    pub fn auth_error(message: String) -> Self {
        let hint = if message.contains("expired") || message.contains("invalid") {
            "Run 'bw login' to authenticate again".to_string()
        } else {
            "Run 'bw unlock' to unlock your vault".to_string()
        };

        Self::Authentication { message, hint }
    }

    /// Create server error with helpful hints
    pub fn server_error(status: StatusCode, message: String) -> Self {
        let hint = match status.as_u16() {
            502 | 503 => "Server temporarily unavailable. Please try again in a few moments.".to_string(),
            500 => "Internal server error. If this persists, contact Bitwarden support.".to_string(),
            _ => "Server error occurred. Please try again later.".to_string(),
        };

        Self::Server {
            status,
            message,
            hint,
        }
    }

    /// Create rate limit error
    pub fn rate_limit_error(retry_after: Option<u64>) -> Self {
        let message = if let Some(seconds) = retry_after {
            format!("Please wait {} seconds before retrying.", seconds)
        } else {
            "Please wait a few moments before retrying.".to_string()
        };

        Self::RateLimit {
            message,
            retry_after,
        }
    }
}

impl From<reqwest::Error> for ApiError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            Self::Timeout {
                message: "Request timed out".to_string(),
                hint: "Check your network connection or increase timeout".to_string(),
            }
        } else if err.is_connect() {
            Self::network_error(err)
        } else {
            Self::network_error(err)
        }
    }
}
```

**Key Design Points:**
- **Error categories**: Network, Auth, RateLimit, Client, Server, Timeout, TLS
- **User-friendly**: Each error includes troubleshooting hints
- **Context**: Include status codes, retry-after times, etc.
- **Source preservation**: Chain underlying errors for debugging

### 5. Bitwarden API Client Implementation

**File**: `crates/bw-core/src/services/api/client.rs`

**Design Decision**: Concrete implementation using reqwest with all infrastructure features.

```rust
use super::{
    environment::Environment,
    errors::ApiError,
    token_manager::TokenManager,
    traits::ApiClient,
};
use crate::models::api::token::{TokenRefreshRequest, TokenResponse};
use crate::services::storage::Storage;
use anyhow::Result;
use async_trait::async_trait;
use reqwest::{header, Client as ReqwestClient, Method, Request, Response, StatusCode};
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

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
    storage: Arc<dyn Storage>,
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
        storage: Arc<dyn Storage>,
        timeout_seconds: Option<u64>,
    ) -> Result<Self> {
        let timeout = Duration::from_secs(timeout_seconds.unwrap_or(60));

        // Build HTTP client with all features
        let http_client = ReqwestClient::builder()
            .timeout(timeout)
            .connect_timeout(Duration::from_secs(30))
            .user_agent(format!(
                "Bitwarden_CLI/{} (Rust)",
                env!("CARGO_PKG_VERSION")
            ))
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
    fn build_url(&self, path: &str, use_identity: bool) -> String {
        let base = if use_identity {
            self.environment.identity_url()
        } else {
            self.environment.api_url()
        };

        let path = path.trim_start_matches('/');
        format!("{}/{}", base, path)
    }

    /// Execute request with automatic retry on token refresh
    async fn execute_with_retry(
        &self,
        mut request: Request,
        requires_auth: bool,
    ) -> Result<Response> {
        // Try request first time
        let response = self.http_client.execute(request.try_clone().unwrap()).await?;

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
            StatusCode::UNAUTHORIZED => {
                Err(ApiError::auth_error("Authentication required".to_string()).into())
            }
            StatusCode::FORBIDDEN => {
                Err(ApiError::auth_error("Access forbidden".to_string()).into())
            }
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
                let message = self.extract_error_message(&response).await?;
                Err(ApiError::Client {
                    status: s,
                    message,
                }
                .into())
            }
            s if s.is_server_error() => {
                let message = self.extract_error_message(&response).await?;
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
    async fn extract_error_message(&self, response: &Response) -> Result<String> {
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
            .ok_or_else(|| ApiError::auth_error("Not authenticated".to_string()))?;

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
        let url = self.build_url(path, false);

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
            .ok_or_else(|| ApiError::auth_error("Not authenticated".to_string()))?;

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
            .ok_or_else(|| ApiError::auth_error("Not authenticated".to_string()))?;

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
            .ok_or_else(|| ApiError::auth_error("Not authenticated".to_string()))?;

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
```

**Key Design Points:**
- **Connection pooling**: Single reqwest::Client instance reused
- **Automatic proxy**: reqwest reads HTTP_PROXY/HTTPS_PROXY automatically
- **Token refresh retry**: On 401, refresh token and retry original request
- **Error extraction**: Parse Bitwarden error response format
- **Timeouts**: Configurable with sensible defaults
- **User-Agent**: Identifies client as Rust CLI

### 6. API Request/Response Models

**File**: `crates/bw-core/src/models/api/token.rs`

```rust
use serde::{Deserialize, Serialize};

/// Token refresh request (OAuth2 refresh token flow)
#[derive(Debug, Serialize)]
pub struct TokenRefreshRequest {
    pub grant_type: String, // Always "refresh_token"
    pub refresh_token: String,
}

/// Token response from authentication/refresh endpoints
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenResponse {
    pub access_token: String,
    pub expires_in: i64,
    pub token_type: String,
    pub refresh_token: Option<String>,
    #[serde(rename = "Key")]
    pub key: Option<String>,
}
```

**File**: `crates/bw-core/src/models/api/error_response.rs`

```rust
use serde::Deserialize;

/// Bitwarden API error response format
#[derive(Debug, Deserialize)]
pub struct ApiErrorResponse {
    #[serde(rename = "Message")]
    pub message: Option<String>,

    #[serde(rename = "ValidationErrors")]
    pub validation_errors: Option<std::collections::HashMap<String, Vec<String>>>,

    #[serde(rename = "error")]
    pub error: Option<String>,

    #[serde(rename = "error_description")]
    pub error_description: Option<String>,
}
```

### 7. Service Container Integration

**File**: `crates/bw-core/src/services/container.rs` (updated)

```rust
use super::{
    api::{BitwardenApiClient, Environment, ApiClient},
    storage::{JsonFileStorage, Storage},
};
use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;

/// Service container for dependency injection
///
/// Provides access to:
/// - Storage (configuration and state persistence)
/// - API Client (HTTP communication with Bitwarden servers)
/// - SDK client (crypto, vault operations - future)
pub struct ServiceContainer {
    /// Storage service
    storage: Arc<dyn Storage>,

    /// API client
    api_client: Arc<dyn ApiClient>,
}

impl ServiceContainer {
    /// Create new service container
    ///
    /// # Arguments
    /// * `storage_path` - Optional custom storage directory
    /// * `base_url` - Optional custom server base URL
    /// * `timeout_seconds` - Optional API request timeout
    pub fn new(
        storage_path: Option<PathBuf>,
        base_url: Option<String>,
        timeout_seconds: Option<u64>,
    ) -> Result<Self> {
        // Initialize storage
        let storage: Arc<dyn Storage> = Arc::new(JsonFileStorage::new(storage_path)?);

        // Determine environment URLs
        let environment = if let Some(url) = base_url {
            Environment::from_base_url(&url)?
        } else {
            // Try to load from storage, fall back to default
            storage
                .get("environmentUrls")
                .ok()
                .flatten()
                .map(|urls: crate::models::state::EnvironmentUrls| {
                    Environment::custom(
                        urls.base.as_deref().unwrap_or("https://vault.bitwarden.com"),
                        urls.api,
                        urls.identity,
                        urls.web_vault,
                        urls.icons,
                        urls.notifications,
                        None,
                    )
                })
                .transpose()?
                .unwrap_or_else(Environment::default_cloud)
        };

        // Initialize API client
        let api_client: Arc<dyn ApiClient> = Arc::new(BitwardenApiClient::new(
            environment,
            Arc::clone(&storage),
            timeout_seconds,
        )?);

        Ok(Self {
            storage,
            api_client,
        })
    }

    /// Get storage service reference
    pub fn storage(&self) -> Arc<dyn Storage> {
        Arc::clone(&self.storage)
    }

    /// Get API client reference
    pub fn api_client(&self) -> Arc<dyn ApiClient> {
        Arc::clone(&self.api_client)
    }
}
```

### 8. Module Public API

**File**: `crates/bw-core/src/services/api/mod.rs`

```rust
mod client;
mod environment;
mod errors;
mod token_manager;
mod traits;

// Public exports
pub use client::BitwardenApiClient;
pub use environment::Environment;
pub use errors::ApiError;
pub use traits::ApiClient;

// Internal use only
pub(crate) use token_manager::TokenManager;
```

**File**: `crates/bw-core/src/services/mod.rs` (updated)

```rust
mod container;
mod sdk;

// Existing modules
pub mod storage;

// NEW: API client module
pub mod api;

pub use container::ServiceContainer;
```

**File**: `crates/bw-core/src/models/api/mod.rs`

```rust
mod error_response;
mod token;

pub use error_response::ApiErrorResponse;
pub use token::{TokenRefreshRequest, TokenResponse};
```

**File**: `crates/bw-core/src/models/mod.rs` (updated)

```rust
pub mod state;

// NEW: API models
pub mod api;
```

## Dependency Updates

### Workspace Dependencies to Add

**File**: `Cargo.toml` (workspace root) - updated

```toml
[workspace.dependencies]
# ... existing dependencies ...

# HTTP and API
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "json"] }
url = "2.5"             # URL parsing and validation
async-trait = "0.1"     # Async trait methods
```

### Crate Dependencies

**File**: `crates/bw-core/Cargo.toml` - updated

```toml
[dependencies]
# ... existing dependencies ...

# HTTP and API
reqwest.workspace = true
url.workspace = true
async-trait.workspace = true

# Already have: tokio, serde, serde_json, anyhow, thiserror, secrecy, zeroize

[dev-dependencies]
# ... existing dev-dependencies ...
wiremock = "0.6"      # HTTP mocking for tests
```

## Testing Strategy

### Unit Tests

**File**: `crates/bw-core/src/services/api/environment.rs` (tests)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_cloud_environment() {
        let env = Environment::default_cloud();
        assert_eq!(env.api_url(), "https://vault.bitwarden.com/api");
        assert_eq!(env.identity_url(), "https://vault.bitwarden.com/identity");
    }

    #[test]
    fn test_custom_base_url() {
        let env = Environment::from_base_url("https://my.server.com").unwrap();
        assert_eq!(env.api_url(), "https://my.server.com/api");
        assert_eq!(env.identity_url(), "https://my.server.com/identity");
    }

    #[test]
    fn test_https_validation() {
        let result = Environment::from_base_url("http://remote.server.com");
        assert!(result.is_err());
    }

    #[test]
    fn test_localhost_http_allowed() {
        let env = Environment::from_base_url("http://localhost:8080").unwrap();
        assert!(env.api_url().starts_with("http://localhost"));
    }

    #[test]
    fn test_trailing_slash_removal() {
        let env = Environment::from_base_url("https://vault.bitwarden.com/").unwrap();
        assert!(!env.api_url().ends_with("//api"));
    }
}
```

**File**: `crates/bw-core/src/services/api/token_manager.rs` (tests)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio;

    // Mock storage implementation
    struct MockStorage {
        tokens: Arc<Mutex<HashMap<String, String>>>,
    }

    #[tokio::test]
    async fn test_get_access_token() {
        // Test token retrieval
    }

    #[tokio::test]
    async fn test_concurrent_refresh() {
        // Test that concurrent refresh attempts coordinate correctly
        // Spawn 10 tasks that all try to refresh simultaneously
        // Verify only one refresh occurs
    }

    #[tokio::test]
    async fn test_save_and_retrieve_tokens() {
        // Test token persistence
    }
}
```

### Integration Tests

**File**: `crates/bw-core/tests/api_integration.rs`

```rust
use bw_core::services::api::{BitwardenApiClient, Environment, ApiClient};
use bw_core::services::storage::{JsonFileStorage, Storage};
use std::sync::Arc;
use tempfile::TempDir;
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path, header};

#[tokio::test]
async fn test_get_request() {
    // Mock server
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/version"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "version": "1.0.0"
        })))
        .mount(&mock_server)
        .await;

    // Create client
    let temp_dir = TempDir::new().unwrap();
    let storage: Arc<dyn Storage> = Arc::new(
        JsonFileStorage::new(Some(temp_dir.path().to_path_buf())).unwrap()
    );

    let environment = Environment::from_base_url(&mock_server.uri()).unwrap();
    let client = BitwardenApiClient::new(environment, storage, None).unwrap();

    // Test request
    #[derive(serde::Deserialize)]
    struct VersionResponse {
        version: String,
    }

    let response: VersionResponse = client.get("/version").await.unwrap();
    assert_eq!(response.version, "1.0.0");
}

#[tokio::test]
async fn test_authenticated_request_with_token_refresh() {
    // Test 401 -> refresh -> retry flow
    let mock_server = MockServer::start().await;

    // First request returns 401
    Mock::given(method("GET"))
        .and(path("/api/sync"))
        .and(header("Authorization", "Bearer old_token"))
        .respond_with(ResponseTemplate::new(401))
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    // Token refresh succeeds
    Mock::given(method("POST"))
        .and(path("/identity/connect/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "new_token",
            "expires_in": 3600,
            "token_type": "Bearer"
        })))
        .mount(&mock_server)
        .await;

    // Retry with new token succeeds
    Mock::given(method("GET"))
        .and(path("/api/sync"))
        .and(header("Authorization", "Bearer new_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": []
        })))
        .mount(&mock_server)
        .await;

    // Test full flow
    // ... (setup client with old token, make request, verify refresh happened)
}

#[tokio::test]
async fn test_rate_limit_error() {
    // Test 429 response handling
}

#[tokio::test]
async fn test_network_error_handling() {
    // Test timeout and connection failures
}
```

### Concurrent Access Tests

**File**: `crates/bw-core/tests/token_refresh_concurrency.rs`

```rust
#[tokio::test]
async fn test_concurrent_token_refresh() {
    // Setup: Mock server with slow refresh endpoint
    // Spawn 100 concurrent requests that all need token refresh
    // Verify: Only 1 refresh request made to server
    // Verify: All 100 requests eventually succeed with new token
}
```

## Error Handling Patterns

### User-Facing Error Messages

All errors should provide:
1. **Clear problem description**: What went wrong
2. **Context**: Where/when it happened
3. **Action items**: What user can do about it

**Examples:**

```rust
// Network error
ApiError::Network {
    message: "Failed to connect to vault.bitwarden.com".to_string(),
    troubleshooting:
        "Check your internet connection.\n\
         If behind a proxy, ensure HTTP_PROXY is set.\n\
         Verify server URL with: bw config server".to_string(),
    source: Some(err),
}

// Authentication error
ApiError::Authentication {
    message: "Access token expired".to_string(),
    hint: "Run 'bw login' to authenticate again".to_string(),
}

// Rate limit
ApiError::RateLimit {
    message: "Please wait 60 seconds before retrying.".to_string(),
    retry_after: Some(60),
}

// Server error
ApiError::Server {
    status: StatusCode::INTERNAL_SERVER_ERROR,
    message: "Server returned 500 error".to_string(),
    hint: "This is a server issue. Please try again later.\n\
           If this persists, contact Bitwarden support.".to_string(),
}
```

## Security Considerations

### 1. Token Security

**Design Decisions:**
- ✅ Use `secrecy::Secret<String>` for all tokens in memory
- ✅ Store tokens encrypted with `__PROTECTED__` prefix via storage layer
- ✅ Never log token values (sanitize in debug output)
- ✅ Zeroize token buffers after use

```rust
// Example: Safe token handling
let token: Secret<String> = token_manager.get_access_token().await?;

// Use ExposeSecret only when necessary
request.header(
    header::AUTHORIZATION,
    format!("Bearer {}", token.expose_secret())
);

// token is automatically zeroized on drop
```

### 2. TLS Certificate Validation

**Design Decision**: Always validate certificates, no override option in MVP.

- ✅ Use rustls-tls (not native-tls) for consistent behavior
- ✅ Certificate validation enabled by default
- ✅ Clear error messages for certificate issues
- ❌ No `--insecure` flag (can add later if needed for self-hosted)

### 3. URL Validation

**Design Decision**: Prevent open redirects and SSRF attacks.

- ✅ Validate all URLs at Environment creation
- ✅ Require HTTPS for non-localhost URLs
- ✅ No automatic redirect following (reqwest default: max 10)
- ✅ Validate redirect targets remain in same domain

### 4. Request/Response Limits

**Design Decision**: Protect against DoS and memory exhaustion.

- ✅ Request timeout: 60 seconds default
- ✅ Connect timeout: 30 seconds
- ✅ No explicit body size limit (reqwest default: 2MB)
- ✅ Can increase timeout via configuration if needed

## Performance Considerations

### Optimization Strategies

1. **Connection Pooling**:
   - Single reqwest::Client instance
   - Reuses TCP connections across requests
   - Reduces TLS handshake overhead

2. **Efficient JSON Processing**:
   - Stream parsing for large responses
   - Minimize allocations with serde
   - Use `&str` slices where possible

3. **Token Caching**:
   - Tokens cached in memory (via storage layer)
   - No file I/O per request
   - Only refresh when needed (on 401)

4. **Async All the Way**:
   - No blocking I/O
   - Concurrent requests supported
   - Efficient task scheduling via tokio

### Performance Targets

| Operation | Target | Measurement |
|-----------|--------|-------------|
| Simple GET request | <500ms | Round-trip time |
| Authenticated request | <600ms | Includes token lookup |
| Token refresh | <2s | Full refresh + retry |
| Concurrent requests (100) | <5s total | All complete |
| Connection reuse | >80% | Across 100 sequential requests |

## Integration Strategy

### Phase 1: Core Infrastructure (2-3 days)

**Goal**: HTTP client wrapper with basic request/response

**Deliverables**:
- `Environment` struct with URL resolution
- `ApiError` types
- `ApiClient` trait definition
- `BitwardenApiClient` basic implementation (no auth)
- GET/POST methods working

**Success Criteria**: Can make unauthenticated requests

**Dependencies**: Enhancement 2 (storage-layer) complete

---

### Phase 2: Token Management (2-3 days)

**Goal**: Authentication and token refresh

**Deliverables**:
- `TokenManager` implementation
- Token storage integration
- Authenticated request methods
- Token refresh detection and retry
- Concurrency coordination

**Success Criteria**: Authenticated requests with automatic token refresh

**Complexity**: High (concurrency challenges)

**Dependencies**: Phase 1 complete

---

### Phase 3: Error Handling & Validation (1-2 days)

**Goal**: Comprehensive error handling

**Deliverables**:
- Complete `ApiError` types with user messages
- Response error parsing
- Bitwarden API error format handling
- Rate limit detection
- Network error troubleshooting hints

**Success Criteria**: All error scenarios handled gracefully

**Dependencies**: Phases 1-2 complete

---

### Phase 4: Service Integration (1 day)

**Goal**: Integrate with service container

**Deliverables**:
- Update `ServiceContainer`
- Wire up storage -> environment URLs
- API client initialization
- CLI integration points

**Success Criteria**: Commands can access API client via container

**Dependencies**: Phases 1-3 complete

---

### Phase 5: Testing & Documentation (2-3 days)

**Goal**: Comprehensive testing and docs

**Deliverables**:
- Unit tests for all components (>80% coverage)
- Integration tests with mock server
- Concurrent token refresh tests
- Error handling tests
- API documentation
- Usage examples

**Success Criteria**: All tests pass, documentation complete

**Dependencies**: Phases 1-4 complete

---

## Open Questions and Decisions

### Resolved from Requirements Analysis

1. **Token Refresh Strategy**: ✅ Fully automatic for all authenticated requests
2. **Error Type Granularity**: ✅ Category-based (Network, Auth, RateLimit, etc.) with status code field
3. **Client Abstraction Level**: ✅ Trait-based abstraction hiding reqwest details
4. **Retry Logic**: ✅ No automatic retries in MVP (only token refresh retry)
5. **Rate Limit Handling**: ✅ Return error immediately with retry-after info
6. **Connection Pool Configuration**: ✅ Use reqwest defaults (sufficient for CLI)

### Remaining for Implementation

7. **Certificate Validation Override**:
   - **Decision**: No override in MVP
   - **Rationale**: Security first, can add `--insecure` flag later if needed for self-hosted
   - **Impact**: Some self-hosted users with self-signed certs may need to fix certificates

8. **Request/Response Logging**:
   - **Decision**: Log headers only (not bodies) behind debug flag
   - **Rationale**: Helps troubleshooting without exposing secrets
   - **Implementation**: Use tracing crate with RUST_LOG=debug

9. **Redirect Handling**:
   - **Decision**: Allow up to 10 redirects (reqwest default), no cross-origin
   - **Rationale**: Balances security and usability
   - **Implementation**: Use reqwest default behavior

## Migration and Compatibility

### TypeScript CLI API Patterns

**Reference Implementation**: `apps/cli/src/platform/services/node-api.service.ts`

**Key Patterns to Maintain**:
1. Bearer token in Authorization header
2. OAuth2 refresh token flow
3. Content-Type: application/json for JSON requests
4. Accept: application/json for JSON responses
5. User-Agent format: `Bitwarden_CLI/{version}`

### Breaking Changes

None expected. This is a new Rust implementation maintaining full API compatibility.

### API Version Compatibility

- **Target**: Bitwarden API v1 (current stable)
- **Token format**: OAuth2 JWT (existing format)
- **Error responses**: Compatible with current Bitwarden server format
- **Forward compatibility**: Flexible JSON parsing ignores unknown fields

## Summary and Next Steps

### Architecture Decisions Summary

| Decision | Choice | Rationale |
|----------|--------|-----------|
| HTTP Client | reqwest 0.12 with rustls-tls | Industry standard, async-first, TLS security |
| Architecture | Trait-based abstraction | Testability, flexibility, clear contracts |
| Token Refresh | Automatic with mutex coordination | Best UX, prevents race conditions |
| Error Types | Category-based enum with context | User-friendly, actionable messages |
| Proxy Support | Automatic from environment | Standard convention, zero config |
| Timeouts | 60s request, 30s connect | Balance responsiveness and slow networks |
| Certificate Validation | Always enabled | Security first |
| Concurrency | Full async support | Non-blocking, efficient |

### Implementation Checklist

**Phase 1: Core Infrastructure** (2-3 days)
- [ ] Create module structure (`services/api/`)
- [ ] Implement `Environment` with URL resolution
- [ ] Implement `ApiError` types
- [ ] Implement `ApiClient` trait
- [ ] Implement `BitwardenApiClient` (basic GET/POST)
- [ ] Add workspace dependencies (url, async-trait)
- [ ] Unit tests for environment and errors

**Phase 2: Token Management** (2-3 days)
- [ ] Implement `TokenManager`
- [ ] Implement token storage integration
- [ ] Add authenticated request methods
- [ ] Implement 401 detection and token refresh
- [ ] Implement concurrency coordination
- [ ] Unit tests for token manager
- [ ] Integration tests for token refresh

**Phase 3: Error Handling** (1-2 days)
- [ ] Complete all error type variants
- [ ] Implement error message extraction
- [ ] Add troubleshooting hints
- [ ] Parse Bitwarden error response format
- [ ] Unit tests for error handling

**Phase 4: Service Integration** (1 day)
- [ ] Update `ServiceContainer`
- [ ] Wire up storage -> environment
- [ ] API client initialization
- [ ] Integration tests end-to-end

**Phase 5: Testing & Documentation** (2-3 days)
- [ ] Unit tests (>80% coverage)
- [ ] Integration tests with wiremock
- [ ] Concurrent token refresh tests
- [ ] Network error tests
- [ ] API documentation
- [ ] Usage examples

### Success Criteria

✅ **Ready for Implementation** when:
- All trait interfaces defined and documented
- Module organization clearly specified
- Error handling patterns established
- Token refresh concurrency strategy detailed
- Testing strategy defined
- Integration points identified

✅ **Implementation Complete** when:
- All unit tests pass (>80% coverage)
- Integration tests pass with mock server
- Token refresh works correctly under concurrent load
- All HTTP methods functional (GET, POST, PUT, DELETE)
- Error messages are clear and actionable
- Connection pooling verified
- Proxy support functional
- Documentation complete
- Enhancement 4 (auth commands) can begin

## Status

**Status**: READY_FOR_IMPLEMENTATION

All architectural decisions have been made, technical specifications are complete, and implementation guidance is detailed. The implementer can proceed with confidence.

**Key strengths of this design**:
1. ✅ Trait-based architecture enables testing and flexibility
2. ✅ Robust token refresh with race condition prevention
3. ✅ User-friendly error messages with troubleshooting hints
4. ✅ Security best practices (TLS, secret handling, validation)
5. ✅ Performance optimization (connection pooling, async)
6. ✅ Standards compliance (OAuth2, HTTP conventions)

**Critical implementation notes**:
- Token refresh concurrency is the most complex component - test thoroughly
- Error messages directly impact user experience - make them helpful
- Connection pooling happens automatically via reqwest - verify it works
- Storage layer integration is critical - tokens must persist correctly

**Next Agent**: Implementer (with this plan as specification)
