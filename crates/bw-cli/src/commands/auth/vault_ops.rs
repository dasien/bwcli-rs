use crate::AppContext;
use crate::GlobalArgs;
use crate::commands::auth::{LockCommand, LogoutCommand, UnlockCommand, input};
use crate::output::Response;
use anyhow::Result;
use bw_core::services::auth::AuthService;

/// Execute vault unlock
pub async fn execute_unlock(cmd: UnlockCommand, global_args: &GlobalArgs, ctx: &AppContext) -> Result<Response> {
    // Use services from context
    let auth_service = AuthService::new(ctx.storage(), ctx.api_client());

    // Gather password
    let password = input::require_password(cmd.password, global_args, None)?;

    // Execute unlock
    let result = auth_service.unlock(password).await?;

    // Format output with session key
    Ok(Response::success(format!(
        "Your vault is unlocked!\n\n\
         To use your vault, set your session key to the BW_SESSION environment variable. ex:\n\
         $ export BW_SESSION=\"{}\"\n\
         > $env:BW_SESSION=\"{}\"",
        result.session_key, result.session_key
    )))
}

/// Execute vault lock
pub async fn execute_lock(_cmd: LockCommand, _global_args: &GlobalArgs, ctx: &AppContext) -> Result<Response> {
    // Use services from context
    let auth_service = AuthService::new(ctx.storage(), ctx.api_client());

    // Execute lock
    auth_service.lock().await?;

    Ok(Response::success("Your vault is locked."))
}

/// Execute logout
pub async fn execute_logout(_cmd: LogoutCommand, _global_args: &GlobalArgs, ctx: &AppContext) -> Result<Response> {
    // Use services from context
    let auth_service = AuthService::new(ctx.storage(), ctx.api_client());

    // Execute logout
    auth_service.logout().await?;

    Ok(Response::success("You have been logged out."))
}

