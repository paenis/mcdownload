//! A tool for managing Minecraft server versions

#![warn(clippy::all)]
#![warn(rustdoc::all)]

pub(crate) mod app;
pub(crate) mod common;
pub(crate) mod types;
pub(crate) mod utils;

use async_once::AsyncOnce;
use clap::error::ErrorKind;
use clap::{arg, command, value_parser, Args, CommandFactory, Parser, Subcommand, ValueEnum};
use color_eyre::eyre::{eyre, Result, WrapErr};
use is_terminal::IsTerminal;
use itertools::Itertools;
use lazy_static::lazy_static;
use prettytable::format::FormatBuilder;
use prettytable::{row, Cell, Row, Table};

use crate::common::MCDL_VERSION;
use crate::types::version::{GameVersionList, VersionNumber};
use crate::utils::net::get_version_manifest;

lazy_static! {
    static ref MANIFEST: AsyncOnce<GameVersionList> = AsyncOnce::new(async {
        get_version_manifest()
            .await
            .expect("Failed to get version manifest")
    });
}

#[doc(hidden)]
#[derive(Parser, Debug)]
#[command(author, version = MCDL_VERSION.as_str())]
#[command(arg_required_else_help = true, subcommand_required = true)]
/// A tool for managing Minecraft server versions
struct Cli {
    #[command(subcommand)]
    action: Action,
}

#[doc(hidden)]
#[derive(Subcommand, Debug)]
enum Action {
    /// List available Minecraft versions
    List {
        #[command(flatten)]
        filter: Option<ListFilter>,
        // #[arg(short, long)]
        // #[arg(rename = "kebab-case")]
        // /// List installed instances and their versions
        // installed: bool,
    },
    /// Get information about a Minecraft version
    Info {
        #[arg(required = true, value_parser = value_parser!(VersionNumber))]
        #[arg(short, long)]
        /// The Minecraft version to get information about
        version: VersionNumber,
    },
    /// Install a server instance
    Install {
        #[arg(value_delimiter = ',', num_args = 0.., value_parser = value_parser!(VersionNumber))]
        #[arg(short, long)]
        /// The version(s) to install
        ///
        /// Defaults to latest release version if none is provided.
        /// Can be specified multiple times, or as a comma or space-separated list.
        version: Option<Vec<VersionNumber>>,
        // #[arg(short, long)]
        // name: Option<String>,
    },
    /// Run a server instance
    Run {
        #[arg(required = true)]
        #[arg(short, long)]
        /// The version to run
        version: String, // in the future, `name` will be used instead
    },
    /// Print the path to a config file or instance directory
    Locate {
        #[arg(required = true)]
        #[arg(value_enum)]
        /// The file or directory to locate
        what: WhatEnum,
    },
    // /// Uninstall a server instance
    // Uninstall {
    //     #[arg(short, long)]
    //     version: String, // in the future, `name` will be used instead
    // },
}

#[doc(hidden)]
#[derive(Args, Debug)]
#[group(id = "filter", required = false, multiple = false)]
struct ListFilter {
    #[arg(short, long)]
    /// Only list release versions (default)
    release: bool,
    #[arg(short, long)]
    /// Only list pre-release versions
    pre_release: bool,
    #[arg(short, long)]
    /// Only list snapshot versions
    snapshot: bool,
    #[arg(short, long)]
    /// Only list other versions
    other: bool,
    #[arg(short, long)]
    /// List all versions
    all: bool,
}

impl Default for ListFilter {
    fn default() -> Self {
        Self {
            release: true,
            pre_release: false,
            snapshot: false,
            other: false,
            all: false,
        }
    }
}

#[doc(hidden)]
#[derive(Clone, Copy, ValueEnum, Debug)]
enum WhatEnum {
    /// The Java Runtime Environment directory
    Java,
    /// The directory containing Minecraft server instances
    Instance,
    /// The directory containing configuration files
    Config,
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();

    match cli.action {
        Action::List { filter } => list_impl(filter).await?,
        Action::Info { version } => info_impl(version).await?,
        Action::Install { version } => install_impl(version).await?,
        Action::Run { version } => run_impl(version).await?,
        Action::Locate { what } => locate_impl(what)?,
        // Action::Uninstall { version } => uninstall_impl(version).await?,
    }

    Ok(())
}

