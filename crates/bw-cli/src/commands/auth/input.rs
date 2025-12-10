//! Input gathering helpers for CLI commands
//!
//! Provides common functions for gathering user input, either from command
//! arguments or interactive prompts.

use super::prompts;
use crate::GlobalArgs;
use anyhow::Result;
use secrecy::Secret;

/// Require a password, either from argument or interactive prompt
///
/// # Arguments
/// * `password_arg` - Password provided as command argument
/// * `global_args` - Global CLI arguments (for nointeraction check)
/// * `prompt_text` - Optional custom prompt text
///
/// # Errors
/// Returns error if password is required but not provided and nointeraction is set
pub fn require_password(
    password_arg: Option<String>,
    global_args: &GlobalArgs,
    prompt_text: Option<&str>,
) -> Result<Secret<String>> {
    if let Some(password) = password_arg {
        return Ok(Secret::new(password));
    }

    if global_args.nointeraction {
        anyhow::bail!(
            "Password is required. Use --nointeraction=false or provide PASSWORD argument."
        );
    }

    prompts::prompt_password(prompt_text)
}

/// Require a string value, either from argument or interactive prompt
///
/// # Arguments
/// * `arg` - Value provided as command argument
/// * `global_args` - Global CLI arguments (for nointeraction check)
/// * `field_name` - Name of the field for error messages
/// * `prompt_fn` - Function to call for interactive prompt
///
/// # Errors
/// Returns error if value is required but not provided and nointeraction is set
pub fn require_string<F>(
    arg: Option<String>,
    global_args: &GlobalArgs,
    field_name: &str,
    prompt_fn: F,
) -> Result<String>
where
    F: FnOnce() -> Result<String>,
{
    if let Some(value) = arg {
        return Ok(value);
    }

    if global_args.nointeraction {
        anyhow::bail!(
            "{} is required. Provide it as an argument or disable --nointeraction.",
            field_name
        );
    }

    prompt_fn()
}

/// Require a secret value, either from argument or interactive prompt
///
/// # Arguments
/// * `arg` - Value provided as command argument
/// * `global_args` - Global CLI arguments (for nointeraction check)
/// * `field_name` - Name of the field for error messages
/// * `prompt_fn` - Function to call for interactive prompt
///
/// # Errors
/// Returns error if value is required but not provided and nointeraction is set
pub fn require_secret<F>(
    arg: Option<String>,
    global_args: &GlobalArgs,
    field_name: &str,
    prompt_fn: F,
) -> Result<Secret<String>>
where
    F: FnOnce() -> Result<Secret<String>>,
{
    if let Some(value) = arg {
        return Ok(Secret::new(value));
    }

    if global_args.nointeraction {
        anyhow::bail!(
            "{} is required. Provide it as an argument or disable --nointeraction.",
            field_name
        );
    }

    prompt_fn()
}
