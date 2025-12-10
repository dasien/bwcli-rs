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

    Restore(commands::RestoreCommand),
    Move(commands::MoveCommand),
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
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .init();

    let cli = Cli::parse();

    // Initialize application context (services) once
    let ctx = match AppContext::new() {
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

    // Execute command and format output
    let result = execute_command(cli.command, &cli.global_args, &ctx).await;

    let exit_code = match result {
        Ok(response) => {
            output::print_response(response, &cli.global_args);
            ExitCode::SUCCESS
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