async fn list_impl(filter: Option<ListFilter>) -> Result<()> {
    let filter = filter.unwrap_or_default();

    let versions = MANIFEST
        .get()
        .await
        .versions
        .iter()
        .filter(|v| {
            match (
                filter.release,
                filter.pre_release,
                filter.snapshot,
                filter.other,
                filter.all,
            ) {
                (true, _, _, _, _) => v.id.is_release(),
                (_, true, _, _, _) => v.id.is_pre_release(),
                (_, _, true, _, _) => v.id.is_snapshot(),
                (_, _, _, true, _) => v.id.is_other(),
                (_, _, _, _, true) => true,
                _ => unreachable!(),
            }
        })
        .sorted()
        .collect_vec();

    if !std::io::stdout().is_terminal() {
        versions.iter().for_each(|v| println!("{}", v.id));
        return Ok(());
    }

    let mut table = Table::new();
    table.set_format(
        FormatBuilder::new()
            .column_separator(' ')
            .borders(' ')
            .padding(1, 1)
            .build(),
    );
    table.set_titles(row![b => "Version", "Type", "Release Date"]);
    for version in versions {
        table.add_row(Row::new(vec![
            Cell::new(&version.id.to_string()),
            Cell::new(&version.release_type.to_string()).style_spec(
                match version.release_type.as_str() {
                    "release" => "Fgb",
                    _ => "",
                },
            ),
            Cell::new(&version.release_time.to_string()),
        ]));
    }
    table.printstd();

    Ok(())
}

async fn info_impl(version: VersionNumber) -> Result<()> {
    let game_version = MANIFEST
        .get()
        .await
        .versions
        .iter()
        .find(|v| v.id == version);

    if let Some(version) = game_version {
        let time_format = "%-d %B %Y at %-I:%M:%S%P UTC";
        let message = format!(
            "Version {} ({})\nReleased: {}\nLast updated: {}",
            version.id,
            version.release_type,
            version.release_time.format(time_format),
            version.time.format(time_format),
        );

        println!("{message}");
    } else {
        Cli::command()
            .error(
                ErrorKind::ValueValidation,
                format!("No such version: {version}"),
            )
            .exit();
    }

    Ok(())
}

async fn install_impl(version: Option<Vec<VersionNumber>>) -> Result<()> {
    let manifest = MANIFEST.get().await;
    let versions = &manifest.versions;
    let version_ids = versions.iter().map(|v| &v.id).collect_vec();
    let latest = &manifest.latest;

    if version.is_none() {
        println!("Installing latest release version\n");
        let latest = versions
            .iter()
            .find(|v| v.id == latest.release)
            .ok_or_else(|| eyre!("No latest release version found"))?;
        app::install_versions(vec![latest])
            .await
            .wrap_err("Error while installing latest version")?;

        return Ok(());
    }

    let version = version.unwrap();
    if version.is_empty() {
        Cli::command()
            .error(ErrorKind::ValueValidation, "No version provided")
            .exit();
    }

    let (valid, invalid): (Vec<_>, Vec<_>) = version.iter().partition(|v| version_ids.contains(v));

    if valid.is_empty() {
        Cli::command()
            .error(
                ErrorKind::ValueValidation,
                format!("No valid versions found (got {})", invalid.len()),
            )
            .exit();
    }

    let mut message = format!(
        "Installing {} version{}: {}",
        valid.len(),
        if valid.len() == 1 { "" } else { "s" },
        valid.iter().map(ToString::to_string).join(", ")
    );

    if !invalid.is_empty() {
        message.push_str(&format!(
            " (skipped {} invalid version{}: {})",
            invalid.len(),
            if invalid.len() == 1 { "" } else { "s" },
            invalid.iter().map(ToString::to_string).join(", ")
        ));
    }

    println!("{message}\n");

    let to_install_versions = versions
        .iter()
        .filter(|v| valid.contains(&&v.id))
        .collect_vec();
    app::install_versions(to_install_versions)
        .await
        .wrap_err("Error while installing versions")?;

    Ok(())
}

async fn run_impl(version: String) -> Result<()> {
    app::run_instance(version.parse()?)
        .await
        .wrap_err("Error while running server")?;

    Ok(())
}

fn locate_impl(what: WhatEnum) -> Result<()> {
    // TODO: pass directly
    let what = match what {
        WhatEnum::Java => "java",
        WhatEnum::Instance => "instance",
        WhatEnum::Config => "config",
    }
    .to_string();

    app::locate(&what).wrap_err(format!("Error while locating `{what}`"))?;

    Ok(())
}

async fn uninstall_impl(version: impl Sized) -> Result<()> {
    todo!()
}
