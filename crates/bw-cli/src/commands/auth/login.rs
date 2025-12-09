use crate::GlobalArgs;
use crate::commands::auth::{LoginApiKeyCommand, LoginPasswordCommand, prompts};
use crate::output::Response;
use anyhow::Result;
use bw_core::services::ServiceContainer;
use bw_core::services::auth::{AuthError, AuthService};
use secrecy::Secret;

/// Execute password-based login
pub async fn execute_password_login(
    cmd: LoginPasswordCommand,
    global_args: &GlobalArgs,
) -> Result<Response> {
    // Initialize services
    let container = ServiceContainer::new(None, None, None, None)?;
    let auth_service = AuthService::new(container.storage(), container.api_client());

    // Gather inputs (prompt if missing and interactive mode allowed)
    let email = get_email_input(cmd.email, global_args)?;
    let password = get_password_input(cmd.password.clone(), global_args)?;

    // Build 2FA data if provided
    let two_factor = if let Some(code) = cmd.code.clone() {
        Some(bw_core::services::auth::TwoFactorData {
            token: code,
            provider: cmd.method.unwrap_or(0),
            remember: false,
        })
    } else {
        None
    };

    // Execute login (first attempt without device verification OTP)
    let result = auth_service
        .login_with_password(&email, password.clone(), two_factor.clone(), None)
        .await;

    // Handle the result
    match result {
        Ok(login_result) => {
            // Format output with session key
            Ok(Response::success(format!(
                "You are logged in!\n\n\
                 To unlock your vault, set your session key to the BW_SESSION environment variable. ex:\n\
                 $ export BW_SESSION=\"{}\"\n\
                 > $env:BW_SESSION=\"{}\"\n\n\
                 You can also pass the session key to any command with the --session option. ex:\n\
                 $ bw list items --session {}",
                login_result.session_key, login_result.session_key, login_result.session_key
            )))
        }
        Err(AuthError::NewDeviceVerificationRequired) => {
            // New device verification required - prompt for OTP
            if global_args.nointeraction {
                anyhow::bail!(
                    "New device verification required. Check your email for the verification code \
                     and provide it via the --code option, or disable --nointeraction to be prompted."
                );
            }

            // Prompt for OTP
            let otp = prompts::prompt_device_verification_otp()?;

            // Retry login with OTP
            let retry_result = auth_service
                .login_with_password(&email, password, two_factor, Some(otp))
                .await?;

            Ok(Response::success(format!(
                "You are logged in!\n\n\
                 To unlock your vault, set your session key to the BW_SESSION environment variable. ex:\n\
                 $ export BW_SESSION=\"{}\"\n\
                 > $env:BW_SESSION=\"{}\"\n\n\
                 You can also pass the session key to any command with the --session option. ex:\n\
                 $ bw list items --session {}",
                retry_result.session_key, retry_result.session_key, retry_result.session_key
            )))
        }
        Err(e) => Err(e.into()),
    }
}

/// Execute API key-based login
pub async fn execute_api_key_login(
    cmd: LoginApiKeyCommand,
    global_args: &GlobalArgs,
) -> Result<Response> {
    // Initialize services
    let container = ServiceContainer::new(None, None, None, None)?;
    let auth_service = AuthService::new(container.storage(), container.api_client());

    // Gather inputs
    let client_id = get_client_id_input(cmd.client_id, global_args)?;
    let client_secret = get_client_secret_input(cmd.client_secret, global_args)?;

    // Execute login
    let result = auth_service
        .login_with_api_key(&client_id, client_secret)
        .await?;

    // Format output with session key
    Ok(Response::success(format!(
        "You are logged in!\n\n\
         To unlock your vault, set your session key to the BW_SESSION environment variable. ex:\n\
         $ export BW_SESSION=\"{}\"\n\
         > $env:BW_SESSION=\"{}\"\n\n\
         You can also pass the session key to any command with the --session option. ex:\n\
         $ bw list items --session {}",
        result.session_key, result.session_key, result.session_key
    )))
}

// Helper functions for input gathering

fn get_email_input(email_arg: Option<String>, global_args: &GlobalArgs) -> Result<String> {
    if let Some(email) = email_arg {
        return Ok(email);
    }

    if global_args.nointeraction {
        anyhow::bail!("Email is required. Use --nointeraction=false or provide EMAIL argument.");
    }

    prompts::prompt_email()
}

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

fn get_client_id_input(client_id_arg: Option<String>, global_args: &GlobalArgs) -> Result<String> {
    if let Some(client_id) = client_id_arg {
        return Ok(client_id);
    }

    if global_args.nointeraction {
        anyhow::bail!(
            "Client ID is required. Use --nointeraction=false or provide --client-id argument."
        );
    }

    prompts::prompt_client_id()
}

fn get_client_secret_input(
    client_secret_arg: Option<String>,
    global_args: &GlobalArgs,
) -> Result<Secret<String>> {
    if let Some(client_secret) = client_secret_arg {
        return Ok(Secret::new(client_secret));
    }

    if global_args.nointeraction {
        anyhow::bail!(
            "Client secret is required. Use --nointeraction=false or provide --client-secret argument."
        );
    }

    prompts::prompt_client_secret()
}
