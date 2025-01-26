use std::sync::OnceLock;

use anyhow::Result;
use serde::Deserialize;
use ureq::{Agent, AgentBuilder};

use crate::minecraft::VersionNumber;

static AGENT: OnceLock<Agent> = OnceLock::new();
const BASE_URL: &str = "https://piston-meta.mojang.com/";

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
    id: VersionNumber,
    /// Type of release, e.g. "release", "snapshot", "old_beta", "old_alpha".
    r#type: String, // TODO: potential enum
    /// URL pointing to the specific game version package.
    url: String,
    /// Last modified time (of what? probably the manifest, but not sure).
    time: String, // chrono::DateTime, either FixedOffset or Utc
    /// Time of release.
    release_time: String, // see above
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
struct JavaVersion {
    component: String,
    major_version: u8,
}

/// Package information for a specific game version, from the `url` field of the [`MinecraftVersion`] struct. Includes downloads and Java version information.
#[derive(Debug, Deserialize)]
pub struct GamePackage {
    downloads: Vec<Download>,
    id: VersionNumber,
    java_version: JavaVersion,
    release_time: String,
    time: String,
    r#type: String,
}

impl MinecraftVersion {
    pub fn get_package(&self) -> Result<GamePackage> {
        get(&self.url)
    }
}

#[derive(Debug, Deserialize)]
pub struct VersionManifest {
    latest: LatestVersions,
    versions: Vec<MinecraftVersion>,
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
        &self.latest_release().id
    }
    pub fn latest_snapshot_id(&self) -> &VersionNumber {
        &self.latest_snapshot().id
    }
}

/// Builds a URL from a relative path.
#[inline]
fn piston(path: &str) -> String {
    format!("{BASE_URL}{path}")
}

fn build_agent() -> Agent {
    // TODO: set user agent at compile time (e.g. vergen)
    AgentBuilder::new()
        .user_agent("mcdl/0.3.0")
        .timeout(std::time::Duration::from_secs(5))
        .build()
}

fn agent() -> &'static Agent {
    AGENT.get_or_init(build_agent)
}

/// Calls the Piston API and returns the parsed JSON response
pub fn get<T: serde::de::DeserializeOwned>(path: &str) -> Result<T> {
    // TODO: adapt to use caching
    Ok(agent().get(&piston(path)).call()?.into_json()?)
}

/// Convenience method to get the Minecraft version manifest
///
/// This is the same as calling `get::<VersionManifest>("mc/game/version_manifest_v2.json")`
pub fn get_manifest() -> Result<VersionManifest> {
    get("mc/game/version_manifest_v2.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_url() {
        assert_eq!(
            piston("versions"),
            "https://piston-meta.mojang.com/versions"
        );
    }

    #[test]
    fn latest_version() {
        let v = get::<serde_json::Value>("mc/game/version_manifest_v2.json").unwrap();
        let latest = v.get("latest");
        assert!(latest.is_some());
        let release = latest.unwrap().get("release");
        assert!(release.is_some());
        eprintln!("{:?}", release.unwrap());
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
                time: "2024-12-03T10:24:48+00:00".into(),
                release_time: "2024-12-03T10:12:57+00:00".into(),
            }
        )
    }

    #[test]
    fn deserialize_all() {
        let json = include_str!("../../test_data/versions.json");

        // check that manifest versions deserialize successfully
        let _versions: Vec<_> = serde_json::from_str::<Vec<MinecraftVersion>>(&json)
            .unwrap()
            .into_iter()
            .map(|v| v.id)
            .collect();
    }
}
