mod impls;
mod parse;

use crate::cli::parse::options;
use crate::minecraft::VersionNumber;

pub trait Execute {
    type Error;
    /// Process a command using this struct.
    fn execute(&self) -> Result<(), Self::Error>;
}

#[derive(Debug, Clone)]
pub(super) enum Options {
    /// Show version
    ShowVersion,
    /// Subcommand
    Cmd(Cmd),
}

impl Execute for Options {
    type Error = anyhow::Error;
    fn execute(&self) -> Result<(), Self::Error> {
        match self {
            Options::ShowVersion => eprintln!(concat!("mcdl ", env!("CARGO_PKG_VERSION"))),
            Options::Cmd(cmd) => cmd.execute()?,
        }
        Ok(())
    }
}

/// Filter the list of versions.
#[derive(Debug, Clone)]
struct ListFilter {
    /// If true, only include installed versions. This filter is _inclusive_.
    installed: bool,
    /// If the corresponding element is true, include release, pre-release, snapshot, and non-standard versions.
    ///
    /// At least one must be true. This filter is _exclusive_.
    included_types: (bool, bool, bool, bool),
}

impl ListFilter {
    fn includes(&self, version: &VersionNumber) -> bool {
        if self.installed {
            todo!("installed filter")
        } else {
            match version {
                VersionNumber::Release(_) => self.included_types.0,
                VersionNumber::PreRelease(_) => self.included_types.1,
                VersionNumber::Snapshot(_) => self.included_types.2,
                VersionNumber::NonStandard(_) => self.included_types.3,
            }
        }
    }
}

#[derive(Debug, Clone)]
enum Cmd {
    /// Install
    Install { versions: Vec<VersionNumber> },
    /// List installed or available versions
    List { filter: ListFilter },
    /// Print information about a version
    Info { v: VersionNumber },
}

impl Execute for Cmd {
    type Error = anyhow::Error;
    fn execute(&self) -> Result<(), Self::Error> {
        Ok(match self {
            Cmd::Install { versions } => impls::install(versions)?,
            Cmd::List { filter } => impls::list(filter)?,
            Cmd::Info { v } => impls::info(v)?,
        })
    }
}

pub fn parse() -> Options {
    options().run()
}
