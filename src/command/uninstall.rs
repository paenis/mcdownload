use clap::Args;

use crate::command::McdlCommand;

#[derive(Debug, Args)]
pub struct UninstallCmd {
    /// Name or ID of the server instance to uninstall
    specifier: String,
}

impl McdlCommand for UninstallCmd {
    #[tracing::instrument]
    async fn execute(&self) -> color_eyre::Result<()> {
        todo!()
    }
}
