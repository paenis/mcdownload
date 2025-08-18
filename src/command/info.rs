use clap::Args;

use crate::command::McdlCommand;
use crate::metadata::api::models::minecraft::VersionId;

#[derive(Debug, Args)]
pub struct InfoCmd {
    /// The version to show information about
    #[clap(value_parser = empty)]
    version: VersionId,
}

fn empty(_: &str) -> Result<VersionId, String> {
    Ok(VersionId::empty())
}

impl McdlCommand for InfoCmd {
    async fn execute(&self) -> anyhow::Result<()> {
        todo!()
    }
}
