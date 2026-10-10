use std::time::Duration;

use anyhow::{Result, anyhow};
use reqwest::{Client, RequestBuilder, Response, StatusCode};
use serde::de::DeserializeOwned;

use crate::api::API_BASE_URL;

/// How many times a rate-limited request is retried in test builds.
const TEST_RATE_LIMIT_RETRIES: usize = 3;
/// Wait used when a 429 response carries no `retry_after`.
const DEFAULT_RETRY_AFTER_MS: u64 = 1000;

#[derive(Debug)]
#[allow(unused)]
pub enum Endpoint {
    Config,
    CurrentUser,
    User(String),
    Dms,
    Server(String),
    Channel(String),
    MessageHistory(String),
    SendMessage(String),
    SyncUnreads,
    AckMessage {
        channel_id: String,
        message_id: String,
    },
    Custom(String),
}

impl Endpoint {
    pub fn path(&self) -> String {
        match self {
            Self::Config => String::from("/"),
            Self::CurrentUser => String::from("/users/@me"),
            Self::User(id) => format!("/users/{id}"),
            Self::Dms => String::from("/users/dms"),
            Self::Server(id) => format!("/servers/{id}"),
            Self::Channel(id) => format!("/channels/{id}"),
            Self::MessageHistory(id) => format!("/channels/{id}/messages"),
            Self::SendMessage(id) => format!("/channels/{id}/messages"),
            Self::SyncUnreads => String::from("/sync/unreads"),
            Self::AckMessage {
                channel_id,
                message_id,
            } => format!("/channels/{channel_id}/ack/{message_id}"),
            Self::Custom(path) => path.clone(),
        }
    }
}

// SECURITY: intentionally does NOT implement `Debug`. This type holds the user's
// session token, and a `{:?}`, `dbg!`, or derived `Debug` on a parent struct would
// write it to the log file or terminal. Do not add `Debug` here (or derive it on any
// type that contains an `ApiClient`) unless the token is redacted.
#[derive(Clone)]
pub struct ApiClient {
    client: Client,
    token: String,
    base_url: String,
}

impl ApiClient {
    pub fn new(token: String, base_url: Option<String>) -> Self {
        Self {
            client: Client::new(),
            token,
            base_url: base_url.unwrap_or(API_BASE_URL.to_string()),
        }
    }

    /// Makes a GET request to the specified endpoint and deserializes the JSON response into `T`.
    /// The `endpoint` should start with a slash, e.g., `/users/@me`.
    pub async fn get<T: DeserializeOwned>(&self, endpoint: Endpoint) -> Result<T> {
        let url = format!("{}{}", self.base_url, endpoint.path());

        let response = self
            .send(|| self.client.get(&url).header("X-Session-Token", &self.token))
            .await?;

        if response.status().is_success() {
            let data = response.json::<T>().await?;
            Ok(data)
        } else {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            Err(anyhow!(
                "API GET request to {endpoint:?} failed: {status} - {text}"
            ))
        }
    }

    pub async fn post<T: DeserializeOwned, B: serde::Serialize>(
        &self,
        endpoint: Endpoint,
        body: &B,
    ) -> Result<T> {
        let url = format!("{}{}", self.base_url, endpoint.path());

        let response = self
            .send(|| {
                self.client
                    .post(&url)
                    .header("X-Session-Token", &self.token)
                    .json(body)
            })
            .await?;

        if response.status().is_success() {
            let data = response.json::<T>().await?;
            Ok(data)
        } else {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            Err(anyhow!(
                "API POST request to {endpoint:?} failed: {status} - {text}"
            ))
        }
    }

    pub async fn put_empty(&self, endpoint: Endpoint) -> Result<()> {
        let url = format!("{}{}", self.base_url, endpoint.path());

        let response = self
            .send(|| self.client.put(&url).header("X-Session-Token", &self.token))
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            Err(anyhow!(
                "API PUT request to {endpoint:?} failed: {status} - {text}"
            ))
        }
    }

    /// Sends the request produced by `build`. In test builds, a 429 is retried
    /// after the server's `retry_after` delay so live tests wait out the rate
    /// limit instead of failing; the last attempt's response is returned as-is.
    async fn send(&self, build: impl Fn() -> RequestBuilder) -> reqwest::Result<Response> {
        if cfg!(test) {
            for _ in 0..TEST_RATE_LIMIT_RETRIES {
                let response = build().send().await?;
                if response.status() != StatusCode::TOO_MANY_REQUESTS {
                    return Ok(response);
                }

                let body = response
                    .json::<serde_json::Value>()
                    .await
                    .unwrap_or_default();
                let wait_ms = body["retry_after"]
                    .as_u64()
                    .unwrap_or(DEFAULT_RETRY_AFTER_MS);
                tokio::time::sleep(Duration::from_millis(wait_ms)).await;
            }
        }

        build().send().await
    }

    pub fn clone_token(&self) -> String {
        self.token.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[tokio::test]
    async fn test_get_config() {
        // Token doesn't matter for the root config endpoint, but we provide a dummy one
        let client = ApiClient::new("dummy_token".to_string(), None);

        let result = client.get::<Value>(Endpoint::Config).await;
        assert!(result.is_ok(), "Failed to get config: {:?}", result.err());

        let data = result.unwrap();
        assert!(
            data.get("revolt").is_some(),
            "Response did not contain 'revolt' key"
        );
    }
}
