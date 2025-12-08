use crate::GlobalArgs;
use crate::output::Response;
use clap::{Args, Subcommand};

#[derive(Args)]
pub struct ConfigCommand {
    #[command(subcommand)]
    pub subcommand: ConfigSubcommand,
}

#[derive(Subcommand)]
pub enum ConfigSubcommand {
    /// Set server URL
    Server(ConfigServerCommand),
}

#[derive(Args)]
pub struct ConfigServerCommand {
    /// Server URL
    #[arg(value_name = "URL")]
    pub url: String,
}

pub async fn execute_config(
    _cmd: ConfigCommand,
    _global_args: &GlobalArgs,
) -> anyhow::Result<Response> {
    Ok(Response::error("Not yet implemented"))
}
