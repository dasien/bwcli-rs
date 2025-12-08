use crate::GlobalArgs;
use crate::commands::auth::{LockCommand, LogoutCommand, UnlockCommand, prompts};
use crate::output::Response;
use anyhow::Result;
use bw_core::services::ServiceContainer;
use bw_core::services::auth::AuthService;
use secrecy::Secret;

/// Execute vault unlock
pub async fn execute_unlock(cmd: UnlockCommand, global_args: &GlobalArgs) -> Result<Response> {
    // Initialize services
    let container = ServiceContainer::new(None, None, None, None)?;
    let auth_service = AuthService::new(container.storage(), container.api_client());

    // Gather password
    let password = get_password_input(cmd.password, global_args)?;

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
pub async fn execute_lock(_cmd: LockCommand, _global_args: &GlobalArgs) -> Result<Response> {
    // Initialize services
    let container = ServiceContainer::new(None, None, None, None)?;
    let auth_service = AuthService::new(container.storage(), container.api_client());

    // Execute lock
    auth_service.lock().await?;

    Ok(Response::success("Your vault is locked."))
}

/// Execute logout
pub async fn execute_logout(_cmd: LogoutCommand, global_args: &GlobalArgs) -> Result<Response> {
    // Confirmation prompt (unless --nointeraction)
    if !global_args.nointeraction {
        let confirmed = prompts::prompt_confirmation("Are you sure you want to log out?")?;
        if !confirmed {
            return Ok(Response::error("Logout cancelled"));
        }
    }

    // Initialize services
    let container = ServiceContainer::new(None, None, None, None)?;
    let auth_service = AuthService::new(container.storage(), container.api_client());

    // Execute logout
    auth_service.logout().await?;

    Ok(Response::success("You have been logged out."))
}

// Helper functions

fn get_password_input(
    password_arg: Option<String>,
    global_args: &GlobalArgs,
) -> Result<Secret<String>> {
    if let Some(password) = password_arg {
        return Ok(Secret::new(password));
    }

    if global_args.nointeraction {
        anyhow::bail!(
            "Password is required. Use --nointeraction=false or provide PASSWORD argument."
        );
    }

    prompts::prompt_password(None)
}
