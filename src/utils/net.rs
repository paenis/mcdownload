use std::env::current_exe;

use bytes::Bytes;
use chrono::{Duration, Utc};
use color_eyre::eyre::{eyre, Result};
use reqwest::StatusCode;

use crate::types::{
    net::CachedResponse,
    version::{GameVersion, GameVersionList, VersionMetadata},
};

const PISTON_API_URL: &str = "https://piston-meta.mojang.com/";
const FABRIC_API_URL: &str = "https://meta.fabricmc.net/";

const CACHE_PATH: &str = ".meta";
const CACHE_EXPIRATION_TIME: i64 = 60 * 10; // 10 minutes

pub(crate) fn api_path(path: &str) -> String {
    format!("{}{}", PISTON_API_URL, path)
}

pub(crate) fn fabric_api_path(path: &str) -> String {
    format!("{}{}", FABRIC_API_URL, path)
}

pub(crate) async fn get_version_manifest() -> Result<GameVersionList> {
    let version_manifest_url = api_path("mc/game/version_manifest.json");
    let cache_file = current_exe()?
        .parent()
        .expect("infallible")
        .join(CACHE_PATH)
        .join("manifest.mpk");

    // check if file exists and is not expired
    // if so, return cached data
    if let Ok(cached) = CachedResponse::<GameVersionList>::from_file(&cache_file).await {
        if !cached.is_expired() {
            return Ok(cached.data);
        }
    }

    // file doesn't exist or is expired, get fresh data
    let response: GameVersionList = reqwest::get(version_manifest_url).await?.json().await?;

    // save to disk
    let cached_response = CachedResponse::new(
        &response,
        Utc::now() + Duration::seconds(CACHE_EXPIRATION_TIME),
    );
    cached_response.save(&cache_file).await?;

    Ok(response)
}

pub(crate) async fn get_version_metadata(version: &GameVersion) -> Result<VersionMetadata> {
    let meta_url = version.url.clone();

    let cache_file = current_exe()?
        .parent()
        .expect("infallible")
        .join(CACHE_PATH)
        .join(format!("{}.mpk", version.id));

    if let Ok(cached) = CachedResponse::<VersionMetadata>::from_file(&cache_file).await {
        if !cached.is_expired() {
            return Ok(cached.data);
        }
    }

    let response: VersionMetadata = reqwest::get(meta_url).await?.json().await?;

    let cached_response = CachedResponse::new(
        &response,
        Utc::now() + Duration::seconds(CACHE_EXPIRATION_TIME),
    );
    cached_response.save(&cache_file).await?;

    Ok(response)
}

pub(crate) async fn download_jre(major_version: &u8) -> Result<Bytes> {
    let url = format!(
        "https://api.adoptium.net/v3/binary/latest/{feature_version}/{release_type}/{os}/{arch}/{image_type}/{jvm_impl}/{heap_size}/{vendor}",
        feature_version = major_version,
        release_type = "ga",
        os = std::env::consts::OS, // fine
        arch = std::env::consts::ARCH,
        image_type = "jre",
        jvm_impl = "hotspot",
        heap_size = "normal",
        vendor = "eclipse",
    );

    let response = reqwest::get(&url).await?;

    match response.status() {
        StatusCode::TEMPORARY_REDIRECT | StatusCode::OK => Ok(response.bytes().await?),
        StatusCode::BAD_REQUEST => Err(eyre!("Bad input parameter in URL: {url}")),
        StatusCode::NOT_FOUND => Err(eyre!("No binary found for the given parameters: {url}")),
        status => Err(eyre!("Unexpected error (status code {status}): {url}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_version_manifest() {
        let manifest = get_version_manifest().await.unwrap();
        assert!(manifest.versions.len() > 0);
    }

    #[tokio::test]
    async fn test_get_version_metadata() {
        let manifest = get_version_manifest().await.unwrap();
        let version = manifest.versions.get(0).unwrap();
        let metadata = get_version_metadata(version).await.unwrap();
        assert!(metadata.downloads.get("server").is_some());
    }

    #[tokio::test]
    async fn test_download_jre() {
        let jre = download_jre(&8).await.unwrap();
        assert!(jre.len() > 0);
    }
}
