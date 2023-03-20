pub(crate) mod types;

use crate::types::GameVersionList;

use anyhow::Result;
use clap::{arg, command, crate_version, value_parser, ArgAction, ArgGroup, Command};
use itertools::Itertools;

const PISTON_API_URL: &str = "https://piston-meta.mojang.com/";
const FABRIC_API_URL: &str = "https://meta.fabricmc.net/";

fn api_path(path: &str) -> String {
    format!("{}{}", PISTON_API_URL, path)
}

async fn get_version_manifest() -> Result<GameVersionList> {
    let version_manifest_url = api_path("mc/game/version_manifest.json");
    let response = reqwest::get(version_manifest_url)
        .await?
        .json::<GameVersionList>()
        .await?;

    Ok(response)
}

fn into_iter_filter_vec<T, F>(iter: impl Iterator<Item = T>, f: F) -> Vec<T>
where
    F: Fn(&T) -> bool,
{
    iter.into_iter().filter(f).collect_vec()
}

#[tokio::main]
async fn main() -> Result<()> {
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
                        .value_parser(value_parser!(String)), // TODO: validate version, i.e implement FromStr for VersionNumber
                ),
        );
    // .subcommand(Command::new("uninstall").about("Uninstall a Minecraft version"))

    let matches = cmd.get_matches();

    if let Some(matches) = matches.subcommand_matches("list") {
        let versions = get_version_manifest().await?;

        let versions = match (
            matches.get_flag("release"),
            matches.get_flag("pre-release"),
            matches.get_flag("snapshot"),
            matches.get_flag("other"),
            matches.get_flag("all"),
        ) {
            (true, false, false, false, false) => {
                into_iter_filter_vec(versions, |v| v.id.is_release())
            }
            (false, true, false, false, false) => {
                into_iter_filter_vec(versions, |v| v.id.is_pre_release())
            }
            (false, false, true, false, false) => {
                into_iter_filter_vec(versions, |v| v.id.is_snapshot())
            }
            (false, false, false, true, false) => {
                into_iter_filter_vec(versions, |v| v.id.is_other())
            }
            (false, false, false, false, true) => versions.collect_vec(),
            _ => into_iter_filter_vec(versions, |v| v.id.is_release()),
        };

        // print as terminal table, tabulated to fill terminal width
        let table = versions.into_iter().map(|v| v.id).collect_vec();

        let max_len = table.iter().map(|v| v.to_string().len()).max().unwrap() + 1;

        let table = table
            .into_iter()
            .chunks(term_size::dimensions().unwrap().0 / (max_len + 1))
            .into_iter()
            .map(|c| c.collect_vec())
            .collect_vec();

        for row in table {
            println!(
                "{}",
                row.into_iter()
                    .map(|v| v.to_string()) // for some reason, this is necessary
                    .map(|v| format!("{:width$}", v, width = max_len))
                    .join(" ")
                    .trim()
            );
        }
    }

    if let Some(matches) = matches.subcommand_matches("install") {
        todo!("Install version(s)");

        #[allow(unreachable_code)]
        match matches.get_count("version") {
            0 => todo!("Install latest release version"),
            1 => todo!("Install specified version"),
            _ => todo!("Install multiple versions (async(?))"),
        }
        // let versions: Vec<crate::types::VersionNumber> = matches
        //     .get_many("version")
        //     .expect("No version specified")
        //     .map(|v: &String| crate::types::VersionNumber::from_str(v))
        //     .collect();
        // println!("{:#?}", versions);
    };

    if let Some(matches) = matches.subcommand_matches("run") {
        todo!("Run version");
    };

    // let versions_other = get_version_manifest()
    //     .await?
    //     .into_iter()
    //     .filter(|v| match v.id {
    //         VersionNumber::Other(_) => true,
    //         _ => false,
    //     })
    //     .collect_vec();

    // println!("{:#?}", versions_other);

    // let release_versions = versions.into_iter().filter(|v| v.release_type == "release");
    // let release_ids = release_versions.map(|v| v.id).collect_vec();

    // println!("{:?}", release_ids);
    // println!("{}", serde_json::to_string_pretty(&versions)?);
    // println!("{}", versions_list.iter().format("\n"));

    Ok(())
}
