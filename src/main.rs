pub(crate) mod types;
pub(crate) mod utils;

use crate::types::version::{GameVersion, VersionNumber};
use crate::utils::net::get_version_manifest;

use anyhow::Result;
use clap::{arg, command, crate_version, value_parser, ArgAction, ArgGroup, Command};
use itertools::Itertools;

#[tokio::main]
async fn main() -> Result<()> {
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
    // TODO: cache this?
    let manifest = get_version_manifest().await?;
    let versions = manifest.versions;
    let version_ids = versions.iter().map(|v| &v.id).collect_vec();
    let latest = manifest.latest;

    if let Some(matches) = matches.subcommand_matches("list") {
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

        for chunk in versions_filtered.chunks(
            term_size::dimensions()
                .expect("No terminal output (what were you thinking?)")
                .0
                / (max_len + 1),
        ) {
            let row = chunk
                .into_iter()
                .map(|v| format!("{:width$}", v.id.to_string(), width = max_len))
                .join(" ");
            println!("{}", row.trim());
        }
    }

    if let Some(matches) = matches.subcommand_matches("info") {
        let version = matches
            .get_one::<String>("version")
            .expect("No version provided")
            .parse::<VersionNumber>()
            .unwrap_or_else(|v| {
                cmd.error(
                    clap::error::ErrorKind::ValueValidation,
                    format!("Version failed to parse: {}", v),
                )
                .exit()
            });

        if !version_ids.contains(&&version) {
            cmd.error(
                clap::error::ErrorKind::ValueValidation,
                format!("Invalid version: {}", version),
            )
            .exit();
        }

        let version = versions
            .iter()
            .find(|v| v.id == version)
            .unwrap_or_else(|| unreachable!()); // at least i think so

        println!("Version: {}", version.id);
        println!("Type: {}", version.release_type);
        // println!("URL: {}", version.url);
        println!("Release time: {}", version.release_time);
        // println!("Updated: {}", version.time); // meta update time?
    }

    if let Some(matches) = matches.subcommand_matches("install") {
        if let Some(matches) = matches.get_many::<String>("version") {
            let to_install = matches
                .into_iter()
                .map(|v| v.parse::<VersionNumber>().expect("Failed to parse version"))
                .collect_vec();

            let (valid, invalid): (Vec<_>, Vec<_>) = to_install
                .into_iter()
                .partition(|v| version_ids.contains(&&v));

            if valid.is_empty() {
                cmd.error(
                    clap::error::ErrorKind::ValueValidation,
                    format!("No valid versions found (got {})", invalid.len()),
                )
                .exit();
            }

            println!(
                "Installing {} versions: {}",
                valid.len(),
                valid.iter().map(|v| v.to_string()).join(", ")
            );
            if !invalid.is_empty() {
                println!(
                    "(Skipped {} invalid versions: {})",
                    invalid.len(),
                    invalid.iter().map(|v| v.to_string()).join(", ")
                );
            }

            let to_install_versions = versions
                .iter()
                .filter(|v| valid.contains(&v.id))
                .collect_vec();

            for version in to_install_versions {
                // dbg!(&version.id);

                let url = version.url.clone(); // CANNOT be borrowed (unless i'm stupid (likely))

                tokio::spawn(async move {
                    let response = reqwest::get(url).await?.text().await?;
                    // dbg!(response);
                    Ok::<(), reqwest::Error>(())
                });
            }
        } else {
            println!("Installing latest release version");
            let latest = versions.iter().find(|v| v.id == latest.release).expect("No version matching latest release found");
            let url = latest.url.clone();
            tokio::spawn(async move {
                let response = reqwest::get(url).await?.text().await?;
                dbg!(response);
                Ok::<(), reqwest::Error>(())
            });
        }

        std::thread::sleep(std::time::Duration::from_secs(2));
    };

    if let Some(matches) = matches.subcommand_matches("run") {
        println!("{:?}", versions);
        todo!("Run version");
    };

    Ok(())
}
