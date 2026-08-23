use std::sync::Arc;

use tokio::sync::Mutex;

use crate::{
    Result,
    api::client::{ApiClient, Endpoint},
    cache::{CacheStore, Id},
    models::{DirectMessageChannel, User},
};

pub async fn fetch_dms(
    api_client: &ApiClient,
    cache: Arc<Mutex<CacheStore>>,
) -> Result<Vec<DirectMessageChannel>> {
    let dms_json: Vec<serde_json::Value> = api_client.get(Endpoint::Dms).await?;

    let my_user_id = match api_client
        .get::<serde_json::Value>(Endpoint::CurrentUser)
        .await
    {
        Ok(user_val) => user_val
            .get("_id")
            .or_else(|| user_val.get("id"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        Err(_) => None,
    };

    let mut dm_channels = Vec::new();

    for channel in dms_json {
        let id = channel
            .get("_id")
            .or_else(|| channel.get("id"))
            .and_then(|v| v.as_str());

        let mut display_name = channel
            .get("name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        if display_name.is_none() {
            let channel_type = channel
                .get("channel_type")
                .and_then(|v| v.as_str())
                .unwrap_or_default();

            if channel_type == "SavedMessages" {
                display_name = Some("Saved Messages".to_string());
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
                    } else if let Ok(user_val) = api_client
                        .get::<serde_json::Value>(Endpoint::User(target_id.clone()))
                        .await
                        && let Some(username) = user_val.get("username").and_then(|v| v.as_str())
                    {
                        display_name = Some(username.to_string());

                        if let Ok(uid) = Id::<User>::new(target_id) {
                            let mut cache_locked = cache.lock().await;
                            cache_locked
                                .set(
                                    uid,
                                    &User {
                                        id: target_id.clone(),
                                        username: username.to_string(),
                                    },
                                )
                                .ok();
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
                        if let Ok(user_val) = api_client
                            .get::<serde_json::Value>(Endpoint::User(target_id.clone()))
                            .await
                            && let Some(username) =
                                user_val.get("username").and_then(|v| v.as_str())
                        {
                            display_name = Some(username.to_string());

                            if let Ok(uid) = Id::<User>::new(&target_id) {
                                let mut cache_locked = cache.lock().await;
                                let _ = cache_locked.set(
                                    uid,
                                    &User {
                                        id: target_id.clone(),
                                        username: username.to_string(),
                                    },
                                );
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
            id.map(|s| format!("DM ({s})"))
                .unwrap_or_else(|| "Direct Message".to_string())
        });

        if let Some(id_str) = id {
            dm_channels.push(DirectMessageChannel {
                id: id_str.to_string(),
                name,
            });
        }
    }

    Ok(dm_channels)
}
