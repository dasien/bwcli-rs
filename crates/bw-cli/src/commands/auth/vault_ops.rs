use crate::AppContext;
use crate::GlobalArgs;
use crate::commands::auth::{LockCommand, LogoutCommand, UnlockCommand, input};
use crate::output::{CommandOutput, CommandResult};
use anyhow::Result;
use bw_core::services::auth::AuthService;
use std::sync::Arc;

/// Execute vault unlock
pub async fn execute_unlock(
    cmd: UnlockCommand,
    global_args: &GlobalArgs,
    ctx: &AppContext,
) -> CommandResult {
    // Use services from context
    let auth_service = AuthService::new(ctx.storage(), ctx.api_client(), Arc::new(ctx.sdk().clone()));

    // Gather password
    let password = input::require_password(cmd.password, global_args, None)?;

    // Execute unlock
    let result = auth_service.unlock(password).await?;

    // Format output with session key
    // `--raw` prints only the session key, matching the TypeScript CLI (whose
    // help says so outright) and making `export BW_SESSION=$(bw unlock --raw)`
    // work. Without it, `--raw` emitted the entire instructional blurb.
    Ok(CommandOutput::success(format!(
        "Your vault is unlocked!\n\n\
         To use your vault, set your session key to the BW_SESSION environment variable. ex:\n\
         $ export BW_SESSION=\"{}\"\n\
         > $env:BW_SESSION=\"{}\"",
        result.session_key, result.session_key
    ))
    .with_raw(result.session_key))
}

/// Execute vault lock
pub async fn execute_lock(
    _cmd: LockCommand,
    _global_args: &GlobalArgs,
    ctx: &AppContext,
) -> CommandResult {
    // Use services from context
    let auth_service = AuthService::new(ctx.storage(), ctx.api_client(), Arc::new(ctx.sdk().clone()));

    // Execute lock
    auth_service.lock().await?;

    Ok(CommandOutput::success("Your vault is locked."))
}

/// Execute logout
pub async fn execute_logout(
    _cmd: LogoutCommand,
    _global_args: &GlobalArgs,
    ctx: &AppContext,
) -> CommandResult {
    // Use services from context
    let auth_service = AuthService::new(ctx.storage(), ctx.api_client(), Arc::new(ctx.sdk().clone()));

    // Execute logout
    auth_service.logout().await?;

    Ok(CommandOutput::success("You have been logged out."))
}
