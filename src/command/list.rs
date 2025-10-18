use clap::Args;

use crate::command::McdlCommand;

#[derive(Debug, Args)]
pub struct ListCmd {
    /// Show the details of installed instances, instead of available versions
    #[arg(long, short = 'i')]
    show_installed: bool,
    #[command(flatten, next_help_heading = "Version Filters")]
    filter: VersionTypeFilter,
}

// TODO: change to api categories (release, snapshot, beta, alpha, [experiment])
#[derive(Debug, Clone, Args)]
struct VersionTypeFilter {
    /// Whether to include release versions
    #[arg(long, short = 'r')]
    show_release: bool,
    /// Whether to include pre-release versions
    #[arg(long, short = 'p')]
    show_pre_release: bool,
    /// Whether to include snapshot versions
    #[arg(long, short = 's')]
    show_snapshot: bool,
    /// Whether to include non-standard versions
    #[arg(long, short = 'n')]
    show_non_standard: bool,
}

impl Default for VersionTypeFilter {
    fn default() -> Self {
        Self {
            show_release: true,
            show_pre_release: false,
            show_snapshot: false,
            show_non_standard: false,
        }
    }
}

impl McdlCommand for ListCmd {
    #[tracing::instrument]
    async fn execute(&self) -> color_eyre::Result<()> {
        todo!()
    }
}
