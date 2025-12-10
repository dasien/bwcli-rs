use crate::AppContext;
use crate::GlobalArgs;
use crate::commands::auth::{LoginApiKeyCommand, LoginPasswordCommand, input, prompts};
use crate::output::Response;
use anyhow::Result;
use bw_core::models::auth::TwoFactorMethod;
use bw_core::services::auth::{AuthError, AuthService};

/// Execute password-based login
pub async fn execute_password_login(
    cmd: LoginPasswordCommand,
    global_args: &GlobalArgs,
    ctx: &AppContext,
) -> Result<Response> {
    // Use services from context
    let auth_service = AuthService::new(ctx.storage(), ctx.api_client());

    // Gather inputs (prompt if missing and interactive mode allowed)
    let email = input::require_string(cmd.email, global_args, "Email", prompts::prompt_email)?;
    let password = input::require_password(cmd.password, global_args, None)?;

    // Build 2FA data if provided
    let two_factor = cmd.code.as_ref().map(|code| {
        bw_core::services::auth::TwoFactorData {
            token: code.clone(),
            provider: cmd.method.unwrap_or(TwoFactorMethod::Authenticator as u8),
            remember: false,
        }
    });

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
    ctx: &AppContext,
) -> Result<Response> {
    // Use services from context
    let auth_service = AuthService::new(ctx.storage(), ctx.api_client());

    // Gather inputs
    let client_id = input::require_string(cmd.client_id, global_args, "Client ID", prompts::prompt_client_id)?;
    let client_secret = input::require_secret(cmd.client_secret, global_args, "Client secret", prompts::prompt_client_secret)?;

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

