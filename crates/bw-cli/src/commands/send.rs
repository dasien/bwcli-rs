use crate::AppContext;
use crate::GlobalArgs;
use crate::commands::input::get_json_string;
use crate::auth_gate::{Need, Requires, Unlocked, require};
use crate::output::{CommandOutput, CommandResult};
use bitwarden_send::{
    AuthEdit, SendAddRequest, SendAuthType, SendClientExt, SendEditRequest, SendId, SendTextView,
    SendView, SendViewType,
};
use chrono::{DateTime, Duration, Utc};
use clap::{Args, Subcommand};
use serde::Deserialize;
use std::str::FromStr;

#[derive(Subcommand)]
pub enum SendCommands {
    /// List all Sends
    List(SendListCommand),

    /// Get Send template
    Template(SendTemplateCommand),

    /// Get Send by ID
    Get(SendGetCommand),

    /// Create a new Send
    Create(SendCreateCommand),

    /// Edit existing Send
    Edit(SendEditCommand),

    /// Remove password from Send
    #[command(name = "remove-password")]
    RemovePassword(SendRemovePasswordCommand),

    /// Delete Send
    Delete(SendDeleteCommand),
}

/// `send template` had C27's defect too, undetected: the old match said
/// `Send(_) => true`, so a static template demanded a session. Same root cause,
/// same fix — the requirement is declared per subcommand.
impl Requires for SendCommands {
    fn requires(&self) -> Need {
        match self {
            SendCommands::Template(_) => Need::Nothing,
            _ => Need::UnlockedVault,
        }
    }
}

#[derive(Args)]
pub struct SendListCommand;

#[derive(Args)]
pub struct SendTemplateCommand {
    /// Template type (file or text)
    #[arg(value_name = "TYPE")]
    pub send_type: Option<String>,
}

#[derive(Args)]
pub struct SendGetCommand {
    /// Send ID
    #[arg(value_name = "ID")]
    pub id: String,
}

#[derive(Args)]
pub struct SendCreateCommand {
    /// JSON encoded Send
    #[arg(value_name = "JSON")]
    pub json: String,

    /// File path (for file sends)
    #[arg(long)]
    pub file: Option<String>,

    /// Send text content
    #[arg(long)]
    pub text: Option<String>,

    /// Hidden text (for text sends)
    #[arg(long)]
    pub hidden: bool,
}

#[derive(Args)]
pub struct SendEditCommand {
    /// Send ID
    #[arg(value_name = "ID")]
    pub id: String,

    /// JSON encoded Send
    #[arg(value_name = "JSON")]
    pub json: String,
}

#[derive(Args)]
pub struct SendRemovePasswordCommand {
    /// Send ID
    #[arg(value_name = "ID")]
    pub id: String,
}

#[derive(Args)]
pub struct SendDeleteCommand {
    /// Send ID
    #[arg(value_name = "ID")]
    pub id: String,
}

