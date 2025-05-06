use std::sync::LazyLock;
use std::time::Duration;

use anyhow::Result;
use http_cache_reqwest::{CACacheManager, Cache, CacheMode, HttpCache, HttpCacheOptions};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use serde::de::DeserializeOwned;

static CLIENT: LazyLock<ClientWithMiddleware> = LazyLock::new(|| {
    const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));
    tracing::debug!("init reqwest client with UA `{}`", USER_AGENT);
    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .connect_timeout(Duration::from_secs(3))
        .build()
        .expect("failed to build reqwest client");
    let cache = Cache(HttpCache {
        mode: CacheMode::Default,
        manager: CACacheManager {
            path: "./.cache".into(),
        },
        options: HttpCacheOptions::default(),
    });

    ClientBuilder::new(client).with(cache).build()
});

static RT: LazyLock<tokio::runtime::Runtime> = LazyLock::new(|| {
    tracing::debug!("init tokio runtime");
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("failed to build tokio runtime")
});

/// Fetches a resource either from cache or the internet, returning the parsed JSON response.
pub fn get_cached<T: DeserializeOwned>(uri: &str, mode: Option<CacheMode>) -> Result<T> {
    // fetch
    let response = match mode {
        Some(mode) => RT.block_on(async { CLIENT.get(uri).with_extension(mode).send().await })?,
        None => RT.block_on(async { CLIENT.get(uri).send().await })?,
    };

    // parse from json
    let result = RT.block_on(async { response.json().await })?;

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_json() {
        let uri = "https://example.com";
        let response = RT.block_on(async { CLIENT.get(uri).send().await });

        assert!(response.is_ok());
        assert!(get_cached::<()>(uri, None).is_err_and(|e| e.to_string().contains("decoding")));
    }
}
