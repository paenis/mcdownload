use clap::Args;

use crate::models::minecraft::VersionNumber;

#[derive(Debug, Args)]
pub struct InfoCmd {
    /// The version to show information about
    #[arg(value_parser)]
    version: VersionNumber,
}
