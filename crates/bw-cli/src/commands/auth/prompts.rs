use anyhow::Result;
use bw_core::models::auth::TwoFactorMethod;
use dialoguer::{Confirm, Input, Password, Select};
use secrecy::Secret;

/// Prompt for email address
pub fn prompt_email() -> Result<String> {
    let email: String = Input::new().with_prompt("Email address").interact_text()?;

    validate_email(&email)?;
    Ok(email)
}

/// Prompt for password (hidden input)
pub fn prompt_password(prompt_text: Option<&str>) -> Result<Secret<String>> {
    let prompt = prompt_text.unwrap_or("Master password");

    let password = Password::new().with_prompt(prompt).interact()?;

    if password.is_empty() {
        anyhow::bail!("Password cannot be empty");
    }

    Ok(Secret::new(password))
}

/// Prompt for client ID (API key login)
pub fn prompt_client_id() -> Result<String> {
    let client_id: String = Input::new().with_prompt("Client ID").interact_text()?;

    if client_id.is_empty() {
        anyhow::bail!("Client ID cannot be empty");
    }

    Ok(client_id)
}

/// Prompt for client secret (API key login)
pub fn prompt_client_secret() -> Result<Secret<String>> {
    let client_secret = Password::new().with_prompt("Client secret").interact()?;

    if client_secret.is_empty() {
        anyhow::bail!("Client secret cannot be empty");
    }

    Ok(Secret::new(client_secret))
}

/// Prompt for 2FA method selection
pub fn prompt_two_factor_method(available_methods: &[TwoFactorMethod]) -> Result<TwoFactorMethod> {
    let methods: Vec<&str> = available_methods.iter().map(|m| m.display_name()).collect();

    let selection = Select::new()
        .with_prompt("Two-step login method")
        .items(&methods)
        .default(0)
        .interact()?;

    Ok(available_methods[selection])
}

/// Prompt for 2FA code
pub fn prompt_two_factor_code(method: TwoFactorMethod) -> Result<String> {
    let prompt = match method {
        TwoFactorMethod::Authenticator => "Authenticator app code",
        TwoFactorMethod::Email => "Email verification code",
        TwoFactorMethod::YubiKey => "YubiKey OTP",
        _ => "Two-factor code",
    };

    let code: String = Input::new().with_prompt(prompt).interact_text()?;

    // Basic validation: TOTP codes are 6 digits
    if method == TwoFactorMethod::Authenticator {
        if !code.chars().all(|c| c.is_ascii_digit()) || code.len() != 6 {
            anyhow::bail!("Authenticator code must be 6 digits");
        }
    }

    Ok(code)
}

/// Prompt for confirmation
pub fn prompt_confirmation(prompt: &str) -> Result<bool> {
    Confirm::new()
        .with_prompt(prompt)
        .default(false)
        .interact()
        .map_err(Into::into)
}

/// Prompt for new device verification OTP (sent via email)
pub fn prompt_device_verification_otp() -> Result<String> {
    eprintln!("New device verification required. Enter OTP sent to login email:");

    let otp: String = Input::new()
        .with_prompt("Device verification code")
        .interact_text()?;

    if otp.is_empty() {
        anyhow::bail!("Device verification code cannot be empty");
    }

    Ok(otp)
}

/// Validate email format
fn validate_email(email: &str) -> Result<()> {
    // Basic email validation: must contain @ and .
    if !email.contains('@') || !email.contains('.') {
        anyhow::bail!("Invalid email format");
    }

    if email.len() < 3 {
        anyhow::bail!("Email too short");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_email() {
        assert!(validate_email("test@example.com").is_ok());
        assert!(validate_email("invalid").is_err());
        assert!(validate_email("@.").is_err());
        assert!(validate_email("no-at-sign.com").is_err());
    }
}
