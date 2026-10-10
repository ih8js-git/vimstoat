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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        cache::Id,
        test_helpers::{json_str, keyring_client},
    };
    use serde_json::Value;
    use tokio::sync::OnceCell;

    /// Presence strings the Stoat API sends; each must be handled in `parse_user`.
    const KNOWN_PRESENCES: [&str; 5] = ["Online", "Idle", "Focus", "Busy", "Invisible"];

    static REAL_ME: OnceCell<Value> = OnceCell::const_new();

    /// The real `/users/@me` payload, fetched once per test run.
    async fn real_me() -> &'static Value {
        REAL_ME
            .get_or_init(|| async {
                keyring_client("user")
                    .await
                    .get::<Value>(Endpoint::CurrentUser)
                    .await
                    .expect("GET /users/@me should succeed with the real token")
            })
            .await
    }

    fn field<'a>(obj: &'a Value, key: &str) -> &'a Value {
        obj.get(key)
            .unwrap_or_else(|| panic!("missing field `{key}`"))
    }

    /// An optional key's value, treating absent and `null` the same.
    fn optional<'a>(obj: &'a Value, key: &str) -> Option<&'a Value> {
        obj.get(key).filter(|v| !v.is_null())
    }

    #[tokio::test]
    async fn test_parse_user_real_payload() {
        let me = real_me().await;

        let user = parse_user(me).expect("real /users/@me payload should parse");

        assert_eq!(user.id, json_str(me, "_id"));
        assert_eq!(user.username, json_str(me, "username"));
    }

    #[tokio::test]
    async fn test_fetch_user_wrappers_agree() {
        let client = keyring_client("user").await;

        let current = fetch_current_user(&client).await.unwrap();
        let by_id = fetch_user(&client, &current.id).await.unwrap();

        assert!(!current.username.is_empty());
        assert_eq!(by_id, current);
    }

    #[tokio::test]
    async fn test_users_me_identity_shape() {
        let me = real_me().await;

        assert!(
            Id::<User>::new(json_str(me, "_id")).is_ok(),
            "`_id` is not a valid id"
        );
        assert!(!json_str(me, "username").is_empty(), "`username` is empty");

        let discriminator = json_str(me, "discriminator");
        assert!(
            discriminator.len() == 4 && discriminator.chars().all(|c| c.is_ascii_digit()),
            "`discriminator` is not 4 digits"
        );
        assert_eq!(json_str(me, "relationship"), "User");
    }

    #[tokio::test]
    async fn test_users_me_presence_shape() {
        let me = real_me().await;

        assert!(field(me, "online").is_boolean(), "`online` is not a bool");

        if let Some(status) = optional(me, "status") {
            assert!(status.is_object(), "`status` is not an object");

            if let Some(presence) = optional(status, "presence") {
                let presence = presence
                    .as_str()
                    .expect("`status.presence` is not a string");
                assert!(
                    KNOWN_PRESENCES.contains(&presence),
                    "`status.presence` {presence:?} is not handled by parse_user"
                );
            }
            if let Some(text) = optional(status, "text") {
                assert!(text.is_string(), "`status.text` is not a string");
            }
        }
    }

    #[tokio::test]
    async fn test_users_me_avatar_shape() {
        let me = real_me().await;

        // Avatar is optional; an account without one passes vacuously.
        let Some(avatar) = optional(me, "avatar") else {
            return;
        };

        for key in ["_id", "tag", "filename", "content_type"] {
            assert!(!json_str(avatar, key).is_empty(), "`avatar.{key}` is empty");
        }
        assert!(
            field(avatar, "size").is_u64(),
            "`avatar.size` is not a number"
        );
        json_str(field(avatar, "metadata"), "type");
    }

    #[tokio::test]
    async fn test_users_me_relations_shape() {
        let me = real_me().await;

        let Some(relations) = optional(me, "relations") else {
            return;
        };
        let relations = relations.as_array().expect("`relations` is not an array");

        for relation in relations {
            assert!(
                Id::<User>::new(json_str(relation, "_id")).is_ok(),
                "`relations[]._id` is not a valid id"
            );
            assert!(
                !json_str(relation, "status").is_empty(),
                "`relations[].status` is empty"
            );
        }
    }
}
