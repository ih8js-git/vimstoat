use crate::{
    Result,
    api::client::{ApiClient, Endpoint},
    models::{DirectMessageChannel, User},
};

pub async fn fetch_unreads(api_client: &ApiClient) -> Result<Vec<serde_json::Value>> {
    api_client.get(Endpoint::SyncUnreads).await
}

pub async fn fetch_dms(
    api_client: &ApiClient,
    known_users: &std::collections::HashMap<String, User>,
) -> Result<(Vec<DirectMessageChannel>, Vec<User>)> {
    let dms_json: Vec<serde_json::Value> = api_client.get(Endpoint::Dms).await?;

    let (unreads_map, unreads_fetched) = match fetch_unreads(api_client).await {
        Ok(unreads) => {
            let mut map = std::collections::HashMap::new();
            for unread in unreads {
                let channel_id_opt = unread
                    .get("_id")
                    .and_then(|v| {
                        if let Some(obj) = v.as_object() {
                            obj.get("channel").and_then(|c| c.as_str())
                        } else {
                            v.as_str()
                        }
                    })
                    .or_else(|| unread.get("channel").and_then(|c| c.as_str()))
                    .or_else(|| unread.get("channel_id").and_then(|c| c.as_str()));

                if let Some(ch_id) = channel_id_opt {
                    map.insert(ch_id.to_string(), unread);
                }
            }
            (map, true)
        }
        Err(e) => {
            log::warn!("Could not fetch unreads: {e}");
            (std::collections::HashMap::new(), false)
        }
    };

    let my_user_id = match api_client
        .get::<serde_json::Value>(Endpoint::CurrentUser)
        .await
    {
        Ok(user_val) => user_val
            .get("_id")
            .or_else(|| user_val.get("id"))
            .and_then(|v| v.as_str())
            .map(std::string::ToString::to_string),
        Err(_) => None,
    };

    let mut dm_channels = Vec::new();
    let mut new_users = Vec::new();

    for channel in dms_json {
        let id = channel
            .get("_id")
            .or_else(|| channel.get("id"))
            .and_then(|v| v.as_str());

        let mut recipient_id: Option<String> = None;

        let mut display_name = channel
            .get("name")
            .and_then(|v| v.as_str())
            .map(std::string::ToString::to_string);

        if display_name.is_none() {
            let channel_type = channel
                .get("channel_type")
                .and_then(|v| v.as_str())
                .unwrap_or_default();

            if channel_type == "SavedMessages" {
                display_name = Some("Saved Messages".to_string());
                recipient_id = my_user_id.clone();
            } else {
                let mut user_ids: Vec<String> = Vec::new();

                if let Some(recipients_arr) = channel.get("recipients").and_then(|v| v.as_array()) {
                    for v in recipients_arr {
                        let id_opt = v.as_str().or_else(|| {
                            v.get("_id")
                                .or_else(|| v.get("id"))
                                .and_then(|i| i.as_str())
                        });
                        if let Some(id_str) = id_opt
                            && !user_ids.contains(&id_str.to_string())
                        {
                            user_ids.push(id_str.to_string());
                        }
                    }
                }

                for key in &["recipient", "user", "user_id"] {
                    let id_opt = channel.get(*key).and_then(|v| {
                        v.as_str().or_else(|| {
                            v.get("_id")
                                .or_else(|| v.get("id"))
                                .and_then(|i| i.as_str())
                        })
                    });
                    if let Some(id_str) = id_opt
                        && !user_ids.contains(&id_str.to_string())
                    {
                        user_ids.push(id_str.to_string());
                    }
                }

                if user_ids.len() == 1 {
                    let target_id = &user_ids[0];
                    if Some(target_id) == my_user_id.as_ref() {
                        display_name = Some("Saved Messages".to_string());
                        recipient_id = my_user_id.clone();
                    } else {
                        recipient_id = Some(target_id.clone());
                        if let Some(user) = known_users.get(target_id) {
                            display_name = Some(user.username.clone());
                        }

                        if display_name.is_none()
                            && let Ok(user) =
                                crate::api::user::fetch_user(api_client, target_id).await
                        {
                            display_name = Some(user.username.clone());
                            new_users.push(user);
                        }
                    }
                    if display_name.is_none() {
                        display_name = Some(target_id.clone());
                    }
                } else if user_ids.len() == 2 {
                    let other_id = if let Some(my_id) = &my_user_id {
                        user_ids.iter().find(|id| *id != my_id).cloned()
                    } else {
                        user_ids.first().cloned()
                    };

                    if let Some(target_id) = other_id {
                        recipient_id = Some(target_id.clone());
                        if let Some(user) = known_users.get(&target_id) {
                            display_name = Some(user.username.clone());
                        }

                        if display_name.is_none()
                            && let Ok(user) =
                                crate::api::user::fetch_user(api_client, &target_id).await
                        {
                            display_name = Some(user.username.clone());
                            new_users.push(user);
                        }
                        if display_name.is_none() {
                            display_name = Some(target_id);
                        }
                    }
                } else if user_ids.len() >= 3 {
                    display_name = Some(format!("Group DM ({} members)", user_ids.len()));
                }
            }
        }

        let name = display_name.unwrap_or_else(|| {
            id.map_or_else(|| "Direct Message".to_string(), |s| format!("DM ({s})"))
        });

        let last_message_id = channel
            .get("last_message_id")
            .or_else(|| channel.get("last_message"))
            .and_then(|v| v.as_str());

        let has_unread = match last_message_id {
            None => false,
            Some(latest) => {
                if !unreads_fetched {
                    false
                } else if let Some(id_str) = id
                    && let Some(unread) = unreads_map.get(id_str)
                {
                    let has_mentions = unread
                        .get("mentions")
                        .and_then(|m| m.as_array())
                        .is_some_and(|m| !m.is_empty());

                    if has_mentions {
                        true
                    } else {
                        match unread.get("last_id").and_then(|v| v.as_str()) {
                            Some(last_read) => latest > last_read,
                            None => true,
                        }
                    }
                } else {
                    true
                }
            }
        };

        if let Some(id_str) = id {
            dm_channels.push(DirectMessageChannel {
                id: id_str.to_string(),
                name,
                recipient_id,
                last_message_id: last_message_id.map(std::string::ToString::to_string),
                has_unread,
                typing_users: std::collections::HashSet::new(),
            });
        }
    }

    Ok((dm_channels, new_users))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        api::user::fetch_current_user,
        test_helpers::{json_str, keyring_client},
    };
    use serde_json::Value;
    use std::collections::HashMap;
    use tokio::sync::OnceCell;

    /// Everything the tests compare against, fetched once per test run.
    struct Live {
        client: ApiClient,
        my_id: String,
        raw_dms: Vec<Value>,
        dms: Vec<DirectMessageChannel>,
        new_users: Vec<User>,
    }

    static LIVE: OnceCell<Live> = OnceCell::const_new();

    async fn live() -> &'static Live {
        LIVE.get_or_init(|| async {
            let client = keyring_client("dm").await;
            let my_id = fetch_current_user(&client)
                .await
                .expect("GET /users/@me should succeed with the real token")
                .id;
            let raw_dms = client
                .get::<Vec<Value>>(Endpoint::Dms)
                .await
                .expect("GET /users/dms should succeed with the real token");
            let (dms, new_users) = fetch_dms(&client, &HashMap::new())
                .await
                .expect("fetch_dms should succeed with the real token");

            Live {
                client,
                my_id,
                raw_dms,
                dms,
                new_users,
            }
        })
        .await
    }

    fn channel_type(raw: &Value) -> &str {
        json_str(raw, "channel_type")
    }

    fn parsed<'a>(live: &'a Live, raw: &Value) -> &'a DirectMessageChannel {
        let id = json_str(raw, "_id");
        live.dms
            .iter()
            .find(|dm| dm.id == id)
            .unwrap_or_else(|| panic!("channel {id} from /users/dms missing from fetch_dms"))
    }

    /// The single recipient of a one-to-one DM that is not the current user.
    fn other_recipient<'a>(raw: &'a Value, my_id: &str) -> &'a str {
        let recipients: Vec<&str> = raw
            .get("recipients")
            .and_then(Value::as_array)
            .expect("DirectMessage channel has no `recipients` array")
            .iter()
            .map(|r| r.as_str().expect("`recipients[]` is not a string"))
            .collect();
        let others: Vec<&str> = recipients.into_iter().filter(|r| *r != my_id).collect();
        assert_eq!(
            others.len(),
            1,
            "DirectMessage should have exactly one other recipient"
        );
        others[0]
    }

    #[tokio::test]
    async fn test_fetch_dms_returns_every_channel_once() {
        let live = live().await;

        let mut raw_ids: Vec<&str> = live.raw_dms.iter().map(|c| json_str(c, "_id")).collect();
        let mut parsed_ids: Vec<&str> = live.dms.iter().map(|dm| dm.id.as_str()).collect();
        raw_ids.sort_unstable();
        parsed_ids.sort_unstable();

        assert_eq!(parsed_ids, raw_ids);
    }

    #[tokio::test]
    async fn test_fetch_dms_preserves_last_message_id() {
        let live = live().await;

        for raw in &live.raw_dms {
            let expected = raw
                .get("last_message_id")
                .and_then(Value::as_str)
                .map(str::to_string);
            assert_eq!(parsed(live, raw).last_message_id, expected);
        }
    }

    #[tokio::test]
    async fn test_saved_messages_points_at_current_user() {
        let live = live().await;

        for raw in live
            .raw_dms
            .iter()
            .filter(|c| channel_type(c) == "SavedMessages")
        {
            let dm = parsed(live, raw);
            assert_eq!(dm.name, "Saved Messages");
            assert_eq!(dm.recipient_id.as_deref(), Some(live.my_id.as_str()));
        }
    }

    #[tokio::test]
    async fn test_direct_message_resolves_other_user() {
        let live = live().await;

        for raw in live
            .raw_dms
            .iter()
            .filter(|c| channel_type(c) == "DirectMessage")
        {
            let dm = parsed(live, raw);
            let other_id = other_recipient(raw, &live.my_id);

            assert_eq!(dm.recipient_id.as_deref(), Some(other_id));

            // Reuse the users fetch_dms fetched; refetching here trips the rate limit.
            let other = live
                .new_users
                .iter()
                .find(|u| u.id == other_id)
                .unwrap_or_else(|| panic!("recipient of DM {} was not fetched", dm.id));
            assert_eq!(dm.name, other.username, "DM {} named wrongly", dm.id);
        }
    }

    #[tokio::test]
    async fn test_group_uses_channel_name() {
        let live = live().await;

        for raw in live.raw_dms.iter().filter(|c| channel_type(c) == "Group") {
            assert_eq!(parsed(live, raw).name, json_str(raw, "name"));
        }
    }

    #[tokio::test]
    async fn test_new_users_are_exactly_the_fetched_recipients() {
        let live = live().await;

        let mut expected: Vec<&str> = live
            .raw_dms
            .iter()
            .filter(|c| channel_type(c) == "DirectMessage")
            .map(|c| other_recipient(c, &live.my_id))
            .collect();
        let mut returned: Vec<&str> = live.new_users.iter().map(|u| u.id.as_str()).collect();
        expected.sort_unstable();
        returned.sort_unstable();

        assert_eq!(returned, expected);
        assert!(
            live.new_users.iter().all(|u| u.id != live.my_id),
            "current user should never be returned as a new user"
        );
    }

    #[tokio::test]
    async fn test_known_users_skip_fetch_and_drive_names() {
        let live = live().await;

        // Rename every known user so we can tell the cache was used over the API.
        let known: HashMap<String, User> = live
            .new_users
            .iter()
            .map(|u| {
                let mut cached = u.clone();
                cached.username = format!("cached-{}", u.id);
                (u.id.clone(), cached)
            })
            .collect();

        let (dms, new_users) = fetch_dms(&live.client, &known).await.unwrap();

        assert!(new_users.is_empty(), "known users should not be refetched");
        for dm in &dms {
            if let Some(user) = dm.recipient_id.as_ref().and_then(|id| known.get(id)) {
                assert_eq!(dm.name, user.username);
            }
        }
    }

    #[tokio::test]
    async fn test_unreads_are_keyed_by_channel() {
        let live = live().await;
        let unreads = fetch_unreads(&live.client)
            .await
            .expect("GET /sync/unreads should succeed with the real token");

        // fetch_dms matches unreads to channels via `_id.channel`; if the API
        // shape drifts, every DM silently shows as unread.
        for unread in &unreads {
            let key = unread.get("_id").expect("unread has no `_id`");
            json_str(key, "channel");
            assert_eq!(json_str(key, "user"), live.my_id);
        }
    }

    #[tokio::test]
    async fn test_has_unread_matches_read_state() {
        let live = live().await;
        let unreads = fetch_unreads(&live.client).await.unwrap();
        let by_channel: HashMap<&str, &Value> = unreads
            .iter()
            .map(|u| (json_str(&u["_id"], "channel"), u))
            .collect();

        for dm in &live.dms {
            let Some(latest) = dm.last_message_id.as_deref() else {
                assert!(!dm.has_unread, "DM {} with no messages is unread", dm.id);
                continue;
            };
            let Some(unread) = by_channel.get(dm.id.as_str()) else {
                continue;
            };
            let has_mentions = unread["mentions"].as_array().is_some_and(|m| !m.is_empty());
            let caught_up = unread["last_id"]
                .as_str()
                .is_some_and(|last| last >= latest);

            if has_mentions {
                assert!(dm.has_unread, "DM {} with mentions is not unread", dm.id);
            } else if caught_up {
                assert!(!dm.has_unread, "fully read DM {} is unread", dm.id);
            }
        }
    }

    #[tokio::test]
    async fn test_fetch_dms_rejects_invalid_token() {
        let client = ApiClient::new("definitely_not_a_real_session_token".to_string(), None);

        assert!(fetch_dms(&client, &HashMap::new()).await.is_err());
    }
}
