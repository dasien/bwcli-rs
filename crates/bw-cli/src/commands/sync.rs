use crate::AppContext;
use crate::GlobalArgs;
use crate::auth_gate::Unlocked;
use crate::output::{CommandOutput, CommandResult};
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
    unlocked: &Unlocked<'_>,
) -> CommandResult {
    // Use services from context
    let account_manager = Arc::new(AccountManager::new(ctx.storage()));

    // Create vault service
    let vault_service = VaultService::new(
        ctx.storage(),
        Arc::new(ctx.sdk().clone()),
        account_manager,
    );

    // Handle --last flag
    if cmd.last {
        match vault_service.get_last_sync().await? {
            Some(timestamp) => Ok(CommandOutput::success_message(timestamp)),
            None => Err(anyhow::Error::msg("Never synced")),
        }
    } else {
        // Perform sync
        match vault_service.sync(cmd.force).await {
            Ok(timestamp) => Ok(CommandOutput::success_message(format!(
                "Syncing complete. Last sync: {}",
                timestamp
            ))),
            Err(e) => Err(anyhow::Error::msg(e.to_string())),
        }
    }
}
