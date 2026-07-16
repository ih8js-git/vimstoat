use crate::api::client::{ApiClient, Endpoint};
use keyring::KeyringEntry;
use serde_json::Value;

pub struct Auth {
    pub token_entry: KeyringEntry,
}

impl Auth {
    pub fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let crate_id = "vimstoat";
        let token_entry = KeyringEntry::try_new(crate_id)?;
        Ok(Self { token_entry })
    }

    pub async fn store_token(&self, token: &str) -> Result<(), String> {
        self.token_entry.set_secret(token).await.map_err(|e| {
            format!(
                "{}\n\nUnderlying Details:\n{:?}\n\n💡 Hint: If you are on a minimal Linux install, you likely need to install a Secret Service provider (e.g., `sudo pacman -S gnome-keyring`).",
                e, e
            )
        })
    }

    pub async fn validate_token(&self, token: &str) -> Result<ApiClient, String> {
        let client = ApiClient::new(token.to_string());

        client
            .get::<Value>(Endpoint::CurrentUser)
            .await
            .map(|_| client)
            .map_err(|e| {
                let err_msg = e.to_string();
                if err_msg.contains("401") {
                    "Invalid token. Please check your session token and try again.".to_string()
                } else if err_msg.contains("API GET request") {
                    err_msg
                } else {
                    "Could not connect to the server. Please check your internet connection."
                        .to_string()
                }
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_keyring_store_and_get() {
        // Use a test-specific ID so we don't overwrite the actual vimstoat token during tests
        let test_id = "vimstoat_test_keyring";
        let token_entry =
            KeyringEntry::try_new(test_id).expect("Failed to create test keyring entry");
        let auth = Auth { token_entry };

        let test_token = "test_secret_token_12345";

        // Test storing the token
        let store_result = auth.store_token(test_token).await;
        // The test might fail on CI or headless systems without a secret service, so we handle it gracefully
        if let Err(e) = store_result {
            println!(
                "Skipping keyring test because the environment doesn't support it: {}",
                e
            );
            return;
        }

        // Test getting the token
        let retrieved_token = auth
            .token_entry
            .get_secret()
            .await
            .expect("Failed to retrieve token from keyring");
        assert_eq!(
            retrieved_token, test_token,
            "Retrieved token did not match stored token"
        );

        // Clean up
        let _ = auth.token_entry.delete_secret().await;
    }
}
