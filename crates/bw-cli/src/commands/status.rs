use crate::GlobalArgs;
use crate::output::Response;
use clap::Args;

#[derive(Args)]
pub struct StatusCommand;

pub async fn execute_status(
    _cmd: StatusCommand,
    _global_args: &GlobalArgs,
) -> anyhow::Result<Response> {
    Ok(Response::error("Not yet implemented"))
}
