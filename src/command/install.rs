use clap::Args;

use crate::command::McdlCommand;
use crate::metadata::api::models::minecraft::VersionId;

/*
`install` command should have some way of specifying version, name, and server type (e.g. fabric, forge, paper), for example:
mcdl install -v 1.20.1 -(i|n) <name> -s <server type>

preferably it will also support installing multiple versions at once:
mcdl install -v 1.20.1 -n foo -s fabric -v 1.19.4 -n bar -s forge

this type of positional argument grouping is not easy to implement with clap's current API, so it might require delimiting the arguments:
mcdl install -v 1.20.1:<name>:<server type> [-v ...]

this kinda sucks (what if i want to leave out the name?), so i might want to switch to `bpaf` instead of `clap`
*/

#[derive(Debug, Args)]
pub struct InstallCmd {
    /// Specifications of the server instances to install
    ///
    /// Each item should be formatted as [<version>][:[<name>][:[<server type>]]].
    /// If any part is omitted, it will use default values (i.e. latest version, random name, vanilla server).
    /// For example:
    ///
    /// `1.20.1` will install a vanilla server with a random name,
    ///
    /// `1.19.4:my-server:fabric` will install a Fabric server with the name "my-server",
    ///
    /// `::forge` will install the latest Forge server with a random name.
    #[clap(value_parser = empty, num_args = 1..)]
    specs: Option<Vec<ServerSpec>>,
}

fn empty(_: &str) -> Result<ServerSpec, String> {
    Ok(ServerSpec::empty())
}

// TODO: move
#[derive(Debug, Clone)]
struct ServerSpec {
    version: VersionId,
    name: String,
    server_type: String,
}

impl ServerSpec {
    fn empty() -> Self {
        Self {
            version: VersionId::empty(),
            name: "placeholder".into(),
            server_type: "vanilla".into(),
        }
    }
}

impl McdlCommand for InstallCmd {
    async fn execute(&self) -> anyhow::Result<()> {
        todo!()
    }
}
