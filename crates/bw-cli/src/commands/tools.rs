use crate::AppContext;
use crate::GlobalArgs;
use crate::commands::vault::{create_vault_service, create_write_service};
use crate::auth_gate::Unlocked;
use crate::output::{CommandOutput, CommandResult};
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
    /// Data to encode. Read from stdin when omitted, which is the TypeScript
    /// CLI's only form ("Base 64 encode stdin") and the one its own docs pipe
    /// into `create`, `edit` and `move`.
    #[arg(value_name = "DATA")]
    pub data: Option<String>,
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
) -> CommandResult {
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
            Ok(CommandOutput::success_json(serde_json::json!({
                "data": result
            })))
        } else {
            Ok(CommandOutput::success_raw(result))
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
            // Generator options added in SDK 3.0 that the CLI does not expose
            // yet (custom charsets, consecutive-character limit).
            ..Default::default()
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
            Ok(CommandOutput::success_json(serde_json::json!({
                "data": result
            })))
        } else {
            Ok(CommandOutput::success_raw(result))
        }
    }
}

pub async fn execute_encode(
    cmd: EncodeCommand,
    global_args: &GlobalArgs,
    _ctx: &AppContext,
) -> CommandResult {
    use base64::{Engine as _, engine::general_purpose};

    let data = match cmd.data {
        Some(data) => data,
        None => {
            use std::io::Read;
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf)?;
            // A trailing newline from the shell is not part of the payload, and
            // encoding it changes the base64 the next command receives.
            buf.trim_end_matches(['\n', '\r']).to_string()
        }
    };

    let encoded = general_purpose::STANDARD.encode(&data);

    if global_args.response {
        Ok(CommandOutput::success_json(serde_json::json!({
            "data": encoded
        })))
    } else {
        Ok(CommandOutput::success_raw(encoded))
    }
}

pub async fn execute_decrypt(
    _cmd: DecryptCommand,
    _global_args: &GlobalArgs,
    _ctx: &AppContext,
) -> CommandResult {
    Err(anyhow::Error::msg("Not yet implemented"))
}

pub async fn execute_import(
    cmd: ImportCommand,
    global_args: &GlobalArgs,
    ctx: &AppContext,
    unlocked: &Unlocked<'_>,
) -> CommandResult {
    use bw_core::services::import_export::{ImportOptions, ImportService};
    use std::collections::HashMap;

    let session = global_args.session.as_deref().unwrap_or("");
    if session.is_empty() {
        anyhow::bail!("Vault is locked. Run 'bw unlock' and set BW_SESSION.");
    }

    if cmd.organizationid.is_some() {
        anyhow::bail!("Importing into an organization is not supported yet.");
    }

    let import_service = ImportService::new();
    let data = import_service
        .parse_file(&cmd.format, &cmd.file, ImportOptions::default())
        .await?;

    let vault_service = create_vault_service(ctx);
    let write_service = create_write_service(ctx, global_args.nointeraction);

    // Folder names are the only link the intermediate format carries, so build
    // a name -> id map from what already exists and create whatever is missing.
    let mut folder_ids: HashMap<String, bitwarden_vault::FolderId> = vault_service
        .list_folders(None, session)
        .await?
        .into_iter()
        .filter_map(|f| f.id.map(|id| (f.name, id)))
        .collect();

    let mut folders_created = 0usize;
    for folder in &data.folders {
        if folder.name.is_empty() || folder_ids.contains_key(&folder.name) {
            continue;
        }

        let created = write_service
            .create_folder(folder.name.clone(), session)
            .await?;
        if let Some(id) = created.id {
            folder_ids.insert(folder.name.clone(), id);
        }
        folders_created += 1;
    }

    // Create items one at a time. The server has a bulk import endpoint that
    // would be far fewer round trips for large exports; see
    // docs/sdk-3.0-migration.md.
    let mut items_created = 0usize;
    let mut failures: Vec<String> = Vec::new();

    for item in &data.items {
        let folder_id = item
            .folder_name
            .as_ref()
            .and_then(|name| folder_ids.get(name).copied());

        match write_service
            .create_cipher(item.to_cipher_view(folder_id), session)
            .await
        {
            Ok(_) => items_created += 1,
            Err(e) => failures.push(format!("{}: {}", item.name, e)),
        }
    }

    if !failures.is_empty() {
        eprintln!("{} item(s) could not be imported:", failures.len());
        for failure in &failures {
            eprintln!("  {failure}");
        }
    }

    if global_args.response {
        Ok(CommandOutput::success(serde_json::json!({
            "format": cmd.format,
            "itemsCreated": items_created,
            "foldersCreated": folders_created,
            "failed": failures.len(),
        })))
    } else {
        Ok(CommandOutput::success_raw(format!(
            "Imported {items_created} item(s) and {folders_created} folder(s)."
        )))
    }
}

pub async fn execute_export(
    cmd: ExportCommand,
    global_args: &GlobalArgs,
    ctx: &AppContext,
    unlocked: &Unlocked<'_>,
) -> CommandResult {
    use bw_core::services::import_export::{ExportData, ExportOptions, ExportService};
    use secrecy::Secret;
    use std::sync::Arc;

    // Export decrypts through the SDK key store, which is only populated when
    // the vault is unlocked.
    if global_args.session.as_deref().unwrap_or("").is_empty() {
        anyhow::bail!("Vault is locked. Run 'bw unlock' and set BW_SESSION.");
    }

    // Refuse before reading the vault: the SDK's organization export is a
    // `todo!()` that would abort the process.
    if cmd.organizationid.is_some() {
        anyhow::bail!("Exporting an organization vault is not supported yet.");
    }

    // Matches the TypeScript CLI's default.
    let format = cmd.format.as_deref().unwrap_or("csv");

    let vault_service = create_vault_service(ctx);
    let folders = vault_service.encrypted_folders().await?;
    let ciphers = vault_service.encrypted_ciphers().await?;

    let service = ExportService::new(Arc::new(ctx.sdk().clone()));
    let result = service
        .export(
            format,
            cmd.output.as_deref(),
            ExportData { folders, ciphers },
            ExportOptions {
                password: cmd.password.map(Secret::new),
                organization_id: cmd.organizationid,
            },
        )
        .await?;

    // With `--response` the JSON is the payload, so the document belongs inside
    // it. Writing it separately as well would put two documents on stdout.
    if global_args.response {
        return Ok(CommandOutput::success(serde_json::json!({
            "format": result.format,
            "itemCount": result.item_count,
            "encrypted": result.encrypted,
            "output": result.output_path,
            "data": result.contents,
        })));
    }

    match (result.contents, &result.output_path) {
        // Exported to stdout: the document *is* the output, so nothing else may
        // go there. The count is progress information, and progress goes to
        // stderr — otherwise `bw export --format json > vault.json` writes a
        // file no JSON parser will accept.
        (Some(contents), _) => {
            // The count is progress information, and progress goes to stderr —
            // otherwise `bw export --format json > vault.json` writes a file no
            // JSON parser will accept. The renderer writes the document itself.
            eprintln!("Exported {} item(s)", result.item_count);
            Ok(CommandOutput::bytes(contents.into_bytes()))
        }
        // Exported to a file: stdout carries no payload, so say what happened.
        (None, Some(path)) => Ok(CommandOutput::success_raw(format!(
            "Saved {} item(s) to {}",
            result.item_count, path
        ))),
        (None, None) => Ok(CommandOutput::success_raw(format!(
            "Exported {} item(s)",
            result.item_count
        ))),
    }
}
