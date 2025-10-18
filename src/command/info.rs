use clap::Args;

use crate::command::McdlCommand;
use crate::metadata::api::models::minecraft::VersionId;

#[derive(Debug, Args)]
pub struct InfoCmd {
    /// The version to show information about
    version: VersionId,
}

impl McdlCommand for InfoCmd {
    #[tracing::instrument]
    async fn execute(&self) -> color_eyre::Result<()> {
        todo!()
    }
}
