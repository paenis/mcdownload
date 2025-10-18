use clap::Args;

use crate::command::McdlCommand;
use crate::metadata::ServerSpec;

/*
`install` command should have some way of specifying version, name, and server type (e.g. fabric, forge, paper), for example:
mcdl install -v 1.20.1 -(i|n) <name> -s <server type>

preferably it will also support installing multiple versions at once:
mcdl install -v 1.20.1 -n foo -s fabric -v 1.19.4 -n bar -s forge

this type of positional argument grouping is not easy to implement with clap's current API, so it might require delimiting the arguments:
mcdl install -v 1.20.1:<name>:<server type> [-v ...]
*/

#[derive(Debug, Args)]
pub struct InstallCmd {
    /// Specifications of the server instances to install
    ///
    /// Each item should be formatted as [<version>][:[<name>][:[<server type>]]].
    /// If any part is omitted, it will use default values (i.e. latest version, "unnamed", vanilla server).
    /// For example:
    ///
    /// `1.20.1` will install a 1.20.1 vanilla server, called "unnamed",
    ///
    /// `1.19.4:my-server:fabric` will install a 1.19.4 Fabric server with the name "my-server",
    ///
    /// `::forge` will install the latest Forge server, called "unnamed".
    #[clap(num_args = 1..)]
    specs: Option<Vec<ServerSpec>>,
}

impl McdlCommand for InstallCmd {
    #[tracing::instrument]
    async fn execute(&self) -> color_eyre::Result<()> {
        // todo!()
        Ok(())
    }
}
