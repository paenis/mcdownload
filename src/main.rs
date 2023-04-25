#![warn(clippy::all)]

//! A tool for managing Minecraft server versions

pub(crate) mod app;
pub(crate) mod types;
pub(crate) mod utils;

use clap::error::ErrorKind;
use clap::{arg, command, crate_version, value_parser, ArgAction, ArgGroup, Command};
use color_eyre::eyre::{self, eyre, Result, WrapErr};
use is_terminal::IsTerminal;
use itertools::Itertools;

use crate::types::version::{GameVersion, VersionNumber};
use crate::utils::net::get_version_manifest;

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
                .after_help("Defaults to latest release version if none is provided")
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
                .after_help("Version must be installed first")
                .arg(
                    arg!(-v --version <VERSION> "The version to run")
                        .required(true)
                        .value_parser(value_parser!(String)), // parse as String here, validate later
                ),
        )
        .subcommand(
            Command::new("locate")
                .about("Print the path to a config file or instance directory")
                .after_help("Supported locations:\n\
                \tjre | java - The Java Runtime Environment directory\n\
                \tinstances | versions | server - The directory containing Minecraft server versions\n\
                \tconfig | settings - The directory containing config files")
                .arg(arg!([what] "The file or directory to locate").required(true)),
        );
    // .subcommand(Command::new("uninstall").about("Uninstall a Minecraft version"))

    let matches = cmd.get_matches_mut();

    // shared manifest between subcommands
    let manifest_thread = tokio::spawn(async move {
        let manifest = get_version_manifest().await?;
        Ok::<_, eyre::Report>(manifest)
    });

    if let Some(matches) = matches.subcommand_matches("list") {
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

        if !std::io::stdout().is_terminal() {
            versions_filtered.iter().for_each(|v| println!("{}", v.id));
            return Ok(());
        }

        let term_width = match terminal_size::terminal_size() {
            Some((terminal_size::Width(w), _)) => {
                if w < 20 {
                    cmd.error(ErrorKind::Io, "Terminal width is too small")
                        .exit();
                }
                w as usize
            }
            _ => panic!("stdout is a terminal but has no size"),
        };

        // Print a terminal table with tabulated data
        let max_len = versions_filtered
            .iter()
            .map(|v| v.id.to_string().len())
            .max()
            .expect("No versions found")
            + 1;

        // unwrap checked above
        for chunk in versions_filtered.chunks(term_width / (max_len + 1)) {
            let row = chunk
                .iter()
                .map(|v| format!("{:max_len$}", v.id.to_string()))
                .join(" ");
            println!("{}", row.trim());
        }
    } else if let Some(matches) = matches.subcommand_matches("info") {
        let manifest = manifest_thread.await??;
        let versions = manifest.versions;
        let version_ids = versions.iter().map(|v| &v.id).collect_vec();

        let version: VersionNumber = matches
            .get_one::<String>("version")
            .expect("No version provided")
            .parse()
            .expect("infallible");

        if !version_ids.contains(&&version) {
            cmd.error(
                ErrorKind::ValueValidation,
                format!("Invalid version: {version}"),
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

        println!("{message}");
    } else if let Some(matches) = matches.subcommand_matches("install") {
        let manifest = manifest_thread.await??;
        let versions = manifest.versions;
        let version_ids = versions.iter().map(|v| &v.id).collect_vec();
        let latest = manifest.latest;

        if let Some(matches) = matches.get_many::<String>("version") {
            let (valid, invalid): (Vec<_>, Vec<_>) = matches
                .into_iter()
                .map(|v| v.parse::<VersionNumber>().expect("infallible"))
                .partition(|v| version_ids.contains(&v));

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

            println!("{message}\n");

            let to_install_versions = versions
                .iter()
                .filter(|v| valid.contains(&v.id))
                .collect_vec();

            app::install_versions(to_install_versions)
                .await
                .wrap_err("Error while installing multiple versions")?;
        } else {
            println!("Installing latest release version\n");
            let latest = versions
                .iter()
                .find(|v| v.id == latest.release)
                .ok_or_else(|| eyre!("No latest release version found"))?;

            app::install_versions(vec![latest])
                .await
                .wrap_err("Error while installing latest version")?;
        }
    } else if let Some(matches) = matches.subcommand_matches("run") {
        let version: VersionNumber = matches
            .get_one::<String>("version")
            .expect("No version provided")
            .parse()
            .expect("infallible");

        app::run_version(version)
            .await
            .wrap_err("Error while running server")?;
    } else if let Some(matches) = matches.subcommand_matches("locate") {
        let what = matches
            .get_one::<String>("what")
            .expect("No input provided");

        app::locate(what).wrap_err(format!("Error while locating {what}"))?;
    };

    Ok(())
}
