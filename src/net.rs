use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::time::{Duration, SystemTime};

use ahash::RandomState;
use anyhow::Result;
use backon::{BackoffBuilder, BlockingRetryable, FibonacciBuilder};
use bincode::{Decode, Encode};
use serde::de::DeserializeOwned;
use ureq::config::Config;
use ureq::{Agent, http};

// should be tied to crate version, more or less
const CACHE_VERSION: u64 = (0 << 16) | (3 << 8) | 2;
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
        .user_agent(concat!("mcdl/", env!("CARGO_PKG_VERSION")))
        .timeout_global(Some(Duration::from_secs(3)))
        .build()
        .into()
});

fn cache_path(uri: &str) -> PathBuf {
    let key = const_hex::encode(HASHER.hash_one(uri).to_ne_bytes());
    let (prefix, _) = key.split_at(2);
    let (first, second) = prefix.split_at(1);
    let subpath = Path::new(first).join(second).join(&key);

    // TODO: dedicated cache directory
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(".cache")
        .join(subpath)
}

/// Fetches a resource from the internet, returning the HTTP response.
///
/// Will retry on error using the provided backoff strategy.
fn fetch_remote_retry(
    uri: &str,
    strategy: impl BackoffBuilder,
) -> Result<http::Response<ureq::Body>> {
    let result = (|| AGENT.get(uri).call())
        .retry(strategy)
        .notify(|e, d| println!("sleeping for {d:?}: {e}"))
        .when(|e| {
            // retry possibly transient errors
            match dbg!(e) {
                ureq::Error::ConnectionFailed | ureq::Error::HostNotFound => true,
                ureq::Error::StatusCode(n) if *n >= 500 => true,
                _ => false,
            }
        })
        .call()?;

    Ok(result)
}

/// Fetches a resource either from cache or the internet, returning the parsed JSON response.
///
/// Resources are cached for `ttl` after the first fetch (default 10 minutes if None is provided).
/// If `ttl` is a duration of zero, the resource is fetched immediately and no cache is created.
pub fn get_cached<T: DeserializeOwned + Decode<()> + Encode>(
    uri: &str,
    ttl: Option<Duration>,
) -> Result<T> {
    // TODO: use `http-cache-semantics`?
    // use #[io_cached] from `cached`?
    // just give up and use `reqwest` with middleware?

    // bypass cache if ttl is zero
    if ttl.is_some_and(|ttl| ttl.is_zero()) {
        let mut response = fetch_remote_retry(
            uri,
            FibonacciBuilder::default().with_min_delay(Duration::from_millis(200)),
        )?;
        return Ok(response.body_mut().read_json()?);
    }

    let cache_path = cache_path(uri);

    // check cache
    if let Ok(cached) = Cached::load(&cache_path) {
        if !cached.is_expired() {
            return Ok(cached.into_inner());
        }
    }

    // fetch
    let mut response = fetch_remote_retry(
        uri,
        FibonacciBuilder::default().with_min_delay(Duration::from_millis(200)),
    )?;

    // parse cache-control header
    let server_ttl = response
        .headers()
        .get(http::header::CACHE_CONTROL)
        .filter(|h| !h.is_empty())
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.split(',').find(|d| d.trim().starts_with("max-age=")))
        .and_then(|d| d.trim().strip_prefix("max-age="))
        .and_then(|s| s.parse::<u64>().ok())
        .map(Duration::from_secs);

    let age = response
        .headers()
        .get(http::header::AGE)
        .filter(|h| !h.is_empty())
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
        .map_or(Duration::ZERO, Duration::from_secs);

    // server > provided > default
    let adjusted_ttl = server_ttl
        .or(ttl)
        .unwrap_or(DEFAULT_TTL)
        .saturating_sub(age);

    // parse from json
    let result = response.body_mut().read_json()?;

    if adjusted_ttl.is_zero() {
        // cache expired, don't bother saving
        return Ok(result);
    }

    // save to cache
    let cached = Cached::new(result, adjusted_ttl);
    cached.save(&cache_path)?;

    Ok(cached.into_inner())
}

#[derive(Debug, Clone, Encode, Decode)]
struct Cached<T> {
    inner: T,
    expiry: SystemTime,
}

impl<T> Cached<T> {
    fn new(inner: T, ttl: Duration) -> Self {
        Self {
            inner,
            expiry: SystemTime::now() + ttl,
        }
    }

    fn is_expired(&self) -> bool {
        SystemTime::now() > self.expiry
    }

    fn save(&self, path: impl AsRef<Path>) -> Result<()>
    where Self: Encode {
        let path = path.as_ref();
        let parent = path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("invalid path"))?;

        if !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }

        let mut writer = BufWriter::new(File::create(path)?); // truncate if exists
        bincode::encode_into_std_write(self, &mut writer, bincode::config::standard())?;
        Ok(())
    }

    fn load(path: impl AsRef<Path>) -> Result<Self>
    where T: Decode<()> {
        let path = path.as_ref();
        let mut reader = BufReader::new(File::open(path)?);
        Ok(bincode::decode_from_std_read(
            &mut reader,
            bincode::config::standard(),
        )?)
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
    use super::*;

    #[test]
    fn not_json() {
        let uri = "https://example.com";
        assert!(fetch_remote_retry(uri, FibonacciBuilder::default()).is_ok());
        assert!(
            get_cached::<()>(uri, Some(Duration::ZERO))
                .is_err_and(|e| e.to_string().contains("json: "))
        );
    }
}
