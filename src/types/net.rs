use chrono::{DateTime, Utc};
use derive_more::Constructor;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Constructor)]
pub(crate) struct CachedResponse<T> {
    pub data: T,
    pub expires: DateTime<Utc>,
}

impl<T> CachedResponse<T> {
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires
    }
}
