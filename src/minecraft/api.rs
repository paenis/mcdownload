use std::collections::HashMap;

use anyhow::Result;
use serde::Deserialize;

use crate::minecraft::VersionNumber;
use crate::net;

const BASE_URL: &str = "https://piston-meta.mojang.com/";

/// Create a piston URL from a relative path
macro_rules! piston {
    ($path:expr) => {
        const_format::concatcp!(BASE_URL, $path)
    };
}

// these use `VersionNumber` because it's possible that they parse as non-standard versions
#[derive(Debug, Deserialize)]
struct LatestVersions {
    release: VersionNumber,
    snapshot: VersionNumber,
}

/// Data type representing the entries in the `versions` field of the [top-level piston-meta JSON object](https://piston-meta.mojang.com/mc/game/version_manifest_v2.json)
///
/// The actual JSON object also includes the `sha1` and `complianceLevel` fields, but they are not relevant for this project
#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MinecraftVersion {
    /// Version number corresponding to the release.
    pub id: VersionNumber,
    /// Type of release, e.g. "`release`", "`snapshot`", "`old_beta`", "`old_alpha`".
    r#type: String, // TODO: potential enum
    /// URL pointing to the specific game version package.
    url: String,
    /// Last modified time (of what? probably the manifest, but not sure).
    time: jiff::Timestamp,
    /// Time of release.
    release_time: jiff::Timestamp,
    // /// SHA-1 hash of something...
    // sha1: String,
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
    id: VersionNumber,
    #[serde(default)]
    java_version: JavaVersion,
    release_time: jiff::Timestamp,
    time: jiff::Timestamp,
    r#type: String,
}

impl MinecraftVersion {
    pub fn get_package(&self) -> Result<GamePackage> {
        crate::RT.block_on(net::get_cached(&self.url, None))
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

    pub fn latest_release_id(&self) -> &VersionNumber {
        &self.latest.release
    }
    pub fn latest_snapshot_id(&self) -> &VersionNumber {
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
pub fn get_manifest() -> Result<VersionManifest> {
    crate::RT.block_on(net::get_cached(
        piston!("mc/game/version_manifest_v2.json"),
        None,
    ))
}

pub fn find_version(id: &VersionNumber) -> Option<MinecraftVersion> {
    // TODO: Result<Option<MinecraftVersion>>
    get_manifest().ok()?.into_iter().find(|v| &v.id == id)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn create_url() {
        assert_eq!(
            piston!("versions"),
            "https://piston-meta.mojang.com/versions"
        );
    }

    #[test]
    fn latest_version() {
        let manifest = get_manifest().unwrap();
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
                id: VersionNumber::Release(crate::minecraft::ReleaseVersionNumber { major: 1, minor: 21, patch: 4 }),
                r#type: "release".into(),
                url: "https://piston-meta.mojang.com/v1/packages/a3bcba436caa849622fd7e1e5b89489ed6c9ac63/1.21.4.json".into(),
                time: "2024-12-03T10:24:48+00:00".parse().unwrap(),
                release_time: "2024-12-03T10:12:57+00:00".parse().unwrap(),
            }
        )
    }

    #[test]
    fn deserialize_all() {
        // check that manifest versions deserialize successfully
        let _versions: Vec<_> = get_manifest().unwrap().into_iter().map(|v| v.id).collect();
    }

    #[test]
    fn all_valid() {
        // slow

        let now = std::time::Instant::now();
        let manifest = get_manifest().unwrap();

        let test_ids: Vec<VersionNumber> = std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("test_data/id.list"),
        )
        .unwrap()
        .lines()
        .map(|l| l.parse().unwrap())
        .collect::<Vec<_>>();

        let manifest_ids: Vec<VersionNumber> = manifest.into_iter().map(|v| v.id).collect();

        assert!(test_ids.iter().all(|v| manifest_ids.contains(v)));

        eprintln!(
            "checked {} versions in {:?} ({:?}/version)",
            test_ids.len(),
            now.elapsed(),
            now.elapsed() / test_ids.len() as u32
        );
    }
}
