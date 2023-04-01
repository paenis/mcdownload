use derive_more::Constructor;
use serde::{Deserialize, Serialize};

use super::version::VersionNumber;

#[derive(Serialize, Deserialize, Constructor)]
pub(crate) struct RunManifest {
    pub id: VersionNumber,
    pub java_version: u8,
}
