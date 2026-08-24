use log::error;
use ratatui::crossterm::event::KeyEvent;

use crate::{
    action::Action,
    app::{App, AppEvent, AppState},
    input::InputMode,
};

pub fn handle(app: &mut App, key: KeyEvent) {
    let action = app.input_state.process_key_event(key);
    match action {
        Some(Action::Quit) => app.should_quit = true,
        Some(Action::EnterCommandMode) => {
            app.command_text.clear();
            app.set_input_mode(InputMode::Command);
        }
        Some(Action::CursorUp) => {
            if app.selected_dm_index > 0 {
                app.selected_dm_index -= 1;
            }
        }
        Some(Action::CursorDown) => {
            let total_items = app.store.dm_channels.len();
            if total_items > 0 && app.selected_dm_index + 1 < total_items {
                app.selected_dm_index += 1;
            }
        }
        Some(Action::GoToTopUI) => {
            app.selected_dm_index = 0;
        }
        Some(Action::Enter) if !app.store.dm_channels.is_empty() => {
            let channel_id = app.store.dm_channels[app.selected_dm_index].id.clone();
            app.store.dm_channels[app.selected_dm_index].has_unread = false;
            app.state = AppState::Dm;
            app.is_loading_messages = true;
            app.store.current_dm_messages.clear();
            app.input_text.clear();

            let api_client = app.api_client.clone();
            let app_tx = app.app_tx.clone();
            let users = app.store.users.clone();

            tokio::spawn(async move {
                let query = crate::api::channel::MessageHistoryQuery {
                    limit: Some(50),
                    before: None,
                    after: None,
                    sort: None,
                    nearby: None,
                };
                match crate::api::channel::fetch_message_history(
                    &api_client,
                    &channel_id,
                    Some(&query),
                )
                .await
                {
                    Ok(messages_json) => {
                        let mut parsed_messages = Vec::with_capacity(messages_json.len());
                        let mut new_users_fetched = Vec::new();
                        let mut local_users = users.clone();

                        for msg in messages_json {
                            let id = msg
                                .get("_id")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown")
                                .to_string();

                            let author_id = msg
                                .get("author")
                                .and_then(|v| v.as_str())
                                .unwrap_or("Unknown")
                                .to_string();

                            let mut author_name = author_id.clone();
                            if let Some(user) = local_users.get(&author_id) {
                                author_name = user.username.clone();
                            } else if author_id != "Unknown" {
                                if let Ok(user_val) = api_client
                                    .get::<serde_json::Value>(crate::api::client::Endpoint::User(author_id.clone()))
                                    .await
                                    && let Some(username) = user_val.get("username").and_then(|v| v.as_str())
                                {
                                    author_name = username.to_string();
                                    let new_user = crate::models::User {
                                        id: author_id.clone(),
                                        username: username.to_string(),
                                    };
                                    local_users.insert(author_id.clone(), new_user.clone());
                                    new_users_fetched.push(new_user);
                                }
                            }

                            let content = if let Some(content_val) =
                                msg.get("content").and_then(|v| v.as_str())
                            {
                                content_val.to_string()
                            } else if let Some(sys) = msg.get("system") {
                                format!(
                                    "[System message: {}]",
                                    sys.get("type")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("unknown")
                                )
                            } else {
                                "[Unsupported message]".to_string()
                            };

                            parsed_messages.push(crate::models::Message {
                                id,
                                author_id,
                                author_name,
                                content,
                            });
                        }

                        app_tx
                            .send(AppEvent::DmMessagesLoaded(parsed_messages, new_users_fetched))
                            .await
                            .ok();
                    }
                    Err(e) => {
                        error!("Error fetching messages: {e}");
                        app_tx
                            .send(AppEvent::DmMessagesLoaded(Vec::new(), Vec::new()))
                            .await
                            .ok();
                    }
                }
            });
        }
        _ => {}
    }
}
