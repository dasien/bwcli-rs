use crate::AppContext;
use crate::GlobalArgs;
use crate::output::Response;
use clap::Args;

#[derive(Args)]
pub struct GenerateCommand {
    /// Generate a passphrase instead of password
    #[arg(long)]
    pub passphrase: bool,

    /// Password length (default: 16)
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
    _ctx: &AppContext,
) -> anyhow::Result<Response> {
    use bitwarden_core::Client;
    use bitwarden_generators::{
        GeneratorClientsExt, PassphraseError, PassphraseGeneratorRequest, PasswordError,
        PasswordGeneratorRequest,
    };

    // RNG Note: The SDK uses rand::thread_rng() which is a ChaCha12 CSPRNG
    // seeded from OsRng. This is cryptographically secure and equivalent
    // to our previous direct OsRng usage. The thread-local design provides
    // better performance for repeated calls while maintaining security.

    // Create a minimal SDK client for generator operations
    let client = Client::new(None);
    let generator = client.generator();

    if cmd.passphrase {
        // Generate passphrase using SDK
        let request = PassphraseGeneratorRequest {
            num_words: cmd.words.unwrap_or(3) as u8,
            word_separator: cmd.separator.unwrap_or_else(|| "-".to_string()),
            capitalize: cmd.capitalize,
            include_number: cmd.include_number,
        };

        let result = generator.passphrase(request).map_err(|e| match e {
            PassphraseError::InvalidNumWords { minimum, maximum } => {
                anyhow::anyhow!(
                    "Invalid word count. Number of words must be between {} and {}",
                    minimum,
                    maximum
                )
            }
        })?;

        if global_args.response {
            Ok(Response::success_json(serde_json::json!({
                "data": result
            })))
        } else {
            Ok(Response::success_raw(result))
        }
    } else {
        // Generate password using SDK
        //
        // Character set logic:
        // - If minimum is explicitly set to 0, disable that character set
        // - Otherwise, enable the character set with the specified minimum
        // - Default behavior: all character sets enabled with special chars included
        //   (preserves backward compatibility with current CLI behavior)
        let lowercase_enabled = cmd.lowercase != Some(0);
        let uppercase_enabled = cmd.uppercase != Some(0);
        let numbers_enabled = cmd.number != Some(0);
        let special_enabled = cmd.special != Some(0);

        let request = PasswordGeneratorRequest {
            length: cmd.length.unwrap_or(16) as u8,
            lowercase: lowercase_enabled,
            uppercase: uppercase_enabled,
            numbers: numbers_enabled,
            special: special_enabled,
            avoid_ambiguous: false,
            min_lowercase: cmd.lowercase.filter(|&v| v > 0).map(|v| v as u8),
            min_uppercase: cmd.uppercase.filter(|&v| v > 0).map(|v| v as u8),
            min_number: cmd.number.filter(|&v| v > 0).map(|v| v as u8),
            min_special: cmd.special.filter(|&v| v > 0).map(|v| v as u8),
        };

        let result = generator.password(request).map_err(|e| match e {
            PasswordError::NoCharacterSetEnabled => {
                anyhow::anyhow!(
                    "No character sets enabled. Enable at least one of: lowercase, uppercase, numbers, or special characters"
                )
            }
            PasswordError::InvalidLength => {
                anyhow::anyhow!(
                    "Invalid password length. Length must be at least 4 and greater than the sum of minimum character requirements"
                )
            }
        })?;

        if global_args.response {
            Ok(Response::success_json(serde_json::json!({
                "data": result
            })))
        } else {
            Ok(Response::success_raw(result))
        }
    }
}

pub async fn execute_encode(
    cmd: EncodeCommand,
    global_args: &GlobalArgs,
    _ctx: &AppContext,
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
    _ctx: &AppContext,
) -> anyhow::Result<Response> {
    Ok(Response::error("Not yet implemented"))
}

pub async fn execute_import(
    _cmd: ImportCommand,
    _global_args: &GlobalArgs,
    _ctx: &AppContext,
) -> anyhow::Result<Response> {
    Ok(Response::error("Not yet implemented"))
}

pub async fn execute_export(
    _cmd: ExportCommand,
    _global_args: &GlobalArgs,
    _ctx: &AppContext,
) -> anyhow::Result<Response> {
    Ok(Response::error("Not yet implemented"))
}
