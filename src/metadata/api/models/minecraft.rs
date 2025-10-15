use std::collections::HashMap;
use std::str::FromStr;
use std::sync::LazyLock;

use anyhow::Result as AResult;
use serde::Deserialize;
use thiserror::Error;

use crate::macros::wait;
use crate::net::{self, NetError};

static MANIFEST: LazyLock<VersionManifest> = LazyLock::new(|| {
    wait!(net::get_cached(
        "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json",
        None,
    ))
    .expect("Failed to fetch Minecraft version manifest from Mojang API")
});

#[derive(Error, Debug)]
pub enum VersionIdParseError {
    #[error("network error when getting valid versions: {0}")]
    Network(#[from] NetError),
    #[error("version is invalid")]
    Invalid,
}

/// A valid Minecraft version identifier.
#[derive(Deserialize, Clone, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct VersionId(String);

impl VersionId {
    /// Get the version ID as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for VersionId {
    type Err = VersionIdParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if MANIFEST.versions.iter().any(|v| v.id.0 == s) {
            Ok(VersionId(s.to_string()))
        } else {
            Err(VersionIdParseError::Invalid)
        }
    }
}

impl Default for VersionId {
    fn default() -> Self {
        MANIFEST.latest.release.clone()
    }
}

impl std::fmt::Display for VersionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::fmt::Debug for VersionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Deserialize)]
struct LatestVersions {
    release: VersionId,
    snapshot: VersionId,
}

/// Data type representing the entries in the `versions` field of the [top-level piston-meta JSON object][meta]
///
/// The actual JSON object also includes the `sha1` and `complianceLevel` fields, but they are not relevant for this project
///
/// [meta]: https://piston-meta.mojang.com/mc/game/version_manifest_v2.json
#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MinecraftVersion {
    /// A unique identifier (version number) corresponding to the release.
    pub id: VersionId,
    /// Type of release.
    r#type: VersionType,
    /// URL pointing to the specific game version package.
    url: String,
    /// Last modified time (of what? probably the manifest, but not sure).
    time: jiff::Timestamp,
    /// Time of release.
    release_time: jiff::Timestamp,
    // /// SHA-1 hash of something...
    // sha1: String,
}

#[derive(Debug, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum VersionType {
    /// Stable, major version releases suitable for all players.
    Release,
    /// In-development versions that may change frequently.
    Snapshot,
    OldAlpha,
    OldBeta,
}

/// Download information for a game package, i.e. client and server jars.
#[derive(Debug, Deserialize)]
struct Download {
    size: u64,
    url: String,
}

/// Java version information for a game package.
///
/// `component` is currently unused.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct JavaVersion {
    component: String,
    major_version: u8,
}

impl Default for JavaVersion {
    /// Creates a `JavaVersion` with major version 8 and unspecified component
    fn default() -> Self {
        Self {
            component: String::new(),
            major_version: 8,
        }
    }
}

/// Package information for a specific game version, from the `url` field of the [`MinecraftVersion`] struct. Includes downloads and Java version information.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GamePackage {
    downloads: HashMap<String, Download>,
    id: VersionId,
    #[serde(default)]
    java_version: JavaVersion,
    release_time: jiff::Timestamp,
    time: jiff::Timestamp,
    r#type: String,
}

impl MinecraftVersion {
    pub async fn get_package(&self) -> AResult<GamePackage> {
        Ok(net::get_cached(&self.url, None).await?)
    }
}

#[derive(Debug, Deserialize)]
pub struct VersionManifest {
    latest: LatestVersions,
    pub versions: Vec<MinecraftVersion>,
}

impl VersionManifest {
    pub fn latest_release(&self) -> &MinecraftVersion {
        self.versions
            .iter()
            .find(|v| v.id == self.latest.release)
            .expect("latest release not in manifest")
    }
    pub fn latest_snapshot(&self) -> &MinecraftVersion {
        self.versions
            .iter()
            .find(|v| v.id == self.latest.snapshot)
            .expect("latest snapshot not in manifest")
    }

    pub fn latest_release_id(&self) -> &VersionId {
        &self.latest.release
    }
    pub fn latest_snapshot_id(&self) -> &VersionId {
        &self.latest.snapshot
    }
}

impl IntoIterator for VersionManifest {
    type Item = MinecraftVersion;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.versions.into_iter()
    }
}

pub async fn find_version(id: &VersionId) -> AResult<&'static MinecraftVersion> {
    let ver = MANIFEST
        .versions
        .iter()
        .find(|v| v.id == *id)
        .expect("valid version id should be in manifest");
    Ok(ver)
}

#[cfg(test)]
mod tests {
    use std::io::{BufRead as _, BufReader};
    use std::path::Path;

    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn latest_version() {
        assert_eq!(MANIFEST.latest_release_id(), &MANIFEST.latest_release().id);
        assert_eq!(
            MANIFEST.latest_snapshot_id(),
            &MANIFEST.latest_snapshot().id
        );
    }

    #[test]
    fn deserialize_minecraft_version() {
        let json = r#"{"id": "1.21.4", "type": "release", "url": "https://piston-meta.mojang.com/v1/packages/a3bcba436caa849622fd7e1e5b89489ed6c9ac63/1.21.4.json", "time": "2024-12-03T10:24:48+00:00", "releaseTime": "2024-12-03T10:12:57+00:00", "sha1": "a3bcba436caa849622fd7e1e5b89489ed6c9ac63", "complianceLevel": 1}"#;
        assert_eq!(
            serde_json::from_str::<MinecraftVersion>(json).unwrap(),
            MinecraftVersion {
                id: VersionId("1.21.4".into()),
                r#type: VersionType::Release,
                url: "https://piston-meta.mojang.com/v1/packages/a3bcba436caa849622fd7e1e5b89489ed6c9ac63/1.21.4.json".into(),
                time: "2024-12-03T10:24:48+00:00".parse().unwrap(),
                release_time: "2024-12-03T10:12:57+00:00".parse().unwrap(),
            }
        )
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn deserialize_all() {
        // check that manifest versions deserialize successfully
        let _versions: Vec<_> = MANIFEST.versions.iter().map(|v| v.id.clone()).collect();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn all_valid() {
        let now = std::time::Instant::now();

        let reader = BufReader::new(
            std::fs::File::open(Path::new(env!("CARGO_MANIFEST_DIR")).join("test_data/id.list"))
                .unwrap(),
        );

        let test_ids: Vec<VersionId> = reader
            .lines()
            .map(|l| VersionId::from_str(&l.unwrap()).unwrap())
            .collect();

        let manifest_ids: Vec<VersionId> = MANIFEST.versions.iter().map(|v| v.id.clone()).collect();

        assert!(test_ids.iter().all(|v| manifest_ids.contains(v)));

        eprintln!(
            "checked {} versions in {:?} ({:?}/version)",
            test_ids.len(),
            now.elapsed(),
            now.elapsed() / test_ids.len() as u32
        );
    }
}
