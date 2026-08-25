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
        Err(e) => {
            if !cli.global_args.quiet {
                eprintln!("Failed to initialize: {:#}", e);
            }
            return if cli.global_args.cleanexit {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            };
        }
    };

    // The SDK client starts each process with an empty key store, so a session
    // key has to be loaded before any command performs vault crypto.
    //
    // A failure here must be fatal for commands that need crypto. It used to be
    // logged at debug and ignored, which meant a stale session produced 11
    // items with empty names instead of an error — decryption against an empty
    // key store yields blanks rather than failing.
    if needs_unlocked_vault(&cli.command) {
        match cli.global_args.session.as_deref() {
            Some(session) if !session.is_empty() => {
                if let Err(e) = ctx.container().unlock_sdk(session).await {
                    if !cli.global_args.quiet {
                        eprintln!("Error: {:#}", e);
                        eprintln!("Run 'bw unlock' to get a new session key.");
                    }
                    return if cli.global_args.cleanexit {
                        ExitCode::SUCCESS
                    } else {
                        ExitCode::FAILURE
                    };
                }
            }
            _ => {
                if !cli.global_args.quiet {
                    eprintln!(
                        "Error: Vault is locked. Run 'bw unlock' and set BW_SESSION."
                    );
                }
                return if cli.global_args.cleanexit {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::FAILURE
                };
            }
        }
    }

    // Execute command and format output
    let result = execute_command(cli.command, &cli.global_args, &ctx).await;

    let exit_code = match result {
        Ok(response) => {
            // A command reports a *handled* failure by returning `Ok` around an
            // error `Response` — only unhandled errors reach the `Err` arm. So
            // the response has to be consulted too, or `bw get item nope` exits
            // 0 and every `bw ... && ...` script treats the failure as success.
            let failed = !response.is_success();
            output::print_response(response, &cli.global_args);
            if failed && !cli.global_args.cleanexit {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(e) => {
            if !cli.global_args.quiet {
                eprintln!("Error: {:#}", e);
            }
            if cli.global_args.cleanexit {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
    };

    exit_code
}

/// Whether a command reads or writes vault data and therefore needs the key
/// store populated.
///
/// Listed explicitly rather than inferred so that adding a command forces a
/// deliberate choice. `receive` is absent on purpose: receiving a Send is
/// anonymous and must work while locked, or logged out entirely.
fn needs_unlocked_vault(command: &Commands) -> bool {
    use Commands::*;

    match command {
        List(_) | Get(_) | Create(_) | Edit(_) | Delete(_) | Restore(_) | Move(_) | MoveToFolder(_) | Sync(_)
        | Import(_) | Export(_) | Send(_) | Confirm(_) => true,

        Login(_) | Logout(_) | Lock(_) | Unlock(_) | Status(_) | Config(_) | Generate(_)
        | Encode(_) | Decrypt(_) | Receive(_) => false,
    }
}

async fn execute_command(
    command: Commands,
    global_args: &GlobalArgs,
    ctx: &AppContext,
) -> anyhow::Result<output::Response> {
    use Commands::*;

    match command {
        Login(cmd) => commands::execute_login(cmd, global_args, ctx).await,
        Logout(cmd) => commands::execute_logout(cmd, global_args, ctx).await,
        Lock(cmd) => commands::execute_lock(cmd, global_args, ctx).await,
        Unlock(cmd) => commands::execute_unlock(cmd, global_args, ctx).await,
        List(cmd) => commands::execute_list(cmd, global_args, ctx).await,
        Get(cmd) => commands::execute_get(cmd, global_args, ctx).await,
        Create(cmd) => commands::execute_create(cmd, global_args, ctx).await,
        Edit(cmd) => commands::execute_edit(cmd, global_args, ctx).await,
        Delete(cmd) => commands::execute_delete(cmd, global_args, ctx).await,
        Restore(cmd) => commands::execute_restore(cmd, global_args, ctx).await,
        Move(cmd) => commands::execute_move(cmd, global_args, ctx).await,
        MoveToFolder(cmd) => commands::execute_move_to_folder(cmd, global_args, ctx).await,
        Confirm(cmd) => commands::execute_confirm(cmd, global_args, ctx).await,
        Sync(cmd) => commands::execute_sync(cmd, global_args, ctx).await,
        Generate(cmd) => commands::execute_generate(cmd, global_args, ctx).await,
        Encode(cmd) => commands::execute_encode(cmd, global_args, ctx).await,
        Decrypt(cmd) => commands::execute_decrypt(cmd, global_args, ctx).await,
        Import(cmd) => commands::execute_import(cmd, global_args, ctx).await,
        Export(cmd) => commands::execute_export(cmd, global_args, ctx).await,
        Send(cmd) => commands::execute_send(cmd, global_args, ctx).await,
        Receive(cmd) => commands::execute_receive(cmd, global_args, ctx).await,
        Config(cmd) => commands::execute_config(cmd, global_args, ctx).await,
        Status(cmd) => commands::execute_status(cmd, global_args, ctx).await,
    }
}

// Module declarations
mod commands;
mod context;
mod error;
mod output;

pub use context::AppContext;
pub use error::CliError;
