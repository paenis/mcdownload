use std::collections::HashMap;

use anyhow::Result;
use serde::Deserialize;
use tokio::runtime::Handle;

use crate::net;

/// A valid Minecraft version identifier.
#[derive(Deserialize, Clone, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct VersionId(String);

impl VersionId {
    // TODO: error type
    // HACK: this isn't part of the FromStr trait in order to make it async
    /// Parse a version ID from a string.
    pub async fn from_str(s: &str) -> Result<Self> {
        let manifest = get_manifest().await?;
        if manifest.versions.iter().any(|v| v.id.0 == s) {
            Ok(VersionId(s.to_string()))
        } else {
            Err(anyhow::anyhow!("Invalid version ID"))
        }
    }

    /// Synchronous version of `from_str`, for use in clap value parsers
    ///
    /// NOTE: this function will block the current thread while it runs, so it should ONLY be
    /// used in contexts where async code cannot be used and/or blocking is acceptable.
    pub fn from_str_sync(s: &str) -> Result<Self> {
        // really evil hack to run async code in a sync context
        tokio::task::block_in_place(move || Handle::current().block_on(Self::from_str(s)))
    }

    // FIXME: placeholder implementation
    pub fn empty() -> Self {
        Self("".into())
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
    pub async fn get_package(&self) -> Result<GamePackage> {
        net::get_cached(&self.url, None).await
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

/// Convenience method to get the Minecraft version manifest
///
/// This is the same as calling `get_cached::<VersionManifest>(&piston("mc/game/version_manifest_v2.json"))`
pub async fn get_manifest() -> Result<VersionManifest> {
    net::get_cached(
        "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json",
        None,
    )
    .await
}

pub async fn find_version(id: &VersionId) -> Result<MinecraftVersion> {
    let ver = get_manifest()
        .await?
        .into_iter()
        .find(|v| v.id == *id)
        .expect("version id is valid");
    Ok(ver)
}

#[cfg(test)]
mod tests {
    use std::io::{BufRead as _, BufReader};
    use std::path::Path;

    use tokio::task::JoinSet;

    use super::*;

    #[tokio::test]
    async fn latest_version() {
        let manifest = get_manifest().await.unwrap();
        assert_eq!(manifest.latest_release_id(), &manifest.latest_release().id);
        assert_eq!(
            manifest.latest_snapshot_id(),
            &manifest.latest_snapshot().id
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

    #[tokio::test]
    async fn deserialize_all() {
        // check that manifest versions deserialize successfully
        let _versions: Vec<_> = get_manifest()
            .await
            .unwrap()
            .into_iter()
            .map(|v| v.id)
            .collect();
    }

    #[tokio::test]
    async fn all_valid() {
        // slow

        let now = std::time::Instant::now();
        let manifest = get_manifest().await.unwrap();

        let mut tasks = JoinSet::new();

        let reader = BufReader::new(
            std::fs::File::open(Path::new(env!("CARGO_MANIFEST_DIR")).join("test_data/id.list"))
                .unwrap(),
        );

        for line in reader.lines() {
            tasks.spawn(async move { VersionId::from_str(&line.unwrap()).await.unwrap() });
        }

        let test_ids: Vec<VersionId> = tasks.join_all().await;

        let manifest_ids: Vec<VersionId> = manifest.into_iter().map(|v| v.id).collect();

        assert!(test_ids.iter().all(|v| manifest_ids.contains(v)));

        eprintln!(
            "checked {} versions in {:?} ({:?}/version)",
            test_ids.len(),
            now.elapsed(),
            now.elapsed() / test_ids.len() as u32
        );
    }
}
