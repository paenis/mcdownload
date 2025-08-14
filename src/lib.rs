//! Minecraft server manager

#![deny(rust_2018_idioms)]
#![warn(missing_docs, clippy::all)]

mod cli;
mod macros;
mod minecraft;
mod net;

use std::sync::LazyLock;

use anyhow::Result;
use clap::{ArgAction, Args, Parser, Subcommand, ValueEnum};

use crate::minecraft::VersionNumber;

static RT: LazyLock<tokio::runtime::Runtime> = LazyLock::new(|| {
    tracing::trace!("init tokio runtime");
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("failed to build tokio runtime")
});

#[derive(Debug, Parser)]
#[command(version, /* long_version = ..., */ about, max_term_width = 100)]
pub struct Mcdl {
    #[clap(flatten)]
    global: GlobalOpts,
    #[clap(subcommand)]
    command: Command,
}

impl Mcdl {
    /// Consume the command line arguments and execute the appropriate command.
    pub async fn run(self) -> Result<()> {
        dbg!(self);
        Ok(unimplemented!())
    }
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Show information about a Minecraft version.
    Info {
        /// The version to show information about
        #[clap(value_parser)]
        version: VersionNumber,
    },
    /// Install an instance of a Minecraft server.
    Install {
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
        #[clap(value_parser = parse_server_spec, num_args = 1..)]
        specs: Vec<ServerSpec>,
    },
    /// List installed or available Minecraft versions.
    List {
        /// Whether to show installed versions only
        #[arg(long, short = 'i')]
        installed_only: bool,
        #[command(flatten, next_help_heading = "Version filters")]
        filter: VersionTypeFilter,
    },
}

fn parse_server_spec(s: &str) -> Result<ServerSpec, std::convert::Infallible> {
    Ok(Default::default())
}

/*
`install` command should have some way of specifying version, name, and server type (e.g. fabric, forge, paper), for example:
mcdl install -v 1.20.1 -(i|n) <name> -s <server type>

preferably it will also support installing multiple versions at once:
mcdl install -v 1.20.1 -n foo -s fabric -v 1.19.4 -n bar -s forge

this type of positional argument grouping is not easy to implement with clap's current API, so it might require delimiting the arguments:
mcdl install -v 1.20.1:<name>:<server type> [-v ...]

this kinda sucks (what if i want to leave out the name?), so i might want to switch to `bpaf` instead of `clap`
*/

// TODO: move
#[derive(Debug, Clone, Default)]
struct ServerSpec {
    version: VersionNumber,
    name: String,
    server_type: String,
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

#[derive(Debug, Args)]
struct GlobalOpts {
    /// Color
    #[arg(long, value_enum, global = true, default_value_t)]
    color: Color,

    /// Verbosity level (can be set multiple times)
    #[arg(long, short, global = true, action = ArgAction::Count)]
    verbose: u8,
}

#[derive(Debug, Clone, Copy, Default, ValueEnum)]
enum Color {
    #[default]
    Auto,
    Always,
    Never,
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use clap::CommandFactory;

    use super::*;

    #[test]
    fn verify_app() {
        Mcdl::command().debug_assert();
    }

    #[test]
    fn test_parse_server_spec() {
        let spec = parse_server_spec("1.20.1:my-server:fabric").unwrap();
        assert_eq!(spec.version, VersionNumber::from_str("1.20.1").unwrap());
        assert_eq!(spec.name, "my-server");
        assert_eq!(spec.server_type, "fabric");
    }
}
