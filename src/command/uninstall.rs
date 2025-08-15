use clap::Args;

use crate::command::McdlCommand;

#[derive(Debug, Args)]
pub struct UninstallCmd {
    name: String,
}

impl McdlCommand for UninstallCmd {
    async fn execute(&self) -> anyhow::Result<()> {
        todo!()
    }
}
