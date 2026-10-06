use crate::{
    Result,
    api::client::{ApiClient, Endpoint},
    models::{User, UserStatus},
};
use anyhow::anyhow;

/// Parses a Stoat user JSON payload into a `User` model with status and status_text.
pub fn parse_user(val: &serde_json::Value) -> Option<User> {
    let id = val
        .get("_id")
        .or_else(|| val.get("id"))
        .and_then(|v| v.as_str())?
        .to_string();

    let username = val.get("username").and_then(|v| v.as_str())?.to_string();

    let is_online = val.get("online").and_then(|o| o.as_bool());

    let (status, status_text) = if let Some(status_val) = val.get("status") {
        let configured_presence = status_val
            .get("presence")
            .and_then(|p| p.as_str())
            .map(|p| match p {
                "Online" => UserStatus::Online,
                "Idle" => UserStatus::Idle,
                "Focus" => UserStatus::Focus,
                "Busy" | "DoNotDisturb" => UserStatus::DoNotDisturb,
                "Invisible" => UserStatus::Invisible,
                _ => UserStatus::Offline,
            });

        // If `online` is explicitly false, the user is offline regardless of configured presence
        let presence = match is_online {
            Some(false) => UserStatus::Offline,
            Some(true) => configured_presence.unwrap_or(UserStatus::Online),
            None => configured_presence.unwrap_or(UserStatus::Offline),
        };

        let text = status_val
            .get("text")
            .and_then(|t| t.as_str())
            .map(String::from);

        (presence, text)
    } else {
        (
            if is_online.unwrap_or(false) {
                UserStatus::Online
            } else {
                UserStatus::Offline
            },
            None,
        )
    };

    Some(User {
        id,
        username,
        status,
        status_text,
    })
}

pub async fn fetch_user(api_client: &ApiClient, user_id: &str) -> Result<User> {
    let val: serde_json::Value = api_client.get(Endpoint::User(user_id.to_string())).await?;
    parse_user(&val).ok_or_else(|| anyhow!("Failed to parse user from API response"))
}

pub async fn fetch_current_user(api_client: &ApiClient) -> Result<User> {
    let val: serde_json::Value = api_client.get(Endpoint::CurrentUser).await?;
    parse_user(&val).ok_or_else(|| anyhow!("Failed to parse current user from API response"))
}
