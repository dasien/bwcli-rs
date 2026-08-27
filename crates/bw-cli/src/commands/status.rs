use crate::AppContext;
use crate::GlobalArgs;
use crate::output::{CommandResult, Response};
use bw_core::services::sdk_session;
use bw_core::services::storage::AccountManager;
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
    ctx: &AppContext,
) -> CommandResult {
    // Use services from context
    let storage = ctx.storage();

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
            // Tokens live in the SDK's state database, not `data.json`.
            let has_token = sdk_session::is_authenticated(ctx.sdk()).await;

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
                    Arc::clone(&storage),
                    Arc::new(ctx.sdk().clone()),
                    Arc::new(account_manager),
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
