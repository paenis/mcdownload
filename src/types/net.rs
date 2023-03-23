use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(crate) struct CachedResponse<T> {
    pub data: T,
    pub expires: DateTime<Utc>,
}

impl<T> CachedResponse<T> {
    pub fn new(data: T, expires: DateTime<Utc>) -> Self {
        Self { data, expires }
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires
    }
}