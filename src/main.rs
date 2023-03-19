pub(crate) mod types;

use crate::types::{GameVersionList, VersionNumber};

use std::error::Error;

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
    let versions = get_version_manifest()
        .await?
        .into_iter()
        .collect_vec();

    // let release_versions = versions.into_iter().filter(|v| v.release_type == "release");
    // let release_ids = release_versions.map(|v| v.id).collect_vec();

    // println!("{:?}", release_ids);
    println!("{}", serde_json::to_string_pretty(&versions)?);
    // println!("{}", versions_list.iter().format("\n"));

    Ok(())
}
