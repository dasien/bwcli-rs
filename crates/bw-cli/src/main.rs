use crate::auth_gate::{Need, Requires, Unlocked, require};
use clap::{Parser, Subcommand};
use std::process::ExitCode;

/// Bitwarden CLI - A secure and free password manager for all of your devices
#[derive(Parser)]
#[command(
    name = "bw",
    version,
    about = "Bitwarden CLI",
    long_about = "A secure and free password manager for all of your devices.\n\n\
                  Documentation: https://bitwarden.com/help/cli",
    after_help = "Use 'bw <command> --help' for more information about a command."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[command(flatten)]
    global_args: GlobalArgs,
}

/// Global flags available on all commands
#[derive(Parser, Debug, Clone)]
pub struct GlobalArgs {
    /// Session key for authentication
    #[arg(long, env = "BW_SESSION", global = true, hide_env_values = true)]
    pub session: Option<String>,

    /// Suppress all output
    #[arg(long, env = "BW_QUIET", global = true)]
    pub quiet: bool,

    /// Return raw JSON response
    #[arg(long, env = "BW_RESPONSE", global = true)]
    pub response: bool,

    /// Return raw output (no formatting)
    #[arg(long, env = "BW_RAW", global = true)]
    pub raw: bool,

    /// Pretty-print JSON output
    #[arg(long, env = "BW_PRETTY", global = true)]
    pub pretty: bool,

    /// Do not prompt for interactive input
    #[arg(long, env = "BW_NOINTERACTION", global = true)]
    pub nointeraction: bool,

    /// Always exit with code 0 (success)
    #[arg(long, env = "BW_CLEANEXIT", global = true)]
    pub cleanexit: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Authentication commands
    #[command(subcommand)]
    Login(commands::AuthCommands),

    Logout(commands::LogoutCommand),
    Lock(commands::LockCommand),
    Unlock(commands::UnlockCommand),

    /// Vault management commands
    #[command(subcommand)]
    List(commands::ListCommands),

    #[command(subcommand)]
    Get(commands::GetCommands),

    #[command(subcommand)]
    Create(commands::CreateCommands),

    #[command(subcommand)]
    Edit(commands::EditCommands),

    #[command(subcommand)]
    Delete(commands::DeleteCommands),

    #[command(subcommand)]
    Restore(commands::RestoreCommands),
    /// Move an item to an organization
    ///
    /// `share` is accepted as an alias, and is deprecated in the TypeScript CLI
    /// for the same reason: `move` is the current name.
    #[command(alias = "share")]
    Move(commands::MoveCommand),
    /// Move an item to a folder (not a TypeScript CLI command)
    #[command(name = "move-to-folder")]
    MoveToFolder(commands::MoveToFolderCommand),
    Confirm(commands::ConfirmCommand),

    /// Sync vault with server
    Sync(commands::SyncCommand),

    /// Utility commands
    Generate(commands::GenerateCommand),
    Encode(commands::EncodeCommand),
    Decrypt(commands::DecryptCommand),
    Import(commands::ImportCommand),
    Export(commands::ExportCommand),

    /// Send commands
    #[command(subcommand)]
    Send(commands::SendCommands),

    /// Receive and decrypt a Send
    Receive(commands::ReceiveCommand),

    /// Configuration
    Config(commands::ConfigCommand),

    /// Status
    Status(commands::StatusCommand),
}

