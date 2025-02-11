use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::time::Duration;

use ahash::RandomState;
use anyhow::Result;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use ureq::config::Config;
use ureq::Agent;

// should be tied to crate version, more or less
const CACHE_VERSION: u64 = (0 << 16) | (3 << 8) | 1;
const DEFAULT_TTL: Duration = Duration::from_secs(600);
// this should be deterministic (at least per version per machine) so that cache hits are consistent.
// FxHash works well, may be worth having 2 hashers just for ease of use
const HASHER: RandomState = RandomState::with_seeds(
    CACHE_VERSION,
    !CACHE_VERSION,
    CACHE_VERSION.rotate_right(u64::BITS / 2),
    !CACHE_VERSION.rotate_right(u64::BITS / 2),
);

// TODO: set user agent at compile time (e.g. vergen)
/// Global ureq agent
static AGENT: LazyLock<Agent> = LazyLock::new(|| {
    Config::builder()
        .user_agent("mcdl/0.3.0")
        .timeout_global(Some(std::time::Duration::from_secs(3)))
        .build()
        .into()
});

fn cache_path(uri: &str) -> PathBuf {
    let key = const_hex::encode(HASHER.hash_one(uri).to_ne_bytes());
    let (prefix, _) = key.split_at(2);
    let (first, second) = prefix.split_at(1);
    let subpath = Path::new(first).join(second).join(&key);

    // TODO: dedicated cache directory
    let cache_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(".cache")
        .join(subpath);

    cache_path
}

/// Fetches a resource from the internet, returning the parsed type as well as the http response.
fn fetch_json<T: DeserializeOwned>(path: &str) -> Result<(T, ureq::http::Response<ureq::Body>)> {
    // TODO: make configurable. if this becomes async, can probably push this higher and error immediately when it times out
    let max_duration = Duration::from_millis(500);
    let start_time = std::time::Instant::now();
    let mut attempts = 0;

    loop {
        match AGENT.get(path).call() {
            Ok(mut response) => {
                let result = response.body_mut().read_json()?;
                return Ok((result, response));
            }
            Err(ureq::Error::Timeout(_))
            | Err(ureq::Error::ConnectionFailed)
            | Err(ureq::Error::HostNotFound) => {
                attempts += 1;
                let elapsed = start_time.elapsed();

                if elapsed >= max_duration {
                    return Err(anyhow::anyhow!(
                        "Max retry duration exceeded after {} attempts",
                        attempts
                    ));
                }

                let backoff = Duration::from_millis(25 * 2_u64.pow(attempts));
                let remaining = max_duration.saturating_sub(elapsed);
                let sleep_time = backoff.min(remaining);

                std::thread::sleep(sleep_time);
            }
            Err(e) => return Err(e.into()),
        }
    }
}

/// Fetches a resource either from cache or the internet, returning the parsed JSON response.
///
/// Resources are cached for `ttl` after the first fetch (default 10 minutes if None is provided).
/// If `ttl` is a duration of zero, the resource is fetched immediately and no cache is created.
pub fn get_cached<T: DeserializeOwned + Serialize>(
    path: &str,
    ttl: Option<std::time::Duration>,
) -> Result<T> {
    // bypass cache if ttl is zero
    if ttl.is_some_and(|ttl| ttl.is_zero()) {
        let (result, _) = fetch_json(path)?;
        return Ok(result);
    }

    let cache_path = cache_path(path);

    // check cache
    if let Ok(cached) = Cached::load(&cache_path) {
        if !cached.is_expired() {
            return Ok(cached.into_inner());
        }
    }

    // fetch
    let (result, response) = fetch_json(path)?;

    // parse cache-control header
    let server_ttl = response
        .headers()
        .get(ureq::http::header::CACHE_CONTROL)
        .filter(|h| !h.is_empty())
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.split(',').find(|d| d.trim().starts_with("max-age=")))
        .and_then(|d| d.trim().strip_prefix("max-age="))
        .and_then(|s| s.parse::<u64>().ok())
        .map(Duration::from_secs);

    let age = response
        .headers()
        .get(ureq::http::header::AGE)
        .filter(|h| !h.is_empty())
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
        .map(Duration::from_secs)
        .unwrap_or(Duration::ZERO);

    // server > provided > default
    let adjusted_ttl = server_ttl
        .or(ttl)
        .unwrap_or(DEFAULT_TTL)
        .saturating_sub(age);

    if adjusted_ttl.is_zero() {
        // cache expired, don't bother saving
        return Ok(result);
    }

    // save to cache
    let cached = Cached::new(result, adjusted_ttl);
    cached.save(&cache_path)?;

    Ok(cached.into_inner())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Cached<T> {
    inner: T,
    expiry: std::time::SystemTime,
}

impl<T> Cached<T> {
    fn new(inner: T, ttl: std::time::Duration) -> Self {
        Self {
            inner,
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
        &self.inner
    }

    fn into_inner(self) -> T {
        self.inner
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn cached() {
        let uri = "https://dummyjson.com/quotes";
        let cache_path = dbg!(cache_path(uri));

        let _: serde_json::Value = get_cached(uri, None).unwrap();
        assert!(cache_path.exists());
        let _: serde_json::Value = get_cached(uri, None).unwrap();
    }

    #[test]
    #[should_panic]
    fn not_json() {
        let _: serde_json::Value = get_cached("https://example.com", Some(Duration::ZERO)).unwrap();
    }
}
