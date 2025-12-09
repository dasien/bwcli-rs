use crate::GlobalArgs;
use crate::output::Response;
use bw_core::ServiceContainer;
use bw_core::services::storage::{AccountManager, StorageKey, Storage};
use bw_core::services::vault::VaultService;
use clap::Args;
use serde::Serialize;
use std::env;
use std::sync::Arc;

#[derive(Args)]
pub struct StatusCommand;

/// Status response matching TypeScript CLI format
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StatusData {
    /// Server URL (null for default cloud)
    server_url: Option<String>,
    /// Last sync timestamp
    last_sync: Option<String>,
    /// User's email address
    user_email: Option<String>,
    /// User's ID
    user_id: Option<String>,
    /// Authentication status: "unauthenticated", "locked", or "unlocked"
    status: String,
}

pub async fn execute_status(
    _cmd: StatusCommand,
    _global_args: &GlobalArgs,
) -> anyhow::Result<Response> {
    // Create service container with defaults
    let container = Arc::new(ServiceContainer::new(None, None, None, None)?);
    let storage = container.storage();

    // Create account manager to get user info
    let account_manager = AccountManager::new(Arc::clone(&storage));

    // Get active user ID
    let user_id = account_manager.get_active_user_id().await?;

    // Determine authentication status
    let (status, user_email, last_sync) = match &user_id {
        None => {
            // No active user = unauthenticated
            ("unauthenticated".to_string(), None, None)
        }
        Some(uid) => {
            // Check if we have an access token
            let storage_guard = storage.lock().await;
            let token_key = StorageKey::UserAccessToken.format(Some(uid));
            let has_token = storage_guard
                .get::<serde_json::Value>(&token_key)?
                .map(|v| matches!(v, serde_json::Value::String(s) if !s.is_empty()))
                .unwrap_or(false);
            drop(storage_guard);

            if !has_token {
                // Has user ID but no token = unauthenticated
                ("unauthenticated".to_string(), None, None)
            } else {
                // Has token, check if unlocked (session key available)
                let has_session = env::var("BW_SESSION")
                    .map(|s| !s.is_empty())
                    .unwrap_or(false);

                let auth_status = if has_session {
                    "unlocked".to_string()
                } else {
                    "locked".to_string()
                };

                // Get user email from account registry
                let account_info = account_manager.get_account(uid).await?;
                let email = account_info.map(|a| a.email);

                // Get last sync timestamp
                let vault_service = VaultService::new(
                    container.api_client(),
                    Arc::clone(&storage),
                    Arc::new(container.sdk().clone()),
                );
                let sync_time = vault_service.get_last_sync().await.ok().flatten();

                (auth_status, email, sync_time)
            }
        }
    };

    // Server URL - null for default cloud, otherwise the custom URL
    // For now, we return null since we're using default cloud
    // TODO: Read server URL from user environment settings if available
    let server_url: Option<String> = None;

    let status_data = StatusData {
        server_url,
        last_sync,
        user_email,
        user_id,
        status,
    };

    Ok(Response::success(status_data))
}
