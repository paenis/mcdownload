#![warn(clippy::all)]

//! A tool for managing Minecraft server versions

pub(crate) mod app;
pub(crate) mod types;
pub(crate) mod utils;

use crate::app::install_versions;
use crate::types::version::{GameVersion, VersionNumber};
use crate::utils::net::get_version_manifest;

use clap::{
    arg, command, crate_version, error::ErrorKind, value_parser, ArgAction, ArgGroup, Command,
};
use color_eyre::eyre::{eyre, Result, WrapErr};
use itertools::Itertools;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let mut cmd = command!()
        .about("A tool for managing Minecraft versions")
        .version(crate_version!())
        .arg_required_else_help(true)
        .subcommand(
            Command::new("list")
                .about("List all available Minecraft versions")
                .arg(
                    arg!(-r --release "Only list release versions (default)")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    arg!(-p --"pre-release" "Only list pre-release versions")
                        .action(ArgAction::SetTrue),
                )
                .arg(arg!(-s --snapshot "Only list snapshot versions").action(ArgAction::SetTrue))
                .arg(arg!(-o --other "Only list other versions").action(ArgAction::SetTrue))
                .arg(arg!(-a --all "List all versions").action(ArgAction::SetTrue))
                .group(ArgGroup::new("filter").args([
                    "release",
                    "pre-release",
                    "snapshot",
                    "other",
                    "all",
                ])),
        )
        .subcommand(
            Command::new("info")
                .about("Get information about a Minecraft version")
                .arg(
                    arg!(-v --version <VERSION> "The version to get information about")
                        .required(true)
                        .value_parser(value_parser!(String)),
                ),
        )
        .subcommand(
            Command::new("install")
                .about("Install a Minecraft version")
                .after_help("Defaults to latest release version")
                .arg(
                    arg!(-v --version "The version(s) to install")
                        .action(ArgAction::Append)
                        .value_delimiter(',') // splits as argv regardless
                        .num_args(0..)
                        .value_parser(value_parser!(String)),
                ),
        )
        .subcommand(
            Command::new("run")
                .about("Run a Minecraft version")
                .after_help("Must be installed first")
                .arg(
                    arg!(-v --version <VERSION> "The version to run")
                        .required(true)
                        .value_parser(value_parser!(String)), // parse as String here, validate later
                ),
        );
    // .subcommand(Command::new("uninstall").about("Uninstall a Minecraft version"))

    let matches = cmd.get_matches_mut();

    // shared manifest between subcommands
    let manifest_thread = tokio::spawn(async move {
        let manifest = get_version_manifest().await?;

        Ok::<_, color_eyre::eyre::Report>(manifest)
    });

    if let Some(matches) = matches.subcommand_matches("list") {
        if term_size::dimensions().is_none() {
            return Ok(());
        } // no terminal output, so don't bother

        let versions = manifest_thread.await??.versions;
        let versions_filtered: Vec<&GameVersion> = if matches.get_flag("release") {
            versions.iter().filter(|v| v.id.is_release()).collect_vec()
        } else if matches.get_flag("pre-release") {
            versions
                .iter()
                .filter(|v| v.id.is_pre_release())
                .collect_vec()
        } else if matches.get_flag("snapshot") {
            versions.iter().filter(|v| v.id.is_snapshot()).collect_vec()
        } else if matches.get_flag("other") {
            versions.iter().filter(|v| v.id.is_other()).collect_vec()
        } else if matches.get_flag("all") {
            versions.iter().collect_vec()
        } else {
            versions.iter().filter(|v| v.id.is_release()).collect_vec()
        };

        // Print a terminal table with tabulated data
        let max_len = versions_filtered
            .iter()
            .map(|v| v.id.to_string().len())
            .max()
            .expect("No versions found")
            + 1;

        for chunk in versions_filtered
            .chunks(term_size::dimensions().expect("checked above").0 / (max_len + 1))
        {
            let row = chunk
                .iter()
                .map(|v| format!("{:width$}", v.id.to_string(), width = max_len))
                .join(" ");
            println!("{}", row.trim());
        }
    } else if let Some(matches) = matches.subcommand_matches("info") {
        let manifest = manifest_thread.await??;
        let versions = manifest.versions;
        let version_ids = versions.iter().map(|v| &v.id).collect_vec();

        let version = matches
            .get_one::<String>("version")
            .expect("No version provided")
            .parse::<VersionNumber>()
            .unwrap_or_else(|v| {
                cmd.error(
                    ErrorKind::ValueValidation,
                    format!("Version failed to parse: {}", v),
                )
                .exit()
            });

        if !version_ids.contains(&&version) {
            cmd.error(
                ErrorKind::ValueValidation,
                format!("Invalid version: {}", version),
            )
            .exit();
        }

        let version = versions
            .iter()
            .find(|v| v.id == version)
            .expect("infallible"); // checked above

        let time_format = "%-d %B %Y at %-I:%M:%S%P UTC";
        let message = format!(
            "Version {} ({})\nReleased: {}\nLast updated: {}",
            version.id,
            version.release_type,
            version.release_time.format(time_format),
            version.time.format(time_format),
        );

        println!("{}", message);
    } else if let Some(matches) = matches.subcommand_matches("install") {
        let manifest = manifest_thread.await??;
        let versions = manifest.versions;
        let version_ids = versions.iter().map(|v| &v.id).collect_vec();
        let latest = manifest.latest;

        if let Some(matches) = matches.get_many::<String>("version") {
            let (valid, invalid): (Vec<_>, Vec<_>) = matches
                .into_iter()
                .map(|v| {
                    v.parse::<VersionNumber>()
                        .wrap_err(format!("{}", v))
                        .unwrap()
                })
                .partition(|v| version_ids.contains(&&v));

            if valid.is_empty() {
                cmd.error(
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

            println!("{}\n", message);

            let to_install_versions = versions
                .iter()
                .filter(|v| valid.contains(&v.id))
                .collect_vec();

            install_versions(to_install_versions).await?;
        } else {
            println!("Installing latest release version\n");
            let latest = versions
                .iter()
                .find(|v| v.id == latest.release)
                .ok_or_else(|| eyre!("No latest release version found"))?;

            install_versions(vec![latest]).await?;
        }
    } else if let Some(matches) = matches.subcommand_matches("run") {
        // println!("{:?}", versions);
        todo!("Run version");
    };

    Ok(())
}
