use std::env::current_exe;

use anyhow::Result;
use chrono::{Duration, Utc};
use tokio::fs;

use crate::types::{net::CachedResponse, version::GameVersionList};

const PISTON_API_URL: &str = "https://piston-meta.mojang.com/";
const FABRIC_API_URL: &str = "https://meta.fabricmc.net/";

const CACHE_FILE: &str = ".manifest.json";
const CACHE_EXPIRATION_TIME: i64 = 60 * 10; // 10 minutes

pub(crate) fn api_path(path: &str) -> String {
    format!("{}{}", PISTON_API_URL, path)
}

pub(crate) fn fabric_api_path(path: &str) -> String {
    format!("{}{}", FABRIC_API_URL, path)
}

pub(crate) async fn get_version_manifest() -> Result<GameVersionList> {
    let version_manifest_url = api_path("mc/game/version_manifest.json");
    let cache_file = current_exe()?.parent().expect("infallible").join(CACHE_FILE);

    // check if file exists and is not expired
    // if so, return cached data
    if let Ok(data) = fs::read_to_string(&cache_file).await {
        if let Ok(cached) = serde_json::from_str::<CachedResponse<GameVersionList>>(&data) {
            if !cached.is_expired() {
                return Ok(cached.data);
            }
        }
    }

    // file doesn't exist or is expired, get fresh data
    let response = reqwest::get(version_manifest_url)
        .await?
        .json::<GameVersionList>()
        .await?;

    // save to disk
    let cached_response = CachedResponse::new(&response, Utc::now() + Duration::seconds(CACHE_EXPIRATION_TIME));
    let data = serde_json::to_string(&cached_response)?;
    fs::write(cache_file, data).await?;

    Ok(response)
}
