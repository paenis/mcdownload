use std::sync::LazyLock;
use std::time::Duration;

use anyhow::Result;
use http_cache_reqwest::{CACacheManager, Cache, CacheMode, HttpCache, HttpCacheOptions};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use serde::de::DeserializeOwned;

static CLIENT: LazyLock<ClientWithMiddleware> = LazyLock::new(|| {
    const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));
    tracing::trace!("init reqwest client with UA `{}`", USER_AGENT);
    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .connect_timeout(Duration::from_secs(3))
        .build()
        .expect("failed to build reqwest client");
    let cache = Cache(HttpCache {
        mode: CacheMode::Default,
        manager: CACacheManager::new("./.cache".into(), false),
        options: HttpCacheOptions::default(),
    });

    ClientBuilder::new(client).with(cache).build()
});

/// Fetches a resource either from cache or the internet, returning the parsed JSON response.
pub async fn get_cached<T: DeserializeOwned>(uri: &str, mode: Option<CacheMode>) -> Result<T> {
    // fetch
    let response = match mode {
        Some(mode) => CLIENT.get(uri).with_extension(mode).send().await?,
        None => CLIENT.get(uri).send().await?,
    };

    // parse from json
    let result = response.json().await?;

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn not_json() {
        let uri = "https://example.com";
        let response = CLIENT.get(uri).send().await;

        assert!(response.is_ok());
        assert!(
            get_cached::<()>(uri, None)
                .await
                .is_err_and(|e| e.to_string().contains("decoding"))
        );
    }
}
