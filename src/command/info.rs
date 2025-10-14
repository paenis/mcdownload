use clap::Args;

use crate::command::McdlCommand;
use crate::metadata::api::models::minecraft::VersionId;

#[derive(Debug, Args)]
pub struct InfoCmd {
    /// The version to show information about
    #[clap(value_parser = empty)]
    version: VersionId,
}

// FIXME: this is here to satisfy clap's need for a value parser. replace with actual implementation.
fn empty(_: &str) -> Result<VersionId, String> {
    Ok(VersionId::empty())
}

impl McdlCommand for InfoCmd {
    async fn execute(&self) -> anyhow::Result<()> {
        todo!()
    }
}
