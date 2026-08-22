use super::errors::ApiError;
use crate::models::api::token::{TokenRefreshRequest, TokenResponse};
use crate::services::storage::{JsonFileStorage, Storage, StorageKey};
use anyhow::Result;
use bitwarden_core::auth::ClientManagedTokens;
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
    storage: Arc<Mutex<JsonFileStorage>>,

    /// Refresh coordination state
    /// - None: No refresh in progress
    /// - Some(Future): Refresh in progress, await this
    refresh_state: Arc<Mutex<Option<Arc<Mutex<()>>>>>,
}

impl TokenManager {
    pub fn new(storage: Arc<Mutex<JsonFileStorage>>) -> Self {
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
        self.get_user_token(StorageKey::UserAccessToken).await
    }

    /// Get current refresh token
    ///
    /// # Returns
    /// Secret-wrapped refresh token if available, None otherwise
    pub async fn get_refresh_token(&self) -> Result<Option<Secret<String>>> {
        self.get_user_token(StorageKey::UserRefreshToken).await
    }

    /// Get a user token by storage key type
    ///
    /// Shared implementation for access and refresh token retrieval
    async fn get_user_token(&self, key_type: StorageKey) -> Result<Option<Secret<String>>> {
        let storage = self.storage.lock().await;

        // Get active user ID using namespaced key
        let active_id_key = StorageKey::GlobalActiveAccountId.format(None);
        let active_id: Option<serde_json::Value> = storage.get(&active_id_key)?;

        let user_id = match active_id {
            Some(serde_json::Value::String(id)) if !id.is_empty() => id,
            _ => return Ok(None),
        };

        // Get token for this user using namespaced key
        let token_key = key_type.format(Some(&user_id));
        let token_str: Option<String> = storage.get(&token_key)?;
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
    pub async fn refresh_access_token<F, Fut>(&self, refresh_client: F) -> Result<Secret<String>>
    where
        F: FnOnce(TokenRefreshRequest) -> Fut,
        Fut: std::future::Future<Output = Result<TokenResponse>>,
    {
        // If another task is already refreshing, wait for it and re-read.
        //
        // Note the state lock is taken in a scope and released before anything
        // else touches it. It previously stayed held across a second
        // `refresh_state.lock()`, which deadlocked on the non-reentrant mutex —
        // meaning a token refresh could never actually complete.
        let in_progress = {
            let state = self.refresh_state.lock().await;
            state.as_ref().map(Arc::clone)
        };

        if let Some(existing) = in_progress {
            let _wait = existing.lock().await;

            return self
                .get_access_token()
                .await?
                .ok_or_else(|| ApiError::Authentication {
                    message: "Token refresh completed but access token not found".into(),
                    hint: "Run 'bw login' to authenticate again".to_string(),
                })
                .map_err(|e| anyhow::anyhow!(e));
        }

        // Claim the refresh for ourselves.
        let refresh_lock = Arc::new(Mutex::new(()));
        let _claim = refresh_lock.lock().await;
        {
            let mut state = self.refresh_state.lock().await;
            *state = Some(Arc::clone(&refresh_lock));
        }

        let result = self.perform_refresh(refresh_client).await;

        // Release the claim regardless of outcome, or every later refresh would
        // wait forever on a lock nobody holds.
        {
            let mut state = self.refresh_state.lock().await;
            *state = None;
        }

        result
    }

    /// The actual refresh round trip and token persistence.
    async fn perform_refresh<F, Fut>(&self, refresh_client: F) -> Result<Secret<String>>
    where
        F: FnOnce(TokenRefreshRequest) -> Fut,
        Fut: std::future::Future<Output = Result<TokenResponse>>,
    {
        let refresh_token =
            self.get_refresh_token()
                .await?
                .ok_or_else(|| ApiError::Authentication {
                    message: "No refresh token available. Please log in again.".into(),
                    hint: "Run 'bw login' to authenticate again".to_string(),
                })?;

        let request = TokenRefreshRequest {
            grant_type: "refresh_token".to_string(),
            refresh_token: refresh_token.expose_secret().clone(),
            client_id: "cli".to_string(),
        };

        let response = refresh_client(request)
            .await
            .map_err(|e| anyhow::anyhow!("Token refresh failed: {}", e))?;

        {
            let mut storage = self.storage.lock().await;

            let active_id_key = StorageKey::GlobalActiveAccountId.format(None);
            let active_id: Option<serde_json::Value> = storage.get(&active_id_key)?;

            let user_id = match active_id {
                Some(serde_json::Value::String(id)) if !id.is_empty() => id,
                _ => return Err(anyhow::anyhow!("No active user during token refresh")),
            };

            storage
                .set(
                    &StorageKey::UserAccessToken.format(Some(&user_id)),
                    &response.access_token,
                )
                .await?;

            if let Some(new_refresh_token) = &response.refresh_token {
                storage
                    .set(
                        &StorageKey::UserRefreshToken.format(Some(&user_id)),
                        new_refresh_token,
                    )
                    .await?;
            }

            // Without this the renewed tokens stay in memory and the next
            // invocation refreshes again from the old ones.
            storage.flush().await?;
        }

        Ok(Secret::new(response.access_token))
    }

    /// Save tokens after successful login
    ///
    /// # Arguments
    /// * `user_id` - User ID for namespaced key
    /// * `access_token` - New access token
    /// * `refresh_token` - New refresh token
    pub async fn save_tokens(
        &self,
        user_id: &str,
        access_token: &str,
        refresh_token: &str,
    ) -> Result<()> {
        let mut storage = self.storage.lock().await;
        let access_key = StorageKey::UserAccessToken.format(Some(user_id));
        let refresh_key = StorageKey::UserRefreshToken.format(Some(user_id));
        storage.set(&access_key, &access_token.to_string()).await?;
        storage
            .set(&refresh_key, &refresh_token.to_string())
            .await?;
        Ok(())
    }

    /// Clear stored tokens for a user
    ///
    /// Called on logout. Sets tokens to null (not removed) to match TypeScript CLI behavior.
    pub async fn clear_tokens(&self, user_id: &str) -> Result<()> {
        let mut storage = self.storage.lock().await;
        let access_key = StorageKey::UserAccessToken.format(Some(user_id));
        let refresh_key = StorageKey::UserRefreshToken.format(Some(user_id));
        // Set to null instead of removing (TypeScript CLI compatibility)
        storage.set(&access_key, &serde_json::Value::Null).await?;
        storage.set(&refresh_key, &serde_json::Value::Null).await?;
        Ok(())
    }
}

/// Bridges our stored access token to the SDK client.
///
/// The SDK needs an access token to make authenticated calls through its
/// generated API clients. Without this the CLI runs two disconnected HTTP
/// stacks: our reqwest client (authenticated from `data.json`) and the SDK's
/// (never authenticated), which is why SDK API calls could not be used.
///
/// `ClientManagedTokenHandler` attaches the bearer token but never renews it,
/// so this delegates to [`BitwardenApiClient::valid_access_token`], which
/// refreshes when the token is expired or nearly so.
pub struct StoredAccessToken {
    api_client: Arc<super::BitwardenApiClient>,
}

impl StoredAccessToken {
    pub fn new(api_client: Arc<super::BitwardenApiClient>) -> Self {
        Self { api_client }
    }
}

/// Deliberately opaque: `ClientManagedTokens` requires `Debug`, and a derived
/// implementation would print the access token.
impl std::fmt::Debug for StoredAccessToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("StoredAccessToken(<redacted>)")
    }
}

#[async_trait::async_trait]
impl ClientManagedTokens for StoredAccessToken {
    async fn get_access_token(&self) -> Option<String> {
        // Goes through the API client so an expired token is refreshed first:
        // `ClientManagedTokenHandler` attaches the token but never renews it.
        self.api_client.valid_access_token().await
    }
}
