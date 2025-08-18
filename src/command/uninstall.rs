use clap::Args;

use crate::command::McdlCommand;

#[derive(Debug, Args)]
pub struct UninstallCmd {
    /// Name of the server instance to uninstall
    name: String,
}

impl McdlCommand for UninstallCmd {
    async fn execute(&self) -> anyhow::Result<()> {
        todo!()
    }
}
