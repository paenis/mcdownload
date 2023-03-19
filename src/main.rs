pub(crate) mod types;

use crate::types::{GameVersionList, VersionNumber};

use std::error::Error;

use clap::{arg, command, crate_version, value_parser, ArgAction, ArgGroup, Command};
use itertools::Itertools;

const PISTON_API_URL: &str = "https://piston-meta.mojang.com/";
const FABRIC_API_URL: &str = "https://meta.fabricmc.net/";

fn api_path(path: &str) -> String {
    format!("{}{}", PISTON_API_URL, path)
}

async fn get_version_manifest() -> Result<GameVersionList, Box<dyn Error>> {
    let version_manifest_url = api_path("mc/game/version_manifest.json");
    let response = reqwest::get(version_manifest_url)
        .await?
        .json::<GameVersionList>()
        .await?;

    Ok(response)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let cmd = command!()
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
            Command::new("install")
                .about("Install a Minecraft version")
                .after_help("Defaults to latest release version")
                .arg(
                    arg!(-v --version "The version(s) to install")
                        .action(ArgAction::Append)
                        .value_delimiter(' ')
                        .num_args(0..)
                        .value_parser(value_parser!(String)),
                ),
        )
        .subcommand(
            Command::new("run").about("Run a Minecraft version").arg(
                arg!(-v --version <VERSION> "The version to run")
                    .required(true)
                    .value_parser(value_parser!(String)), // TODO: validate version, i.e implement FromStr for VersionNumber
            ),
        );
    // .subcommand(Command::new("uninstall").about("Uninstall a Minecraft version"))

    let matches = cmd.get_matches();

    let versions_other = get_version_manifest()
        .await?
        .into_iter()
        .filter(|v| match v.id {
            VersionNumber::Other(_) => true,
            _ => false,
        })
        .collect_vec();

    println!("{:#?}", versions_other);

    // let release_versions = versions.into_iter().filter(|v| v.release_type == "release");
    // let release_ids = release_versions.map(|v| v.id).collect_vec();

    // println!("{:?}", release_ids);
    // println!("{}", serde_json::to_string_pretty(&versions)?);
    // println!("{}", versions_list.iter().format("\n"));

    Ok(())
}
