use crate::{
    Result,
    api::client::{ApiClient, Endpoint},
    views::error::AuthError,
};
use keyring::KeyringEntry;
use serde_json::Value;

pub struct Auth {
    pub token_entry: KeyringEntry,
}

impl Auth {
    pub fn new() -> Result<Self> {
        let crate_id = "vimstoat";
        let token_entry = KeyringEntry::try_new(crate_id)?;
        Ok(Self { token_entry })
    }

    pub async fn store_token(&self, token: &str) -> Result<()> {
        self.token_entry
            .set_secret(token)
            .await
            .map_err(std::convert::Into::into)
    }

    pub async fn validate_token(&self, token: &str, base_url: Option<String>) -> Result<ApiClient> {
        let client = ApiClient::new(token.to_string(), base_url);

        client
            .get::<Value>(Endpoint::CurrentUser)
            .await
            .map(|_| client)
            .map_err(|e| {
                let err_msg = e.to_string();
                if err_msg.contains("401") {
                    AuthError::InvalidToken(
                        "Please check your session token and try again.".to_string(),
                    )
                    .into()
                } else if err_msg.contains("API GET request") {
                    AuthError::RequestError(err_msg).into()
                } else {
                    AuthError::ServerConnectionError.into()
                }
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::keyring_token;

    /// Builds an `Auth` backed by a test-specific keyring entry.
    /// Never touches the real vimstoat entry.
    fn test_auth(id: &str) -> Auth {
        Auth {
            token_entry: KeyringEntry::try_new(id).expect("Failed to create test keyring entry"),
        }
    }

    #[tokio::test]
    async fn test_validate_token_accepts_real_token() {
        let token = keyring_token("auth").await;
        let auth = Auth::new().unwrap();

        let client = auth
            .validate_token(&token, None)
            .await
            .expect("real keyring token should validate against the Stoat API");

        // Not assert_eq!: on failure it would print the real token.
        assert!(
            client.clone_token() == token,
            "validated client token does not match the keyring token"
        );
    }

    #[tokio::test]
    async fn test_validate_token_rejects_invalid_token() {
        let auth = test_auth("vimstoat_test_invalid_token");

        // Not expect_err: it requires `ApiClient: Debug`, which is deliberately absent.
        let Err(err) = auth
            .validate_token("definitely_not_a_real_session_token", None)
            .await
        else {
            panic!("bogus token should be rejected by the Stoat API");
        };

        assert!(
            matches!(
                err.downcast_ref::<AuthError>(),
                Some(AuthError::InvalidToken(_))
            ),
            "unexpected error: {err:?}"
        );
    }

    #[tokio::test]
    async fn test_real_token_works_with_authenticated_endpoint() {
        let token = keyring_token("auth").await;

        let client = ApiClient::new(token, None);
        let me = client
            .get::<Value>(Endpoint::CurrentUser)
            .await
            .expect("GET /users/@me should succeed with the real token");

        assert!(me.get("_id").is_some(), "no _id in response: {me}");
        assert!(
            me.get("username").is_some(),
            "no username in response: {me}"
        );
    }

    #[tokio::test]
    async fn test_keyring_store_and_get() {
        // Use a test-specific ID so we don't overwrite the actual vimstoat token during tests
        let test_id = "vimstoat_test_keyring";
        let token_entry =
            KeyringEntry::try_new(test_id).expect("Failed to create test keyring entry");
        let auth = Auth { token_entry };

        let test_token = "test_secret_token_12345";

        // Test storing the token
        auth.store_token(test_token)
            .await
            .expect("keyring must be available to run auth tests");

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

    #[tokio::test]
    async fn test_keyring_store_overwrites_existing_token() {
        let auth = test_auth("vimstoat_test_keyring_overwrite");

        auth.store_token("first_token").await.unwrap();
        auth.store_token("second_token").await.unwrap();

        let retrieved = auth.token_entry.get_secret().await.unwrap();
        assert_eq!(retrieved, "second_token");

        let _ = auth.token_entry.delete_secret().await;
    }

    #[tokio::test]
    async fn test_keyring_get_after_delete_fails() {
        let auth = test_auth("vimstoat_test_keyring_delete");

        auth.store_token("temp_token").await.unwrap();
        auth.token_entry.delete_secret().await.unwrap();

        assert!(auth.token_entry.get_secret().await.is_err());
    }
}
