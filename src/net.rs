use std::sync::LazyLock;
use std::time::Duration;

use serde::de::DeserializeOwned;
use thiserror::Error;

static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));
    tracing::trace!("init reqwest client with UA `{}`", USER_AGENT);
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .connect_timeout(Duration::from_secs(3))
        .build()
        .expect("failed to build reqwest client")
});

pub struct HttpClient;

impl HttpClient {
    /// Fetches a resource from the internet, returning the parsed JSON response.
    pub async fn get<T: DeserializeOwned>(uri: &str) -> Result<T, NetError> {
        let result = CLIENT.get(uri).send().await?.json().await?;

        Ok(result)
    }
}

#[derive(Error, Debug)]
pub enum NetError {
    // TODO: better error handling here
    #[error("failed to deserialize response")]
    Deserialize(#[from] reqwest::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn not_json() {
        let uri = "https://example.com";
        let response = CLIENT.get(uri).send().await;

        assert!(response.is_ok());
        match HttpClient::get::<()>(uri).await {
            Err(NetError::Deserialize(e)) if e.is_decode() => {}
            v => panic!("expected decode error, got {v:?}"),
        }
    }
}
