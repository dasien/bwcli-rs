---
enhancement: 03-api-client
agent: documenter
task_id: task_1764797277_69503
timestamp: 2025-12-03T17:30:00-08:00
status: DOCUMENTATION_COMPLETE
---

# API Client Documentation Summary

## Executive Summary

This document provides comprehensive documentation for the Bitwarden CLI Rust API client implementation. The API client is a production-ready HTTP communication layer that handles all interactions with Bitwarden servers, including authentication, token management, error handling, and proxy support.

**Target Audience:** Developers implementing commands and features that interact with the Bitwarden API.

**Status:** ✅ Complete and ready for use by downstream enhancements (auth commands, vault operations).

## Table of Contents

1. [Overview](#overview)
2. [Getting Started](#getting-started)
3. [Core Concepts](#core-concepts)
4. [API Reference](#api-reference)
5. [Usage Examples](#usage-examples)
6. [Error Handling](#error-handling)
7. [Configuration](#configuration)
8. [Security](#security)
9. [Testing](#testing)
10. [Troubleshooting](#troubleshooting)

---

## Overview

### What is the API Client?

The API client is a Rust library that provides HTTP communication capabilities for the Bitwarden CLI. It abstracts away the complexity of:

- Making HTTP requests to Bitwarden servers
- Managing authentication tokens
- Handling token refresh automatically
- Converting HTTP errors to user-friendly messages
- Supporting proxy configurations
- Ensuring secure TLS connections

### Key Features

✅ **Automatic Token Management** - Tokens are refreshed automatically when they expire
✅ **Connection Pooling** - Efficient connection reuse for better performance
✅ **Type-Safe API** - Generic methods ensure compile-time type safety
✅ **User-Friendly Errors** - Clear error messages with troubleshooting hints
✅ **Proxy Support** - Automatic detection from environment variables
✅ **Secure by Default** - TLS certificate validation, secret protection
✅ **Async Throughout** - Non-blocking I/O with tokio runtime

### Architecture

```
┌─────────────────────────────────────────────┐
│          CLI Commands                       │
│   (login, sync, list, create, etc.)        │
└──────────────────┬──────────────────────────┘
                   │
                   ▼
┌─────────────────────────────────────────────┐
│        ServiceContainer                     │
│   - Provides API client instance           │
│   - Provides storage access                │
└──────────────────┬──────────────────────────┘
                   │
                   ▼
┌─────────────────────────────────────────────┐
│        ApiClient Trait                      │
│   - Abstract interface                     │
│   - Defines HTTP methods                   │
└──────────────────┬──────────────────────────┘
                   │
                   ▼
┌─────────────────────────────────────────────┐
│    BitwardenApiClient                       │
│   - Concrete implementation                │
│   - Token refresh logic                    │
│   - Error handling                         │
└──────────────────┬──────────────────────────┘
                   │
         ┌─────────┴──────────┐
         ▼                    ▼
┌──────────────────┐  ┌──────────────────┐
│  HTTP Transport  │  │ Token Manager    │
│  (reqwest)       │  │ (storage layer)  │
└──────────────────┘  └──────────────────┘
```

---

## Getting Started

### Basic Setup

The API client is accessed through the `ServiceContainer`, which manages all service dependencies.

```rust
use bw_core::services::ServiceContainer;

// Create service container with defaults
let container = ServiceContainer::new(
    None,           // Use default API URL (vault.bitwarden.com)
    None,           // Use default identity URL
    None,           // Use default storage location
    None,           // Use default timeout (60 seconds)
)?;

// Get API client instance
let api_client = container.api_client();
```

### Making Your First Request

```rust
use serde::Deserialize;

#[derive(Deserialize)]
struct VersionResponse {
    version: String,
}

// Make an unauthenticated GET request
let version: VersionResponse = api_client
    .get("/public/version")
    .await?;

println!("Server version: {}", version.version);
```

### Authenticated Requests

```rust
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct SyncResponse {
    // ... sync data fields
}

// Make an authenticated GET request
// Token is automatically included and refreshed if needed
let sync_data: SyncResponse = api_client
    .get_with_auth("/sync")
    .await?;
```

---

## Core Concepts

### 1. Environment URLs

The `Environment` struct manages server URLs for all Bitwarden services.

**Default Cloud Environment:**
```rust
use bw_core::services::api::Environment;

// Use official Bitwarden cloud
let env = Environment::default_cloud();

println!("API URL: {}", env.api_url());
// Output: https://vault.bitwarden.com/api

println!("Identity URL: {}", env.identity_url());
// Output: https://vault.bitwarden.com/identity
```

**Self-Hosted Server:**
```rust
// Use custom self-hosted server
let env = Environment::from_base_url("https://my.server.com")?;

println!("API URL: {}", env.api_url());
// Output: https://my.server.com/api
```

**Available Service URLs:**
- `api_url()` - Main API endpoint
- `identity_url()` - Authentication endpoint
- `web_vault_url()` - Web vault URL
- `icons_url()` - Favicon service
- `notifications_url()` - WebSocket notifications
- `events_url()` - Event logging

### 2. Token Management

Token management is handled automatically by the `TokenManager` component.

**How Token Refresh Works:**

1. Client makes authenticated request with stored access token
2. Server returns 401 Unauthorized (token expired)
3. Client automatically uses refresh token to get new access token
4. Client saves new tokens to storage
5. Client retries original request with new access token
6. Request succeeds transparently to caller

**Concurrency Safety:**

Multiple concurrent requests with expired tokens will coordinate to ensure only one refresh operation occurs:

```rust
// These 100 concurrent requests will trigger only ONE token refresh
let handles: Vec<_> = (0..100)
    .map(|_| {
        let client = api_client.clone();
        tokio::spawn(async move {
            client.get_with_auth("/sync").await
        })
    })
    .collect();

// All requests will wait for the refresh and succeed with new token
for handle in handles {
    let result = handle.await??;
    // ... process result
}
```

### 3. Error Handling

The API client uses structured error types with helpful troubleshooting information.

**Error Categories:**

| Error Type | HTTP Status | Description |
|------------|-------------|-------------|
| `Network` | N/A | Connection failures, DNS errors |
| `Authentication` | 401, 403 | Missing or invalid credentials |
| `NotFound` | 404 | Resource doesn't exist |
| `RateLimit` | 429 | Too many requests |
| `Client` | Other 4xx | Client-side errors |
| `Server` | 5xx | Server-side errors |
| `Timeout` | N/A | Request took too long |
| `Tls` | N/A | Certificate validation failed |
| `Serialization` | N/A | JSON parsing error |
| `Configuration` | N/A | Invalid setup |

**Error Messages Include:**
- Clear problem description
- Troubleshooting hints
- Suggested actions
- Context (status codes, URLs)

---

## API Reference

### ApiClient Trait

The main interface for making HTTP requests.

#### Methods

##### `get<T>(&self, path: &str) -> Result<T>`

Make an unauthenticated GET request.

**Type Parameters:**
- `T` - Response type (must implement `Deserialize`)

**Arguments:**
- `path` - API path relative to base URL (e.g., "/public/version")

**Returns:**
- Deserialized response of type `T`

**Errors:**
- `ApiError::Network` - Connection failures
- `ApiError::NotFound` - Resource not found (404)
- `ApiError::Server` - Server errors (5xx)

**Example:**
```rust
#[derive(Deserialize)]
struct VersionResponse {
    version: String,
}

let version: VersionResponse = api_client.get("/public/version").await?;
```

---

##### `get_with_auth<T>(&self, path: &str) -> Result<T>`

Make an authenticated GET request with automatic token refresh.

**Type Parameters:**
- `T` - Response type (must implement `Deserialize`)

**Arguments:**
- `path` - API path relative to base URL

**Returns:**
- Deserialized response of type `T`

**Errors:**
- `ApiError::Authentication` - Not logged in or token refresh failed
- `ApiError::Network` - Connection failures
- `ApiError::RateLimit` - Too many requests
- `ApiError::Server` - Server errors

**Behavior:**
- Automatically includes Bearer token in Authorization header
- If 401 response received, attempts token refresh
- Retries request once with new token
- Returns error if refresh fails

**Example:**
```rust
#[derive(Deserialize)]
struct SyncResponse {
    ciphers: Vec<Cipher>,
    folders: Vec<Folder>,
}

let sync: SyncResponse = api_client.get_with_auth("/sync").await?;
```

---

##### `post<T, R>(&self, path: &str, body: &T) -> Result<R>`

Make an unauthenticated POST request.

**Type Parameters:**
- `T` - Request body type (must implement `Serialize`)
- `R` - Response type (must implement `Deserialize`)

**Arguments:**
- `path` - API path relative to base URL
- `body` - Request body to serialize as JSON

**Returns:**
- Deserialized response of type `R`

**Example:**
```rust
#[derive(Serialize)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: String,
}

let request = LoginRequest {
    username: "user@example.com".to_string(),
    password: "password".to_string(),
};

let response: TokenResponse = api_client
    .post("/identity/connect/token", &request)
    .await?;
```

---

##### `post_with_auth<T, R>(&self, path: &str, body: &T) -> Result<R>`

Make an authenticated POST request with automatic token refresh.

**Type Parameters:**
- `T` - Request body type (must implement `Serialize`)
- `R` - Response type (must implement `Deserialize`)

**Arguments:**
- `path` - API path relative to base URL
- `body` - Request body to serialize as JSON

**Returns:**
- Deserialized response of type `R`

**Example:**
```rust
#[derive(Serialize)]
struct CreateCipherRequest {
    name: String,
    login: LoginData,
}

#[derive(Deserialize)]
struct CipherResponse {
    id: String,
    name: String,
}

let request = CreateCipherRequest {
    name: "My Website".to_string(),
    login: LoginData { /* ... */ },
};

let cipher: CipherResponse = api_client
    .post_with_auth("/ciphers", &request)
    .await?;
```

---

##### `put_with_auth<T, R>(&self, path: &str, body: &T) -> Result<R>`

Make an authenticated PUT request to update a resource.

**Type Parameters:**
- `T` - Request body type (must implement `Serialize`)
- `R` - Response type (must implement `Deserialize`)

**Arguments:**
- `path` - API path including resource ID (e.g., "/ciphers/abc-123")
- `body` - Updated resource data

**Returns:**
- Deserialized response of type `R`

**Example:**
```rust
let updated_cipher = UpdateCipherRequest {
    name: "Updated Name".to_string(),
    // ... other fields
};

let result: CipherResponse = api_client
    .put_with_auth("/ciphers/abc-123", &updated_cipher)
    .await?;
```

---

##### `delete_with_auth(&self, path: &str) -> Result<()>`

Make an authenticated DELETE request to remove a resource.

**Arguments:**
- `path` - API path including resource ID

**Returns:**
- Empty `Result<()>` on success (204 No Content)

**Example:**
```rust
// Delete a cipher
api_client.delete_with_auth("/ciphers/abc-123").await?;
println!("Cipher deleted successfully");
```

---

##### `environment(&self) -> &Environment`

Get the current environment configuration.

**Returns:**
- Reference to `Environment` with all service URLs

**Example:**
```rust
let env = api_client.environment();
println!("Connecting to: {}", env.api_url());
```

---

##### `is_authenticated(&self) -> bool`

Check if user is currently authenticated.

**Returns:**
- `true` if access token exists in storage
- `false` if not logged in

**Note:** This only checks local storage, does not validate token with server.

**Example:**
```rust
if api_client.is_authenticated().await {
    println!("User is logged in");
} else {
    println!("Please run 'bw login'");
}
```

---

### Environment

#### Constructors

##### `Environment::default_cloud() -> Self`

Create environment for official Bitwarden cloud.

**Returns:**
- Environment configured for `https://vault.bitwarden.com`

**Example:**
```rust
let env = Environment::default_cloud();
```

---

##### `Environment::from_base_url(base_url: &str) -> Result<Self>`

Create environment from a base URL.

**Arguments:**
- `base_url` - Base URL for all services (e.g., "https://my.server.com")

**Returns:**
- Environment with all service URLs derived from base

**Validation:**
- URL must be well-formed
- Must use HTTPS (or HTTP for localhost only)
- Trailing slashes are removed

**Example:**
```rust
let env = Environment::from_base_url("https://vault.example.com")?;
```

---

##### `Environment::custom(...) -> Result<Self>`

Create environment with custom service URLs.

**Arguments:**
- `base_url` - Base URL
- `api_url` - Optional custom API URL
- `identity_url` - Optional custom identity URL
- `web_vault_url` - Optional custom web vault URL
- `icons_url` - Optional custom icons URL
- `notifications_url` - Optional custom notifications URL
- `events_url` - Optional custom events URL

**Returns:**
- Environment with specified URLs

**Example:**
```rust
let env = Environment::custom(
    "https://vault.example.com",
    Some("https://api.example.com".to_string()),
    Some("https://identity.example.com".to_string()),
    None, // use default web vault URL
    None, // use default icons URL
    None, // use default notifications URL
    None, // use default events URL
)?;
```

---

### ApiError

#### Variants

##### `ApiError::Network`

Network connectivity error (DNS failure, connection refused, etc.)

**Fields:**
- `message: String` - Problem description
- `troubleshooting: String` - Troubleshooting steps
- `source: Option<reqwest::Error>` - Underlying error

**Common Causes:**
- No internet connection
- DNS resolution failed
- Firewall blocking connection
- Proxy misconfiguration

**Example:**
```rust
match result {
    Err(e) if matches!(e.downcast_ref::<ApiError>(), Some(ApiError::Network { .. })) => {
        eprintln!("Network error: {}", e);
        // Check connection and try again
    }
    _ => {}
}
```

---

##### `ApiError::Authentication`

Authentication or authorization error.

**Fields:**
- `message: String` - Problem description
- `hint: String` - Suggested action

**Common Causes:**
- Not logged in
- Token expired and refresh failed
- Access forbidden (403)
- Invalid credentials

**Example:**
```rust
match result {
    Err(e) if matches!(e.downcast_ref::<ApiError>(), Some(ApiError::Authentication { .. })) => {
        eprintln!("Authentication error: {}", e);
        // User should run 'bw login'
    }
    _ => {}
}
```

---

##### `ApiError::RateLimit`

Too many requests, rate limit exceeded.

**Fields:**
- `message: String` - Wait instructions
- `retry_after: Option<u64>` - Seconds to wait before retry

**Example:**
```rust
match result {
    Err(e) => {
        if let Some(ApiError::RateLimit { retry_after, .. }) = e.downcast_ref::<ApiError>() {
            if let Some(seconds) = retry_after {
                println!("Rate limited. Retry after {} seconds.", seconds);
            }
        }
    }
    _ => {}
}
```

---

## Usage Examples

### Example 1: Login and Authenticate

```rust
use bw_core::services::ServiceContainer;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct LoginRequest {
    grant_type: String,
    username: String,
    password: String,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: String,
}

async fn login(username: &str, password: &str) -> anyhow::Result<()> {
    let container = ServiceContainer::new(None, None, None, None)?;
    let api_client = container.api_client();

    let request = LoginRequest {
        grant_type: "password".to_string(),
        username: username.to_string(),
        password: password.to_string(),
    };

    // Make login request
    let response: TokenResponse = api_client
        .post("/identity/connect/token", &request)
        .await?;

    // Tokens are automatically saved to storage by token manager
    println!("Login successful!");

    Ok(())
}
```

---

### Example 2: Sync Vault Data

```rust
use bw_core::services::ServiceContainer;
use serde::Deserialize;

#[derive(Deserialize)]
struct SyncResponse {
    ciphers: Vec<Cipher>,
    folders: Vec<Folder>,
    collections: Vec<Collection>,
}

async fn sync_vault() -> anyhow::Result<SyncResponse> {
    let container = ServiceContainer::new(None, None, None, None)?;
    let api_client = container.api_client();

    // Check if authenticated
    if !api_client.is_authenticated().await {
        anyhow::bail!("Not logged in. Please run 'bw login'.");
    }

    // Fetch sync data
    // Token is automatically included and refreshed if needed
    let sync_data: SyncResponse = api_client
        .get_with_auth("/sync?excludeDomains=true")
        .await?;

    println!("Synced {} ciphers", sync_data.ciphers.len());

    Ok(sync_data)
}
```

---

### Example 3: Create a New Cipher

```rust
use bw_core::services::ServiceContainer;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct CreateCipherRequest {
    #[serde(rename = "type")]
    cipher_type: u8,  // 1 = Login
    name: String,
    notes: Option<String>,
    login: LoginData,
}

#[derive(Serialize)]
struct LoginData {
    username: String,
    password: String,
    uris: Vec<LoginUri>,
}

#[derive(Serialize)]
struct LoginUri {
    uri: String,
}

#[derive(Deserialize)]
struct CipherResponse {
    id: String,
    name: String,
}

async fn create_login_cipher() -> anyhow::Result<String> {
    let container = ServiceContainer::new(None, None, None, None)?;
    let api_client = container.api_client();

    let request = CreateCipherRequest {
        cipher_type: 1,
        name: "My Website".to_string(),
        notes: Some("Created from Rust CLI".to_string()),
        login: LoginData {
            username: "user@example.com".to_string(),
            password: "secure-password".to_string(),
            uris: vec![
                LoginUri {
                    uri: "https://example.com".to_string(),
                }
            ],
        },
    };

    let response: CipherResponse = api_client
        .post_with_auth("/ciphers", &request)
        .await?;

    println!("Created cipher: {} ({})", response.name, response.id);

    Ok(response.id)
}
```

---

### Example 4: Error Handling

```rust
use bw_core::services::api::ApiError;
use bw_core::services::ServiceContainer;

async fn fetch_data_with_error_handling() -> anyhow::Result<()> {
    let container = ServiceContainer::new(None, None, None, None)?;
    let api_client = container.api_client();

    match api_client.get_with_auth::<SyncResponse>("/sync").await {
        Ok(data) => {
            println!("Sync successful");
            Ok(())
        }
        Err(e) => {
            // Check for specific error types
            if let Some(api_error) = e.downcast_ref::<ApiError>() {
                match api_error {
                    ApiError::Authentication { message, hint } => {
                        eprintln!("Authentication failed: {}", message);
                        eprintln!("Hint: {}", hint);
                    }
                    ApiError::Network { message, troubleshooting, .. } => {
                        eprintln!("Network error: {}", message);
                        eprintln!("Troubleshooting: {}", troubleshooting);
                    }
                    ApiError::RateLimit { message, retry_after } => {
                        eprintln!("Rate limited: {}", message);
                        if let Some(seconds) = retry_after {
                            eprintln!("Retry after {} seconds", seconds);
                        }
                    }
                    ApiError::Server { status, message, hint } => {
                        eprintln!("Server error ({}): {}", status, message);
                        eprintln!("Hint: {}", hint);
                    }
                    _ => {
                        eprintln!("API error: {}", e);
                    }
                }
            }
            Err(e)
        }
    }
}
```

---

### Example 5: Custom Timeout

```rust
use bw_core::services::ServiceContainer;

async fn api_with_custom_timeout() -> anyhow::Result<()> {
    // Create container with 120 second timeout
    let container = ServiceContainer::new(
        None,           // default API URL
        None,           // default identity URL
        None,           // default storage path
        Some(120),      // 120 second timeout
    )?;

    let api_client = container.api_client();

    // Long-running request with extended timeout
    let result: SyncResponse = api_client
        .get_with_auth("/sync")
        .await?;

    Ok(())
}
```

---

### Example 6: Self-Hosted Server

```rust
use bw_core::services::ServiceContainer;
use bw_core::services::api::Environment;

async fn use_self_hosted_server() -> anyhow::Result<()> {
    // Create environment for self-hosted server
    let env = Environment::from_base_url("https://vault.mycompany.com")?;

    println!("API URL: {}", env.api_url());
    // Output: https://vault.mycompany.com/api

    println!("Identity URL: {}", env.identity_url());
    // Output: https://vault.mycompany.com/identity

    // Create container with custom server URL
    let container = ServiceContainer::new(
        Some("https://vault.mycompany.com".to_string()),
        None,
        None,
        None,
    )?;

    let api_client = container.api_client();

    // All requests now go to self-hosted server
    let version: VersionResponse = api_client
        .get("/public/version")
        .await?;

    Ok(())
}
```

---

## Error Handling

### Error Philosophy

The API client provides clear, actionable error messages that help users understand what went wrong and how to fix it.

**Design Principles:**

1. **Clarity** - Error messages explain what happened in plain language
2. **Context** - Include relevant details (status codes, URLs, etc.)
3. **Actionable** - Provide specific troubleshooting steps
4. **User-Focused** - Written for CLI users, not developers

### Common Error Scenarios

#### Scenario 1: Not Logged In

**Error:**
```
ApiError::Authentication {
    message: "Not authenticated",
    hint: "Run 'bw login' to authenticate again"
}
```

**Cause:** User hasn't logged in yet or tokens have been cleared.

**Solution:**
```bash
bw login
```

---

#### Scenario 2: Network Connection Failed

**Error:**
```
ApiError::Network {
    message: "Failed to connect to server",
    troubleshooting: "Check server URL, DNS settings, and firewall configuration"
}
```

**Common Causes:**
- No internet connection
- Incorrect server URL
- Firewall blocking connection
- DNS resolution failure

**Solutions:**
1. Check internet connection
2. Verify server URL: `bw config server`
3. Check firewall rules
4. Verify DNS is working

---

#### Scenario 3: Token Expired and Refresh Failed

**Error:**
```
ApiError::Authentication {
    message: "Access token expired",
    hint: "Run 'bw login' to authenticate again"
}
```

**Cause:** Access token expired and refresh token is also invalid/expired.

**Solution:** Re-authenticate:
```bash
bw login
```

---

#### Scenario 4: Rate Limited

**Error:**
```
ApiError::RateLimit {
    message: "Please wait 60 seconds before retrying.",
    retry_after: Some(60)
}
```

**Cause:** Too many requests sent to server in short time.

**Solution:** Wait the specified time before retrying.

---

#### Scenario 5: Server Error

**Error:**
```
ApiError::Server {
    status: 503,
    message: "Service unavailable",
    hint: "Server temporarily unavailable. Please try again in a few moments."
}
```

**Cause:** Server is down or experiencing issues.

**Solutions:**
1. Wait a few minutes and retry
2. Check Bitwarden status page
3. If self-hosted, check server logs

---

#### Scenario 6: Request Timeout

**Error:**
```
ApiError::Timeout {
    message: "Request timed out",
    hint: "Check your network connection or increase timeout"
}
```

**Cause:** Network too slow or request taking too long.

**Solutions:**
1. Check network connection
2. Increase timeout:
   ```rust
   ServiceContainer::new(None, None, None, Some(120))? // 120 seconds
   ```

---

### Error Handling Best Practices

#### 1. Always Handle Authentication Errors

```rust
match api_client.get_with_auth::<SyncResponse>("/sync").await {
    Ok(data) => {
        // Process data
    }
    Err(e) => {
        if let Some(ApiError::Authentication { .. }) = e.downcast_ref::<ApiError>() {
            eprintln!("Not logged in. Please run 'bw login'.");
            std::process::exit(1);
        }
        return Err(e);
    }
}
```

---

#### 2. Provide Context in User Messages

```rust
// Good: Include what failed
eprintln!("Failed to sync vault: {}", error);

// Bad: Generic message
eprintln!("An error occurred");
```

---

#### 3. Log Errors for Debugging

```rust
use tracing::{error, warn};

match api_client.get_with_auth::<SyncResponse>("/sync").await {
    Ok(data) => Ok(data),
    Err(e) => {
        // Log full error for debugging
        error!("Sync failed: {:?}", e);

        // Show user-friendly message
        eprintln!("Failed to sync vault. Check your connection and try again.");

        Err(e)
    }
}
```

---

## Configuration

### Server Configuration

#### Set Custom Server URL

```rust
// Via ServiceContainer
let container = ServiceContainer::new(
    Some("https://vault.example.com".to_string()),
    None,
    None,
    None,
)?;
```

Or via CLI:
```bash
bw config server https://vault.example.com
```

---

### Timeout Configuration

#### Default Timeout

Default: 60 seconds for requests, 30 seconds for connection.

#### Custom Timeout

```rust
// Set 120 second timeout
let container = ServiceContainer::new(
    None,
    None,
    None,
    Some(120), // timeout in seconds
)?;
```

---

### Proxy Configuration

The API client automatically detects proxy settings from environment variables.

#### HTTP Proxy

```bash
export HTTP_PROXY=http://proxy.example.com:8080
export HTTPS_PROXY=http://proxy.example.com:8080
```

#### Authenticated Proxy

```bash
export HTTP_PROXY=http://username:password@proxy.example.com:8080
```

#### Bypass Proxy for Specific Hosts

```bash
export NO_PROXY=localhost,127.0.0.1,.internal.domain
```

#### How It Works

The `reqwest` HTTP client automatically reads these environment variables and configures the proxy. No code changes required.

---

## Security

### Token Security

#### In-Memory Protection

All tokens are wrapped in `secrecy::Secret<String>` to prevent accidental exposure:

- Tokens are not logged or printed
- Debug output shows `[REDACTED]` instead of token value
- Tokens are automatically zeroized when dropped

#### Storage Encryption

Tokens are stored encrypted via the storage layer:

```
{
  "__PROTECTED__accessToken": "<encrypted-data>",
  "__PROTECTED__refreshToken": "<encrypted-data>"
}
```

#### Token Lifecycle

1. **Login** - Tokens obtained from identity server
2. **Storage** - Tokens saved encrypted to disk
3. **Usage** - Tokens loaded into memory as Secret<String>
4. **Refresh** - Expired tokens automatically refreshed
5. **Logout** - Tokens securely cleared from storage

---

### TLS/Certificate Validation

#### Always Enabled

Certificate validation is always enabled and cannot be disabled in the MVP.

**Features:**
- Uses `rustls-tls` for modern, secure TLS
- Validates certificate chain
- Checks certificate expiration
- Verifies hostname matches

#### HTTPS Required

HTTPS is required for all remote servers:

```rust
// This works
let env = Environment::from_base_url("https://vault.example.com")?;

// This fails (not localhost)
let env = Environment::from_base_url("http://remote.server.com")?;
// Error: HTTPS required for remote servers

// This works (localhost exception)
let env = Environment::from_base_url("http://localhost:8080")?;
```

---

### URL Validation

All URLs are validated before use:

- Must be well-formed
- Must use HTTPS (except localhost)
- Trailing slashes removed
- No open redirects (max 10 redirects)

---

### Request/Response Limits

Protection against DoS and memory exhaustion:

- **Request timeout:** 60 seconds default
- **Connect timeout:** 30 seconds
- **Body size:** 2MB default (reqwest limit)
- **Redirects:** Maximum 10

---

## Testing

### Unit Tests

The API client includes comprehensive unit tests for core components.

**Environment Tests:**
```bash
cargo test --package bw-core --lib services::api::environment
```

**Test Coverage:**
- Default cloud environment
- Custom base URL
- HTTPS validation
- Localhost HTTP allowance
- Trailing slash removal

---

### Integration Tests

Integration tests use `wiremock` for HTTP mocking.

**Example Test:**
```rust
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path};

#[tokio::test]
async fn test_get_request() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/version"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(json!({"version": "1.0.0"})))
        .mount(&mock_server)
        .await;

    let env = Environment::from_base_url(&mock_server.uri())?;
    let client = BitwardenApiClient::new(env, storage, None)?;

    let response: VersionResponse = client.get("/version").await?;
    assert_eq!(response.version, "1.0.0");
}
```

---

### Testing Your Code

When implementing commands that use the API client, test with:

1. **Unit tests** - Mock the ApiClient trait
2. **Integration tests** - Use wiremock for HTTP mocking
3. **Manual testing** - Test with real Bitwarden server

**Example: Mock ApiClient**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    struct MockApiClient;

    #[async_trait]
    impl ApiClient for MockApiClient {
        async fn get_with_auth<T>(&self, path: &str) -> Result<T>
        where
            T: for<'de> Deserialize<'de>,
        {
            // Return mock data
            Ok(/* ... */)
        }

        // ... implement other methods
    }

    #[tokio::test]
    async fn test_my_command() {
        let mock_client = MockApiClient;
        let result = my_command(&mock_client).await?;
        assert!(result.is_success());
    }
}
```

---

## Troubleshooting

### Problem: "Not authenticated" error

**Symptoms:**
```
ApiError::Authentication {
    message: "Not authenticated",
    hint: "Run 'bw login' to authenticate again"
}
```

**Solutions:**

1. Check if logged in:
   ```rust
   if !api_client.is_authenticated().await {
       println!("Not logged in");
   }
   ```

2. Log in:
   ```bash
   bw login
   ```

3. Check token storage:
   ```bash
   # Tokens should be in data.json
   cat ~/.config/Bitwarden/data.json
   ```

---

### Problem: "Failed to connect to server"

**Symptoms:**
```
ApiError::Network {
    message: "Failed to connect to server",
    troubleshooting: "Check server URL, DNS settings, and firewall configuration"
}
```

**Solutions:**

1. Check internet connection:
   ```bash
   ping vault.bitwarden.com
   ```

2. Verify server URL:
   ```bash
   bw config server
   ```

3. Check DNS:
   ```bash
   nslookup vault.bitwarden.com
   ```

4. Test with curl:
   ```bash
   curl https://vault.bitwarden.com/api/config
   ```

5. Check firewall rules

---

### Problem: Request timeout

**Symptoms:**
```
ApiError::Timeout {
    message: "Request timed out",
    hint: "Check your network connection or increase timeout"
}
```

**Solutions:**

1. Check network speed

2. Increase timeout:
   ```rust
   let container = ServiceContainer::new(
       None,
       None,
       None,
       Some(120), // 120 seconds
   )?;
   ```

3. Check server status

---

### Problem: Certificate validation failed

**Symptoms:**
```
ApiError::Tls {
    message: "Certificate validation failed",
    hint: "..."
}
```

**Causes:**
- Self-signed certificate
- Expired certificate
- Certificate name mismatch

**Solutions:**

For self-hosted servers:
1. Use valid certificate from Let's Encrypt
2. Ensure certificate matches server hostname
3. Check certificate expiration

---

### Problem: Rate limited

**Symptoms:**
```
ApiError::RateLimit {
    message: "Please wait 60 seconds before retrying.",
    retry_after: Some(60)
}
```

**Solutions:**

1. Wait the specified time
2. Reduce request frequency
3. Implement exponential backoff:
   ```rust
   let mut retry_delay = 1;
   loop {
       match api_client.get_with_auth::<SyncResponse>("/sync").await {
           Ok(data) => break Ok(data),
           Err(e) if is_rate_limit(&e) => {
               tokio::time::sleep(Duration::from_secs(retry_delay)).await;
               retry_delay *= 2;
           }
           Err(e) => break Err(e),
       }
   }
   ```

---

### Problem: Proxy not working

**Symptoms:**
- Connection fails when proxy should be used
- Requests don't go through proxy

**Solutions:**

1. Verify environment variables:
   ```bash
   echo $HTTP_PROXY
   echo $HTTPS_PROXY
   ```

2. Check proxy authentication:
   ```bash
   export HTTP_PROXY=http://user:pass@proxy:8080
   ```

3. Test proxy with curl:
   ```bash
   curl -x http://proxy:8080 https://vault.bitwarden.com/api/config
   ```

4. Check NO_PROXY doesn't block target:
   ```bash
   echo $NO_PROXY
   ```

---

### Debugging Tips

#### Enable Debug Logging

```bash
RUST_LOG=debug cargo run
```

Or in code:
```rust
tracing_subscriber::fmt()
    .with_max_level(tracing::Level::DEBUG)
    .init();
```

#### Inspect HTTP Traffic

Use a proxy like `mitmproxy` to inspect HTTP requests:

```bash
# Start mitmproxy
mitmproxy -p 8888

# Set proxy
export HTTP_PROXY=http://localhost:8888
export HTTPS_PROXY=http://localhost:8888

# Run CLI
cargo run
```

#### Check Token Storage

```bash
# View encrypted tokens in storage
cat ~/.config/Bitwarden/data.json | jq
```

---

## Performance Considerations

### Connection Pooling

The API client reuses HTTP connections for better performance:

- Single `reqwest::Client` instance shared across all requests
- TCP connections reused when possible
- Reduces TLS handshake overhead

**Expected Performance:**
- First request to server: ~500ms (includes TLS handshake)
- Subsequent requests: ~200-300ms (connection reused)

---

### Concurrent Requests

The API client supports concurrent requests efficiently:

```rust
use futures::future::join_all;

// Make 10 requests concurrently
let futures: Vec<_> = (0..10)
    .map(|i| {
        let client = api_client.clone();
        async move {
            client.get_with_auth::<SyncResponse>("/sync").await
        }
    })
    .collect();

let results = join_all(futures).await;
```

**Performance:**
- 10 concurrent requests: ~2-3 seconds total (vs ~20-30s sequential)
- Limited by server rate limits and network bandwidth

---

### Token Refresh Optimization

Token refresh is optimized to avoid duplicate refreshes:

- Multiple concurrent requests with expired token trigger only ONE refresh
- All requests wait for refresh to complete
- All requests retry with new token

**Performance Impact:**
- First request after expiration: +2s (refresh time)
- Concurrent requests: No additional overhead

---

## Future Enhancements

The following features are planned for future releases:

### Request/Response Logging

Debug mode logging for troubleshooting:
```bash
RUST_LOG=bw_core::services::api=debug cargo run
```

Will show:
- Request method, URL, headers
- Response status, headers
- Timing information
- No request/response bodies (security)

---

### Retry Logic

Automatic retry with exponential backoff:
- Retry on network failures
- Retry on server errors (500, 502, 503)
- Don't retry on client errors (4xx)
- Exponential backoff: 1s, 2s, 4s, 8s

---

### Circuit Breaker

Prevent cascading failures:
- Open circuit after N failures
- Half-open after timeout
- Close circuit after success
- Fast-fail when open

---

### Custom Certificate Support

For self-hosted servers with self-signed certificates:
```bash
bw config cert-file ./my-cert.pem
```

---

### Request Middleware

Plugin pattern for request/response processing:
- Custom headers
- Request signing
- Response logging
- Metrics collection

---

## Additional Resources

### Related Documentation

- [Storage Layer Documentation](../../02-storage-layer/documenter/required_output/documentation_summary.md)
- [Architecture Plan](../architect/required_output/implementation_plan.md)
- [Implementation Summary](../implementer/required_output/implementation_summary.md)
- [Test Summary](../tester/required_output/test_summary.md)

### External References

- [reqwest Documentation](https://docs.rs/reqwest/)
- [tokio Documentation](https://docs.rs/tokio/)
- [Bitwarden API Documentation](https://bitwarden.com/help/api/)
- [OAuth2 RFC](https://tools.ietf.org/html/rfc6749)

### Code Locations

All API client code is in the `bw-core` crate:

```
crates/bw-core/src/
├── services/
│   └── api/
│       ├── mod.rs              # Public exports
│       ├── traits.rs           # ApiClient trait
│       ├── client.rs           # BitwardenApiClient implementation
│       ├── environment.rs      # Environment URLs
│       ├── token_manager.rs    # Token refresh logic
│       └── errors.rs           # Error types
└── models/
    └── api/
        ├── token.rs            # Token request/response
        └── error_response.rs   # API error format
```

---

## Contact and Support

### For Implementation Questions

Refer to the architecture and implementation documents in this enhancement directory.

### For Bug Reports

Create an issue with:
1. Error message
2. Steps to reproduce
3. Expected vs actual behavior
4. Debug logs (with sensitive data removed)

### For Feature Requests

Enhancement requests should include:
1. Use case description
2. Expected behavior
3. Alternatives considered
4. Implementation complexity estimate

---

## Appendix: Complete Type Signatures

### ApiClient Trait

```rust
#[async_trait]
pub trait ApiClient: Send + Sync {
    async fn get<T>(&self, path: &str) -> Result<T>
    where
        T: for<'de> Deserialize<'de>;

    async fn get_with_auth<T>(&self, path: &str) -> Result<T>
    where
        T: for<'de> Deserialize<'de>;

    async fn post<T, R>(&self, path: &str, body: &T) -> Result<R>
    where
        T: Serialize + Send + Sync,
        R: for<'de> Deserialize<'de>;

    async fn post_with_auth<T, R>(&self, path: &str, body: &T) -> Result<R>
    where
        T: Serialize + Send + Sync,
        R: for<'de> Deserialize<'de>;

    async fn put_with_auth<T, R>(&self, path: &str, body: &T) -> Result<R>
    where
        T: Serialize + Send + Sync,
        R: for<'de> Deserialize<'de>;

    async fn delete_with_auth(&self, path: &str) -> Result<()>;

    fn environment(&self) -> &Environment;

    async fn is_authenticated(&self) -> bool;
}
```

### Environment

```rust
pub struct Environment {
    // Private fields
}

impl Environment {
    pub fn default_cloud() -> Self;
    pub fn from_base_url(base_url: &str) -> Result<Self>;
    pub fn custom(
        base_url: &str,
        api_url: Option<String>,
        identity_url: Option<String>,
        web_vault_url: Option<String>,
        icons_url: Option<String>,
        notifications_url: Option<String>,
        events_url: Option<String>,
    ) -> Result<Self>;

    pub fn api_url(&self) -> &str;
    pub fn identity_url(&self) -> &str;
    pub fn web_vault_url(&self) -> &str;
    pub fn icons_url(&self) -> &str;
    pub fn notifications_url(&self) -> &str;
    pub fn events_url(&self) -> Option<&str>;
}
```

### ApiError

```rust
#[derive(Debug, Error)]
pub enum ApiError {
    Network {
        message: String,
        troubleshooting: String,
        #[source]
        source: Option<reqwest::Error>,
    },
    Authentication {
        message: String,
        hint: String,
    },
    NotFound {
        resource: String,
    },
    RateLimit {
        message: String,
        retry_after: Option<u64>,
    },
    Client {
        status: StatusCode,
        message: String,
    },
    Server {
        status: StatusCode,
        message: String,
        hint: String,
    },
    Timeout {
        message: String,
        hint: String,
    },
    Tls {
        message: String,
        hint: String,
    },
    Serialization(#[from] serde_json::Error),
    Configuration(String),
}

impl ApiError {
    pub fn network_error(err: reqwest::Error) -> Self;
    pub fn auth_error(message: String) -> Self;
    pub fn server_error(status: StatusCode, message: String) -> Self;
    pub fn rate_limit_error(retry_after: Option<u64>) -> Self;
}
```

---

## Document Metadata

**Enhancement:** 03-api-client
**Agent:** documenter
**Status:** DOCUMENTATION_COMPLETE
**Created:** 2025-12-03
**Version:** 1.0.0

**Related Documents:**
- [Requirements Analysis](../requirements-analyst/required_output/analysis_summary.md)
- [Architecture Plan](../architect/required_output/implementation_plan.md)
- [Implementation Summary](../implementer/required_output/implementation_summary.md)
- [Test Summary](../tester/required_output/test_summary.md)

---

*This documentation is generated for the Bitwarden CLI Rust migration project. For questions or updates, refer to the enhancement workflow documentation.*
