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

                        if display_name.is_none() {
                            if let Ok(user) =
                                crate::api::user::fetch_user(api_client, target_id).await
                            {
                                display_name = Some(user.username.clone());
                                new_users.push(user);
                            }
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

                        if display_name.is_none() {
                            if let Ok(user) =
                                crate::api::user::fetch_user(api_client, &target_id).await
                            {
                                display_name = Some(user.username.clone());
                                new_users.push(user);
                            }
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
                has_unread,
                typing_users: std::collections::HashSet::new(),
            });
        }
    }

    Ok((dm_channels, new_users))
}
