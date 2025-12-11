use crate::AppContext;
use crate::GlobalArgs;
use crate::output::Response;
use bw_core::services::storage::AccountManager;
use bw_core::services::vault::VaultService;
use clap::Args;
use std::sync::Arc;

#[derive(Args)]
pub struct SyncCommand {
    /// Force full sync
    #[arg(long)]
    pub force: bool,

    /// Sync only this session (no server communication)
    #[arg(long)]
    pub last: bool,
}

pub async fn execute_sync(
    cmd: SyncCommand,
    _global_args: &GlobalArgs,
    ctx: &AppContext,
) -> anyhow::Result<Response> {
    // Use services from context
    let account_manager = Arc::new(AccountManager::new(ctx.storage()));

    // Create vault service
    let vault_service = VaultService::new(
        ctx.api_client(),
        ctx.storage(),
        Arc::new(ctx.sdk().clone()),
        account_manager,
    );

    // Handle --last flag
    if cmd.last {
        match vault_service.get_last_sync().await? {
            Some(timestamp) => Ok(Response::success_message(timestamp)),
            None => Ok(Response::error("Never synced")),
        }
    } else {
        // Perform sync
        match vault_service.sync(cmd.force).await {
            Ok(timestamp) => Ok(Response::success_message(format!(
                "Syncing complete. Last sync: {}",
                timestamp
            ))),
            Err(e) => Ok(Response::error(e.to_string())),
        }
    }
}
