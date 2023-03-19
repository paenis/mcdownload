use std::{fmt::Display, str::FromStr};

use chrono::{DateTime, FixedOffset};
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_with::SerializeDisplay;

/// Version format for release versions
/// in the form of X.Y.Z
#[derive(Debug, SerializeDisplay, PartialEq, Eq, PartialOrd, Ord)]
pub struct ReleaseVersion {
    major: u64,
    minor: u64,
    patch: u64,
}

impl Display for ReleaseVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReleaseVersion {
                major,
                minor,
                patch: 0,
            } => write!(f, "{}.{}", major, minor),
            ReleaseVersion {
                major,
                minor,
                patch,
            } => write!(f, "{}.{}.{}", major, minor, patch),
        }
    }
}

impl<'de> Deserialize<'de> for ReleaseVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        lazy_static! {
            static ref RE: Regex = Regex::new(r"^(\d+)\.(\d+)(?:\.(\d+))?$").unwrap();
        }
        match RE.captures(&s) {
            Some(caps) => Ok(ReleaseVersion {
                major: caps[1].parse().unwrap(),
                minor: caps[2].parse().unwrap(),
                patch: caps.get(3).map_or(0, |m| m.as_str().parse().unwrap()),
            }),
            None => Err(serde::de::Error::custom(format!(
                "Invalid version (expected X.Y.Z?, got: {})",
                s
            ))),
        }
    }
}

/// Version format for pre-release versions
/// in the form of X.Y.Z-preN or X.Y.Z-rcN
#[derive(Debug, SerializeDisplay, PartialEq, Eq, PartialOrd, Ord)]
pub struct PreReleaseVersion {
    major: u32,
    minor: u32,
    patch: u32,
    pre: String, // /-(pre|rc)\d/
}

impl Display for PreReleaseVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PreReleaseVersion {
                major,
                minor,
                patch: 0,
                pre,
            } => write!(f, "{}.{}-{}", major, minor, pre),
            PreReleaseVersion {
                major,
                minor,
                patch,
                pre,
            } => write!(f, "{}.{}.{}-{}", major, minor, patch, pre),
        }
    }
}

impl<'de> Deserialize<'de> for PreReleaseVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        lazy_static! {
            static ref RE: Regex =
                Regex::new(r"^(\d+)\.(\d+)(?:\.(\d+))?-((?:pre|rc)\d+)$").unwrap();
        }

        match RE.captures(&s) {
            Some(caps) => Ok(PreReleaseVersion {
                major: caps[1].parse().unwrap(),
                minor: caps[2].parse().unwrap(),
                patch: caps.get(3).map_or(0, |m| m.as_str().parse().unwrap()),
                pre: caps[4].to_string(),
            }),
            None => Err(serde::de::Error::custom(format!(
                "Invalid version (expected X.Y.Z?-pre|rcN, got: {})",
                s
            ))),
        }
    }
}

/// Version format for snapshot versions
/// in the form of `XXwYYZ`, where `XX` is the year,
/// `YY` is the week, and `Z` is the iteration (a, b, c, ...)
#[derive(Debug, SerializeDisplay, PartialEq, Eq, PartialOrd, Ord)]
pub struct SnapshotVersion {
    year: u32,         // 13-$currentyear
    week: u32,         // 01-52, probably
    iteration: String, // a, b, c ...
}

impl Display for SnapshotVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:02}w{:02}{}", self.year, self.week, self.iteration)
    }
}

impl<'de> Deserialize<'de> for SnapshotVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        lazy_static! {
            static ref RE: Regex = Regex::new(r"^(\d{2})w(\d{2})([a-z])$").unwrap();
        }
        match RE.captures(&s) {
            Some(caps) => Ok(SnapshotVersion {
                year: caps[1].parse().unwrap(),
                week: caps[2].parse().unwrap(),
                iteration: caps[3].to_string(),
            }),
            None => Err(serde::de::Error::custom(format!(
                "Invalid version (expected XXwYYZ, got: {})",
                s
            ))),
        }
    }
}

/// A version number, which can be one of the following:
/// - Release
/// - PreRelease
/// - Snapshot
/// - Other
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(untagged)]
pub enum VersionNumber {
    Release(ReleaseVersion),
    PreRelease(PreReleaseVersion),
    Snapshot(SnapshotVersion),
    Other(String), // fallback
}

impl FromStr for VersionNumber {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(&format!("\"{}\"", s)).map_err(|e| e.to_string())
    }
}
    


// better way to do this?
impl Display for VersionNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VersionNumber::Release(v) => write!(f, "{}", v),
            VersionNumber::PreRelease(v) => write!(f, "{}", v),
            VersionNumber::Snapshot(v) => write!(f, "{}", v),
            VersionNumber::Other(v) => write!(f, "{}", v),
        }
    }
}

/// A version of the game
///
/// Consists of an ID, a release type, the meta URL, and a release
/// timestamp
#[derive(Debug, Serialize, Deserialize)]
pub struct GameVersion {
    pub id: VersionNumber,
    #[serde(rename = "type")]
    pub release_type: String, // release, snapshot, old_beta, old_alpha. TODO: enum?
    url: String,
    pub time: DateTime<FixedOffset>,
    #[serde(rename = "releaseTime")]
    pub release_time: DateTime<FixedOffset>,
}

/// The latest versions of the game, as returned by the Mojang API
///
/// Includes the latest release and snapshot versions
#[derive(Debug, Serialize, Deserialize)]
struct LatestVersions {
    release: String,
    snapshot: String,
}

/// A list of game versions, as returned by the Mojang API
///
/// Includes the latest versions, and a list of all versions
#[derive(Debug, Serialize, Deserialize)]
pub struct GameVersionList {
    latest: LatestVersions,
    versions: Vec<GameVersion>,
}

impl Iterator for GameVersionList {
    type Item = GameVersion;

    fn next(&mut self) -> Option<Self::Item> {
        self.versions.pop()
    }
}
