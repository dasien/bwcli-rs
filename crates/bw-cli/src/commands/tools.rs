use crate::GlobalArgs;
use crate::output::Response;
use clap::Args;

#[derive(Args)]
pub struct GenerateCommand {
    /// Generate a passphrase instead of password
    #[arg(long)]
    pub passphrase: bool,

    /// Password length (default: 14)
    #[arg(long)]
    pub length: Option<usize>,

    /// Minimum number of lowercase characters
    #[arg(long)]
    pub lowercase: Option<usize>,

    /// Minimum number of uppercase characters
    #[arg(long)]
    pub uppercase: Option<usize>,

    /// Minimum number of numeric characters
    #[arg(long)]
    pub number: Option<usize>,

    /// Minimum number of special characters
    #[arg(long)]
    pub special: Option<usize>,

    /// Number of passphrase words (default: 3)
    #[arg(long)]
    pub words: Option<usize>,

    /// Passphrase word separator (default: -)
    #[arg(long)]
    pub separator: Option<String>,

    /// Capitalize passphrase words
    #[arg(long)]
    pub capitalize: bool,

    /// Include number in passphrase
    #[arg(long, alias = "includeNumber")]
    pub include_number: bool,
}

#[derive(Args)]
pub struct EncodeCommand {
    /// Data to encode
    #[arg(value_name = "DATA")]
    pub data: String,
}

#[derive(Args)]
pub struct DecryptCommand {
    /// Encrypted string to decrypt
    #[arg(value_name = "ENCRYPTED")]
    pub encrypted: String,

    /// Organization ID (for org-encrypted data)
    #[arg(long)]
    pub organizationid: Option<String>,
}

#[derive(Args)]
pub struct ImportCommand {
    /// Import format (bitwardenjson, lastpass, etc.)
    #[arg(value_name = "FORMAT")]
    pub format: String,

    /// Input file path
    #[arg(value_name = "FILE")]
    pub file: String,

    /// Organization ID (import to org)
    #[arg(long)]
    pub organizationid: Option<String>,
}

#[derive(Args)]
pub struct ExportCommand {
    /// Export format (json, csv, encrypted_json)
    #[arg(long)]
    pub format: Option<String>,

    /// Master password (required for encrypted export)
    #[arg(long)]
    pub password: Option<String>,

    /// Organization ID (export org vault)
    #[arg(long)]
    pub organizationid: Option<String>,

    /// Output file path
    #[arg(long)]
    pub output: Option<String>,
}

pub async fn execute_generate(
    cmd: GenerateCommand,
    global_args: &GlobalArgs,
) -> anyhow::Result<Response> {
    use bw_core::services::generator::{
        PassphraseOptions, PasswordOptions, generate_passphrase, generate_password,
    };

    if cmd.passphrase {
        // Generate passphrase
        let options = PassphraseOptions {
            num_words: cmd.words.unwrap_or(3),
            separator: cmd.separator.unwrap_or_else(|| "-".to_string()),
            capitalize: cmd.capitalize,
            include_number: cmd.include_number,
        };

        let passphrase = generate_passphrase(&options)?;

        if global_args.response {
            Ok(Response::success_json(serde_json::json!({
                "data": passphrase
            })))
        } else {
            Ok(Response::success_raw(passphrase))
        }
    } else {
        // Generate password
        let mut options = PasswordOptions {
            length: cmd.length.unwrap_or(14),
            include_lowercase: true,
            include_uppercase: true,
            include_numbers: true,
            include_special: true,
            min_lowercase: cmd.lowercase.unwrap_or(0),
            min_uppercase: cmd.uppercase.unwrap_or(0),
            min_numbers: cmd.number.unwrap_or(1),
            min_special: cmd.special.unwrap_or(1),
            exclude_chars: None,
        };

        // If any minimums are explicitly set to 0, disable that character set
        // This matches TypeScript CLI behavior
        if cmd.lowercase == Some(0) {
            options.include_lowercase = false;
        }
        if cmd.uppercase == Some(0) {
            options.include_uppercase = false;
        }
        if cmd.number == Some(0) {
            options.include_numbers = false;
        }
        if cmd.special == Some(0) {
            options.include_special = false;
        }

        let password = generate_password(&options)?;

        if global_args.response {
            Ok(Response::success_json(serde_json::json!({
                "data": password
            })))
        } else {
            Ok(Response::success_raw(password))
        }
    }
}

pub async fn execute_encode(
    cmd: EncodeCommand,
    global_args: &GlobalArgs,
) -> anyhow::Result<Response> {
    use base64::{Engine as _, engine::general_purpose};

    let encoded = general_purpose::STANDARD.encode(&cmd.data);

    if global_args.response {
        Ok(Response::success_json(serde_json::json!({
            "data": encoded
        })))
    } else {
        Ok(Response::success_raw(encoded))
    }
}

pub async fn execute_decrypt(
    _cmd: DecryptCommand,
    _global_args: &GlobalArgs,
) -> anyhow::Result<Response> {
    Ok(Response::error("Not yet implemented"))
}

pub async fn execute_import(
    _cmd: ImportCommand,
    _global_args: &GlobalArgs,
) -> anyhow::Result<Response> {
    Ok(Response::error("Not yet implemented"))
}

pub async fn execute_export(
    _cmd: ExportCommand,
    _global_args: &GlobalArgs,
) -> anyhow::Result<Response> {
    Ok(Response::error("Not yet implemented"))
}
