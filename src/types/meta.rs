use derive_more::Constructor;
use serde::{Deserialize, Serialize};

use super::version::VersionNumber;

#[derive(Serialize, Deserialize, Constructor)]
pub(crate) struct RunManifest {
    pub id: VersionNumber, // maybe unnecessary
    pub java_version: u8,
    // pub java_args: Vec<String>,
    // pub server_args: Vec<String>,
}
