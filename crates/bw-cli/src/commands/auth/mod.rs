mod login;
mod prompts;
mod vault_ops;

use crate::GlobalArgs;
use crate::output::Response;
use clap::{Args, Subcommand};

// Re-export command implementations
pub use login::{execute_api_key_login, execute_password_login};
pub use vault_ops::{execute_lock, execute_logout, execute_unlock};

/// Authentication subcommands for login
#[derive(Subcommand)]
pub enum AuthCommands {
    /// Log in using email and master password
    Password(LoginPasswordCommand),

    /// Log in using API key
    ApiKey(LoginApiKeyCommand),

    /// Log in using SSO (not yet implemented)
    #[command(hide = true)]
    Sso(LoginSsoCommand),
}

/// Password login command
#[derive(Args)]
pub struct LoginPasswordCommand {
    /// Email address
    #[arg(value_name = "EMAIL")]
    pub email: Option<String>,

    /// Master password
    #[arg(value_name = "PASSWORD")]
    pub password: Option<String>,

    /// Two-step login code
    #[arg(long)]
    pub code: Option<String>,

    /// Two-step login method (0=Authenticator, 1=Email, etc.)
    #[arg(long)]
    pub method: Option<u8>,
}

/// API key login command
#[derive(Args)]
pub struct LoginApiKeyCommand {
    /// Client ID
    #[arg(long)]
    pub client_id: Option<String>,

    /// Client secret
    #[arg(long)]
    pub client_secret: Option<String>,
}

/// SSO login command (not yet implemented)
#[derive(Args)]
pub struct LoginSsoCommand {
    /// Organization identifier
    #[arg(long)]
    pub org_identifier: Option<String>,
}

/// Logout command
#[derive(Args)]
pub struct LogoutCommand;

/// Lock vault command
#[derive(Args)]
pub struct LockCommand;

/// Unlock vault command
#[derive(Args)]
pub struct UnlockCommand {
    /// Master password
    #[arg(value_name = "PASSWORD")]
    pub password: Option<String>,
}

/// Execute login command (routes to appropriate handler)
pub async fn execute_login(
    cmd: AuthCommands,
    global_args: &GlobalArgs,
) -> anyhow::Result<Response> {
    match cmd {
        AuthCommands::Password(password_cmd) => {
            execute_password_login(password_cmd, global_args).await
        }
        AuthCommands::ApiKey(apikey_cmd) => execute_api_key_login(apikey_cmd, global_args).await,
        AuthCommands::Sso(_) => Ok(Response::error(
            "SSO login is not yet implemented. It will be added in a future release.",
        )),
    }
}