/// Render a startup failure and pick the exit code.
///
/// The pre-flight checks below run before any command does, but their failures
/// are still command failures: they must honour `--quiet`, must produce the
/// `--response` JSON document, and must respect `--cleanexit`. They used to
/// `eprintln!` directly and return, so `bw <cmd> --response` printed *nothing*
/// when the vault was locked — the machine-readable mode silently emitted no
/// answer at all. Routing them through the same renderer as every other failure
/// is the point of having one.
fn fail(error: &anyhow::Error, args: &GlobalArgs) -> ExitCode {
    output::print_error(error, args);
    if args.cleanexit {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    // Initialize tracing
    tracing_subscriber::fmt()
        // Diagnostics must go to stderr: stdout carries the command's actual
        // output, and `--response`/`--raw` consumers pipe it into jq. A single
        // SDK log line on stdout makes that output unparseable.
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .init();

    let cli = Cli::parse();

    // Initialize application context (services) once
    let ctx = match AppContext::new().await {
        Ok(ctx) => ctx,
        Err(e) => return fail(&e.context("Failed to initialize"), &cli.global_args),
    };

    // Meet whatever the command declared it needs, once, before it runs. A
    // command that needs an unlocked vault is then *handed* the proof; it cannot
    // forget to check, and cannot check again.
    let unlocked = match auth_gate::satisfy(cli.command.requires(), &cli.global_args, &ctx).await {
        Ok(unlocked) => unlocked,
        Err(e) => return fail(&e, &cli.global_args),
    };

    // Execute command and format output
    let result = execute_command(cli.command, &cli.global_args, &ctx, unlocked.as_ref()).await;

    // `Ok` is success and `Err` is failure, with no third case to get wrong.
    // C31 was the absence of that guarantee: handlers returned an error *inside*
    // `Ok`, so the exit code and the printed message could disagree.
    match result {
        Ok(response) => {
            output::print_output(response, &cli.global_args);
            ExitCode::SUCCESS
        }
        Err(e) => fail(&e, &cli.global_args),
    }
}

/// Every command's requirement, in one place — but *delegated* where a command's
/// subcommands differ.
///
/// This replaced `needs_unlocked_vault(&Commands) -> bool`, which answered per
/// top-level command and therefore could not say "`get` needs an unlocked vault
/// except for `get template`". That flattening *was* `BUGLIST.md` C27, and it
/// silently affected `send template` too.
///
/// The catch-all arms are deliberate in the other direction: a new command
/// defaults to needing an unlocked vault, so the unsafe default is the
/// restrictive one. `Requires` for `GetCommands` and `SendCommands` lives beside
/// those enums.
impl Requires for Commands {
    fn requires(&self) -> Need {
        use Commands::*;

        match self {
            // Delegated: these enums have subcommands that need nothing.
            Get(cmd) => cmd.requires(),
            Send(cmd) => cmd.requires(),

            // No account or session at all. `receive` is anonymous by design —
            // receiving a Send must work while locked, or logged out entirely.
            Login(_) | Logout(_) | Lock(_) | Unlock(_) | Status(_) | Config(_) | Generate(_)
            | Encode(_) | Decrypt(_) | Receive(_) => Need::Nothing,

            // Reads or writes vault data, so the key store must be loaded.
            List(_) | Create(_) | Edit(_) | Delete(_) | Restore(_) | Move(_) | MoveToFolder(_)
            | Sync(_) | Import(_) | Export(_) | Confirm(_) => Need::UnlockedVault,
        }
    }
}

async fn execute_command(
    command: Commands,
    global_args: &GlobalArgs,
    ctx: &AppContext,
    unlocked: Option<&Unlocked<'_>>,
) -> output::CommandResult {
    use Commands::*;

    match command {
        Login(cmd) => commands::execute_login(cmd, global_args, ctx).await,
        Logout(cmd) => commands::execute_logout(cmd, global_args, ctx).await,
        Lock(cmd) => commands::execute_lock(cmd, global_args, ctx).await,
        Unlock(cmd) => commands::execute_unlock(cmd, global_args, ctx).await,
        List(cmd) => commands::execute_list(cmd, global_args, ctx, require(unlocked)?).await,
        // Mixed: `get template` needs nothing, the rest need an unlocked vault.
        Get(cmd) => commands::execute_get(cmd, global_args, ctx, unlocked).await,
        Create(cmd) => commands::execute_create(cmd, global_args, ctx, require(unlocked)?).await,
        Edit(cmd) => commands::execute_edit(cmd, global_args, ctx, require(unlocked)?).await,
        Delete(cmd) => commands::execute_delete(cmd, global_args, ctx, require(unlocked)?).await,
        Restore(cmd) => commands::execute_restore(cmd, global_args, ctx, require(unlocked)?).await,
        Move(cmd) => commands::execute_move(cmd, global_args, ctx, require(unlocked)?).await,
        MoveToFolder(cmd) => commands::execute_move_to_folder(cmd, global_args, ctx, require(unlocked)?).await,
        Confirm(cmd) => commands::execute_confirm(cmd, global_args, ctx, require(unlocked)?).await,
        Sync(cmd) => commands::execute_sync(cmd, global_args, ctx, require(unlocked)?).await,
        Generate(cmd) => commands::execute_generate(cmd, global_args, ctx).await,
        Encode(cmd) => commands::execute_encode(cmd, global_args, ctx).await,
        Decrypt(cmd) => commands::execute_decrypt(cmd, global_args, ctx).await,
        Import(cmd) => commands::execute_import(cmd, global_args, ctx, require(unlocked)?).await,
        Export(cmd) => commands::execute_export(cmd, global_args, ctx, require(unlocked)?).await,
        // Mixed: `send template` needs nothing.
        Send(cmd) => commands::execute_send(cmd, global_args, ctx, unlocked).await,
        Receive(cmd) => commands::execute_receive(cmd, global_args, ctx).await,
        Config(cmd) => commands::execute_config(cmd, global_args, ctx).await,
        Status(cmd) => commands::execute_status(cmd, global_args, ctx).await,
    }
}

// Module declarations
mod commands;
mod auth_gate;
mod context;
mod output;

pub use context::AppContext;
