use anyhow::{Result, anyhow};
use reqwest::Client;
use serde::de::DeserializeOwned;

use crate::api::API_BASE_URL;

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
}

impl Endpoint {
    pub fn path(&self) -> String {
        match self {
            Self::Config => String::from("/"),
            Self::CurrentUser => String::from("/users/@me"),
            Self::User(id) => format!("/users/{}", id),
            Self::Dms => String::from("/users/dms"),
            Self::Server(id) => format!("/servers/{}", id),
            Self::Channel(id) => format!("/channels/{}", id),
            Self::MessageHistory(id) => format!("/channels/{}/messages", id),
            Self::SendMessage(id) => format!("/channels/{}/messages", id),
        }
    }
}

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
            .client
            .get(&url)
            .header("X-Session-Token", &self.token)
            .send()
            .await?;

        if response.status().is_success() {
            let data = response.json::<T>().await?;
            Ok(data)
        } else {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            Err(anyhow!(
                "API GET request to {:?} failed: {} - {}",
                endpoint,
                status,
                text
            ))
        }
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
