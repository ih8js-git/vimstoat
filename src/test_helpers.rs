//! Shared helpers for tests that run against the live Stoat API.

use serde_json::Value;

use crate::api::{auth::Auth, client::ApiClient};

/// Reads the user's real session token from the keyring (read-only).
/// `suite` names the calling test module in panic messages.
pub async fn keyring_token(suite: &str) -> String {
    let auth = Auth::new()
        .unwrap_or_else(|e| panic!("keyring must be available to run {suite} tests: {e}"));

    let token = auth.token_entry.get_secret().await.unwrap_or_else(|e| {
        panic!("{suite} tests need a vimstoat session token stored in the keyring: {e}")
    });

    assert!(
        !token.is_empty(),
        "{suite} tests need a non-empty keyring token"
    );
    token
}

/// An `ApiClient` authenticated with the real keyring token.
pub async fn keyring_client(suite: &str) -> ApiClient {
    ApiClient::new(keyring_token(suite).await, None)
}

/// The string value of `key` in a JSON object.
pub fn json_str<'a>(obj: &'a Value, key: &str) -> &'a str {
    obj.get(key)
        .unwrap_or_else(|| panic!("missing field `{key}`"))
        .as_str()
        .unwrap_or_else(|| panic!("`{key}` is not a string"))
}
