use crate::GlobalArgs;
use crate::output::Response;
use clap::Args;

#[derive(Args)]
pub struct ReceiveCommand {
    /// Send URL or access ID
    #[arg(value_name = "URL")]
    pub url: String,

    /// Password for password-protected Send
    #[arg(long)]
    pub password: Option<String>,
}

pub async fn execute_receive(
    _cmd: ReceiveCommand,
    _global_args: &GlobalArgs,
) -> anyhow::Result<Response> {
    Ok(Response::error(
        "Receive command not yet implemented. Requires: Send API integration, URL parsing, encryption service",
    ))
}
