#![warn(clippy::all)]

pub(crate) mod types;
pub(crate) mod utils;

use std::{env::current_exe, path::PathBuf, time::Duration};

use crate::types::version::{GameVersion, VersionMetadata, VersionNumber};
use crate::utils::net::{get_version_manifest, get_version_metadata};

use anyhow::Result;
use clap::{
    arg, command, crate_version, error::ErrorKind, value_parser, ArgAction, ArgGroup, Command,
};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use itertools::Itertools;
use tokio::{fs, task::JoinSet};

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
    let manifest_thread = tokio::spawn(async move {
        let manifest = get_version_manifest().await?;

        Ok::<_, anyhow::Error>(manifest)
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
            .unwrap_or_else(|| unreachable!()); // at least i think so

        println!("Version: {}", version.id);
        println!("Type: {}", version.release_type);
        // println!("URL: {}", version.url);
        println!("Release time: {}", version.release_time);
        // println!("Updated: {}", version.time); // meta update time?
    } else if let Some(matches) = matches.subcommand_matches("install") {
        let manifest = manifest_thread.await??;
        let versions = manifest.versions;
        let version_ids = versions.iter().map(|v| &v.id).collect_vec();
        let latest = manifest.latest;

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
                    ErrorKind::ValueValidation,
                    format!("No valid versions found (got {})", invalid.len()),
                )
                .exit();
            }

            println!(
                "Installing {} versions: {}",
                valid.len(),
                valid.iter().map(ToString::to_string).join(", ")
            );
            if !invalid.is_empty() {
                println!(
                    "(Skipped {} invalid versions: {})",
                    invalid.len(),
                    invalid.iter().map(ToString::to_string).join(", ")
                );
            }

            println!();

            let to_install_versions = versions
                .iter()
                .filter(|v| valid.contains(&v.id))
                .collect_vec();

            let mut install_threads = JoinSet::new();

            let bars = MultiProgress::new();

            for version in to_install_versions {
                let bar = bars.add(ProgressBar::new_spinner());
                bar.set_style(
                    ProgressStyle::with_template(
                        "{prefix:.bold.blue.bright} {spinner:.green.bright} {wide_msg}",
                    )
                    .unwrap()
                    .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏-"),
                );
                bar.enable_steady_tick(Duration::from_millis(100));
                bar.set_prefix(format!("{}", version.id));

                bar.set_message("Getting version metadata...");
                let version_meta: VersionMetadata = get_version_metadata(&version).await?;

                install_threads.spawn(async move {
                    if !version_meta.downloads.contains_key("server") {
                        bar.finish_with_message("Cancelled (no server jar)");
                        return Ok::<(), anyhow::Error>(());
                    }

                    let dir: PathBuf = current_exe()
                        .unwrap_or_else(|e| panic!("Failed to get current executable path: {}", e))
                        .parent()
                        .unwrap_or_else(|| unreachable!())
                        .join(".versions")
                        .join(&version_meta.id.to_string());

                    if dir.exists() {
                        bar.finish_with_message("Cancelled (already installed)");
                        return Ok::<(), anyhow::Error>(());
                    }

                    let url = version_meta
                        .downloads
                        .get("server")
                        .unwrap_or_else(|| unreachable!())
                        .url
                        .clone();

                    bar.set_message("Downloading server jar...");
                    let server_jar = reqwest::get(url).await?.bytes().await?;

                    // write to disk
                    bar.set_message("Writing server jar to disk...");
                    fs::create_dir_all(&dir).await.unwrap_or_else(|e| {
                        panic!(
                            "Failed to create directory for version {}: {}",
                            version_meta.id, e
                        )
                    });
                    fs::write(dir.join("server.jar"), server_jar)
                        .await
                        .unwrap_or_else(|e| {
                            panic!(
                                "Failed to write server jar for version {}: {}",
                                version_meta.id, e
                            )
                        });

                    // dbg!(response);
                    bar.finish_with_message("Done!");
                    Ok::<(), anyhow::Error>(())
                });
            }

            while let Some(result) = install_threads.join_next().await {
                let output = result?;
                if let Err(e) = output {
                    cmd.error(
                        ErrorKind::ValueValidation,
                        format!("Failed to install requested versions: {}", e),
                    )
                    .exit();
                }
            }
        } else {
            println!("Installing latest release version");
            let latest = versions
                .iter()
                .find(|v| v.id == latest.release)
                .expect("No version matching latest release found");
            let url = latest.url.clone();
            tokio::spawn(async move {
                let response = reqwest::get(url).await?.text().await?;
                dbg!(response);
                Ok::<(), reqwest::Error>(())
            });
        }
    } else if let Some(matches) = matches.subcommand_matches("run") {
        // println!("{:?}", versions);
        todo!("Run version");
    };

    Ok(())
}
