use keyring::KeyringEntry;
use serde::Deserialize;

const BASE_URL: &str = "https://api.stoat.chat";

#[derive(Deserialize)]
pub struct UserInfo {
    pub username: String,
    pub display_name: Option<String>,
}

impl UserInfo {
    /// Returns the display name if set, otherwise falls back to the username.
    pub fn name(&self) -> &str {
        self.display_name.as_deref().unwrap_or(&self.username)
    }
}

pub struct Auth {
    pub token_entry: KeyringEntry,
}

impl Auth {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
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

    /// Validates a token by calling `GET /users/@me` with the `X-Session-Token` header.
    /// Returns the user's info if the token is valid, or an error message if it's invalid.
    pub async fn validate_token(&self, token: &str) -> Result<UserInfo, String> {
        let client = reqwest::Client::new();
        let response = client
            .get(format!("{}/users/@me", BASE_URL))
            .header("X-Session-Token", token)
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;

        if response.status().is_success() {
            response
                .json::<UserInfo>()
                .await
                .map_err(|e| format!("Failed to parse user info: {}", e))
        } else if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            Err("Invalid token. Please check your session token and try again.".to_string())
        } else {
            Err(format!(
                "Unexpected API response: {} {}",
                response.status().as_u16(),
                response.status().canonical_reason().unwrap_or("Unknown")
            ))
        }
    }
}