/// User-supplied shape of a Send.
///
/// Deliberately not `SendView`: that carries server-owned fields (`id`,
/// `accessId`, `key`, `accessCount`, `revisionDate`) and typed dates, none of
/// which a user should have to supply. This mirrors the `bw send template`
/// output instead.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct SendJsonInput {
    name: String,
    notes: Option<String>,
    text: Option<SendTextInput>,
    file: Option<SendFileInput>,
    deletion_date: Option<DateTime<Utc>>,
    expiration_date: Option<DateTime<Utc>>,
    max_access_count: Option<u32>,
    password: Option<String>,
    disabled: bool,
    hide_email: bool,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct SendTextInput {
    text: Option<String>,
    hidden: bool,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct SendFileInput {
    file_name: Option<String>,
}

/// The TypeScript CLI defaults a Send to expiring in 7 days.
fn default_deletion_date() -> DateTime<Utc> {
    Utc::now() + Duration::days(7)
}

fn parse_send_input(json: &str) -> anyhow::Result<SendJsonInput> {
    let json = get_json_string(json)?;
    let input: SendJsonInput = serde_json::from_str(&json)
        .map_err(|e| anyhow::anyhow!("Invalid Send JSON: {e}"))?;
    Ok(input)
}

fn send_response(global_args: &GlobalArgs, view: &SendView) -> CommandResult {
    let value = serde_json::to_value(view)?;
    if global_args.response {
        Ok(CommandOutput::success(value))
    } else {
        Ok(CommandOutput::success(value))
    }
}

pub async fn execute_send(
    cmd: SendCommands,
    global_args: &GlobalArgs,
    ctx: &AppContext,
    unlocked: Option<&Unlocked<'_>>,
) -> CommandResult {
    use SendCommands::*;

    match cmd {
        Template(cmd) => execute_send_template(cmd),
        List(_) => {
            require(unlocked)?;
            let sends = ctx.sdk().sends().list().await?;
            Ok(CommandOutput::success(serde_json::to_value(sends)?))
        }
        Get(cmd) => {
            require(unlocked)?;
            let id = SendId::from_str(&cmd.id)
                .map_err(|_| anyhow::anyhow!("'{}' is not a valid Send id", cmd.id))?;
            let view = ctx.sdk().sends().get(id).await?;
            send_response(global_args, &view)
        }
        Create(cmd) => execute_send_create(cmd, global_args, ctx).await,
        Edit(cmd) => execute_send_edit(cmd, global_args, ctx).await,
        RemovePassword(cmd) => {
            require(unlocked)?;
            let id = SendId::from_str(&cmd.id)
                .map_err(|_| anyhow::anyhow!("'{}' is not a valid Send id", cmd.id))?;
            let view = ctx.sdk().sends().remove_password(id).await?;
            send_response(global_args, &view)
        }
        Delete(cmd) => {
            require(unlocked)?;
            let id = SendId::from_str(&cmd.id)
                .map_err(|_| anyhow::anyhow!("'{}' is not a valid Send id", cmd.id))?;
            ctx.sdk().sends().delete(id).await?;
            Ok(CommandOutput::success_raw("Send deleted."))
        }
    }
}

async fn execute_send_create(
    cmd: SendCreateCommand,
    global_args: &GlobalArgs,
    ctx: &AppContext,
) -> CommandResult {

    let mut input = parse_send_input(&cmd.json)?;

    // Flags win over the JSON body, matching the TypeScript CLI.
    if let Some(text) = cmd.text {
        input.text = Some(SendTextInput {
            text: Some(text),
            hidden: cmd.hidden,
        });
    } else if cmd.hidden {
        if let Some(t) = input.text.as_mut() {
            t.hidden = true;
        }
    }

    if cmd.file.is_some() || input.file.is_some() {
        anyhow::bail!(
            "File Sends are not supported yet; only text Sends can be created. \
             See docs/sdk-3.0-migration.md."
        );
    }

    let text = input
        .text
        .as_ref()
        .and_then(|t| t.text.clone())
        .ok_or_else(|| anyhow::anyhow!("A text Send needs text content (--text or JSON 'text')"))?;

    if input.name.trim().is_empty() {
        anyhow::bail!("A Send needs a name");
    }

    let request = SendAddRequest {
        name: input.name.clone(),
        notes: input.notes.clone(),
        view_type: SendViewType::Text(SendTextView {
            text: Some(text),
            hidden: input.text.as_ref().map(|t| t.hidden).unwrap_or(false),
        }),
        max_access_count: input.max_access_count,
        disabled: input.disabled,
        hide_email: input.hide_email,
        deletion_date: input.deletion_date.unwrap_or_else(default_deletion_date),
        expiration_date: input.expiration_date,
        auth: match input.password.clone() {
            Some(password) if !password.is_empty() => SendAuthType::Password { password },
            _ => SendAuthType::None,
        },
    };

    let view = ctx.sdk().sends().create(request).await?;
    send_response(global_args, &view)
}

async fn execute_send_edit(
    cmd: SendEditCommand,
    global_args: &GlobalArgs,
    ctx: &AppContext,
) -> CommandResult {

    let id = SendId::from_str(&cmd.id)
        .map_err(|_| anyhow::anyhow!("'{}' is not a valid Send id", cmd.id))?;

    let input = parse_send_input(&cmd.json)?;
    let existing = ctx.sdk().sends().get(id).await?;

    if input.file.is_some() {
        anyhow::bail!("File Sends are not supported yet; only text Sends can be edited.");
    }

    // Fall back to the existing values so a partial edit does not blank fields.
    let text = match input.text.as_ref() {
        Some(t) => SendTextView {
            text: t.text.clone(),
            hidden: t.hidden,
        },
        None => existing.text.clone().ok_or_else(|| {
            anyhow::anyhow!("File Sends are not supported yet; only text Sends can be edited.")
        })?,
    };

    let request = SendEditRequest {
        name: if input.name.trim().is_empty() {
            existing.name.clone()
        } else {
            input.name.clone()
        },
        notes: input.notes.clone().or_else(|| existing.notes.clone()),
        view_type: SendViewType::Text(text),
        max_access_count: input.max_access_count.or(existing.max_access_count),
        disabled: input.disabled,
        hide_email: input.hide_email,
        deletion_date: input.deletion_date.unwrap_or(existing.deletion_date),
        expiration_date: input.expiration_date.or(existing.expiration_date),
        // Only touch auth when a password was supplied; otherwise keep it.
        auth: match input.password.clone() {
            Some(password) if !password.is_empty() => AuthEdit::Set {
                auth: SendAuthType::Password { password },
            },
            _ => AuthEdit::Preserve,
        },
    };

    let view = ctx.sdk().sends().edit(id, request).await?;
    send_response(global_args, &view)
}

fn execute_send_template(cmd: SendTemplateCommand) -> CommandResult {
    let send_type = cmd.send_type.as_deref().unwrap_or("text").to_lowercase();

    let template = match send_type.as_str() {
        "text" => serde_json::json!({
            "type": 0,
            "name": "My Text Send",
            "notes": "",
            "text": {
                "text": "Content to share",
                "hidden": false
            },
            "deletionDate": null,
            "expirationDate": null,
            "maxAccessCount": null,
            "password": null,
            "disabled": false,
            "hideEmail": false
        }),
        "file" => serde_json::json!({
            "type": 1,
            "name": "My File Send",
            "notes": "",
            "file": {
                "fileName": "example.txt",
                "size": 0,
                "sizeName": "0 bytes"
            },
            "deletionDate": null,
            "expirationDate": null,
            "maxAccessCount": null,
            "password": null,
            "disabled": false,
            "hideEmail": false
        }),
        _ => {
            return Err(anyhow::Error::msg(format!(
                "Invalid send type: {}. Must be 'text' or 'file'",
                send_type
            )));
        }
    };

    Ok(CommandOutput::success(template))
}
