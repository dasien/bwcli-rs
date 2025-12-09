use super::errors::ApiError;
use crate::models::api::token::{TokenRefreshRequest, TokenResponse};
use crate::services::storage::{JsonFileStorage, Storage};
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
        let storage = self.storage.lock().await;
        // Tokens stored unencrypted on disk (like official CLI without secure storage)
        let token_str: Option<String> = storage.get("accessToken")?;
        Ok(token_str.map(Secret::new))
    }

    /// Get current refresh token
    ///
    /// # Returns
    /// Secret-wrapped refresh token if available, None otherwise
    pub async fn get_refresh_token(&self) -> Result<Option<Secret<String>>> {
        let storage = self.storage.lock().await;
        // Tokens stored unencrypted on disk (like official CLI without secure storage)
        let token_str: Option<String> = storage.get("refreshToken")?;
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
                .ok_or_else(|| ApiError::Authentication {
                    message: "Token refresh completed but access token not found".into(),
                    hint: "Run 'bw login' to authenticate again".to_string(),
                })
                .map_err(|e| anyhow::anyhow!(e));
        }

        // No refresh in progress - start new refresh
        let refresh_lock = Arc::new(Mutex::new(()));
        let _lock_guard = refresh_lock.lock().await;

        // Store refresh lock so other requests can wait
        let mut state_guard = self.refresh_state.lock().await;
        *state_guard = Some(Arc::clone(&refresh_lock));
        drop(state_guard);

        // Get refresh token
        let refresh_token =
            self.get_refresh_token()
                .await?
                .ok_or_else(|| ApiError::Authentication {
                    message: "No refresh token available. Please log in again.".into(),
                    hint: "Run 'bw login' to authenticate again".to_string(),
                })?;

        // Build refresh request
        let request = TokenRefreshRequest {
            grant_type: "refresh_token".to_string(),
            refresh_token: refresh_token.expose_secret().clone(),
        };

        // Call refresh endpoint
        let response = refresh_client(request)
            .await
            .map_err(|e| anyhow::anyhow!("Token refresh failed: {}", e))?;

        // Persist new tokens
        {
            let mut storage = self.storage.lock().await;
            storage
                .set_secure("accessToken", &response.access_token)
                .await?;

            if let Some(new_refresh_token) = &response.refresh_token {
                storage
                    .set_secure("refreshToken", new_refresh_token)
                    .await?;
            }
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
    pub async fn save_tokens(&self, access_token: &str, refresh_token: &str) -> Result<()> {
        let mut storage = self.storage.lock().await;
        storage.set_secure("accessToken", access_token).await?;
        storage.set_secure("refreshToken", refresh_token).await?;
        Ok(())
    }

    /// Clear all stored tokens
    ///
    /// Called on logout
    pub async fn clear_tokens(&self) -> Result<()> {
        let mut storage = self.storage.lock().await;
        storage.remove_secure("accessToken").await?;
        storage.remove_secure("refreshToken").await?;
        Ok(())
    }
}
