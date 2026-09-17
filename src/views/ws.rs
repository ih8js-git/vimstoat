use log::debug;

use crate::{
    api::{self, events::ServerEvent},
    app::{App, AppEvent},
    models::{Message, User},
};

pub fn handle(app: &mut App, event: ServerEvent) {
    debug!("Received WebSocket event: {event:?}");
    api::ws::EventHandler::new(&mut app.store.servers).handle_event(&event);

    match event {
        ServerEvent::Message(msg_val) => {
            let api_client = app.api_client.clone();
            let app_tx = app.app_tx.clone();
            let local_users = app.store.users.clone();

            tokio::spawn(async move {
                if let Some(channel_id) = msg_val.get("channel").and_then(|v| v.as_str()) {
                    let id = msg_val
                        .get("_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string();
                    let author_id = msg_val
                        .get("author")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown")
                        .to_string();
                    let channel_id = channel_id.to_string();

                    let mut author_name = author_id.clone();
                    let mut new_user_fetched = None;

                    if let Some(user) = local_users.get(&author_id) {
                        author_name = user.username.clone();
                    } else if author_id != "Unknown"
                        && let Ok(user_val) = api_client
                            .get::<serde_json::Value>(crate::api::client::Endpoint::User(
                                author_id.clone(),
                            ))
                            .await
                        && let Some(username) = user_val.get("username").and_then(|v| v.as_str())
                    {
                        author_name = username.to_string();
                        let new_user = User {
                            id: author_id.clone(),
                            username: username.to_string(),
                        };
                        new_user_fetched = Some(new_user);
                    }

                    let content = if let Some(content_val) =
                        msg_val.get("content").and_then(|v| v.as_str())
                    {
                        content_val.to_string()
                    } else if let Some(sys) = msg_val.get("system") {
                        format!(
                            "[System message: {}]",
                            sys.get("type")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown")
                        )
                    } else {
                        "[Unsupported message]".to_string()
                    };

                    let message = Message {
                        id,
                        author_id,
                        author_name,
                        content,
                    };

                    app_tx
                        .send(AppEvent::NewMessage {
                            channel_id,
                            message,
                            new_user: new_user_fetched,
                        })
                        .await
                        .ok();
                }
            });
        }
        ServerEvent::MessageUpdate { id, channel, data } => {
            if let Some(content) = data.get("content").and_then(|v| v.as_str()) {
                app.app_tx
                    .try_send(AppEvent::MessageUpdated {
                        channel_id: channel,
                        message_id: id,
                        content: content.to_string(),
                    })
                    .ok();
            }
        }
        ServerEvent::MessageDelete { id, channel } => {
            app.app_tx
                .try_send(AppEvent::MessageDeleted {
                    channel_id: channel,
                    message_id: id,
                })
                .ok();
        }
        _ => {}
    }
}
