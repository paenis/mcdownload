use std::hash::BuildHasher;
use std::io::Write;
use std::path::Path;
use std::sync::LazyLock;

use anyhow::Result;
use rustc_hash::FxBuildHasher;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use ureq::config::Config;
use ureq::Agent;

// TODO: set user agent at compile time (e.g. vergen)
static AGENT: LazyLock<Agent> = LazyLock::new(|| {
    Config::builder()
        .user_agent("mcdl/0.3.0")
        .timeout_global(Some(std::time::Duration::from_secs(5)))
        .build()
        .into()
});

/// Fetches a resource from the internet, returning the parsed JSON response.
fn get<T: DeserializeOwned>(path: &str) -> Result<T> {
    Ok(AGENT.get(path).call()?.body_mut().read_json()?)
}

/// Fetches a resource either from cache or the internet, returning the parsed JSON response.
///
/// Resources are cached for `ttl` after the first fetch (default 10 minutes if None is provided).
pub fn get_cached<T: DeserializeOwned + Serialize>(
    path: &str,
    ttl: Option<std::time::Duration>,
) -> Result<T> {
    let key = format!("{:016x}", FxBuildHasher.hash_one(path));
    let (prefix, _) = key.split_at(2);
    let (first, second) = prefix.split_at(1);
    let subpath = Path::new(first).join(second).join(&key);

    // TODO: dedicated cache directory
    let cache_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(".cache")
        .join(subpath);

    // check cache
    if let Ok(cached) = Cached::load(&cache_path) {
        if !cached.is_expired() {
            return Ok(cached.into_inner());
        }
    }

    // fetch
    let result: T = get(path)?;

    // save to cache, default 10 minutes
    let cached = Cached::new(
        result,
        ttl.unwrap_or_else(|| std::time::Duration::from_secs(60 * 10)),
    );
    cached.save(&cache_path)?;

    Ok(cached.into_inner())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Cached<T> {
    inner: Box<T>,
    expiry: std::time::SystemTime,
}

impl<T> Cached<T> {
    fn new(inner: T, ttl: std::time::Duration) -> Self {
        Self {
            inner: Box::new(inner),
            expiry: std::time::SystemTime::now() + ttl,
        }
    }

    fn is_expired(&self) -> bool {
        std::time::SystemTime::now() > self.expiry
    }

    fn save(&self, path: impl AsRef<Path>) -> Result<()>
    where Self: Serialize {
        let path = path.as_ref();
        let parent = path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("invalid path"))?;
        let data = serde_json::to_vec(self)?;

        if !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
        let mut file = std::fs::File::create(path)?; // truncate if exists
        file.write_all(&data)?;
        Ok(())
    }

    fn load(path: impl AsRef<Path>) -> Result<Self>
    where T: DeserializeOwned {
        let path = path.as_ref();
        let data = std::fs::read(path)?;
        Ok(serde_json::from_slice(&data)?)
    }

    fn inner(&self) -> &T {
        self.inner.as_ref()
    }

    fn into_inner(self) -> T {
        *self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cached() {
        let _: serde_json::Value = get_cached("https://dummyjson.com/quotes", None).unwrap();
    }
}
