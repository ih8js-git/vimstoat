use ratatui::crossterm::event::{KeyCode, KeyEvent};

use crate::app::{App, AppState};

pub async fn handle(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => {
            if !app.input_text.is_empty() {
                app.state = AppState::ValidatingToken;
                match app
                    .auth
                    .validate_token(&app.input_text, Some(app.api_base_url.clone()))
                    .await
                {
                    Ok(client) => match app.auth.store_token(&app.input_text).await {
                        Ok(_) => {
                            app.api_client = client;
                            if let Err(e) = app.authenticate_ws(&app.api_client.clone_token()).await
                            {
                                app.state = AppState::Error(e);
                                return;
                            }
                            if let Ok(me_val) = app
                                .api_client
                                .get::<serde_json::Value>(crate::api::client::Endpoint::CurrentUser)
                                .await
                                && let (Some(my_id), Some(my_username)) = (
                                    me_val.get("_id").and_then(|v| v.as_str()),
                                    me_val.get("username").and_then(|v| v.as_str()),
                                )
                                && let Ok(uid) = crate::cache::Id::<crate::models::User>::new(my_id)
                            {
                                let user = crate::models::User {
                                    id: my_id.to_string(),
                                    username: my_username.to_string(),
                                };
                                let mut cache_locked = app.cache.lock().await;
                                cache_locked.set(uid, &user).ok();
                                app.store.users.insert(user.id.clone(), user);
                            }
                            app.state = AppState::LoggedIn;
                        }
                        Err(detailed_err) => {
                            app.state = AppState::Error(detailed_err);
                        }
                    },
                    Err(e) => {
                        app.state = AppState::Error(e);
                    }
                }
            }
        }
        KeyCode::Char(c) => {
            app.input_text.push(c);
            app.input_cursor += 1;
        }
        KeyCode::Backspace => {
            app.input_text.pop();
            app.input_cursor = app.input_cursor.saturating_sub(1);
        }
        KeyCode::Esc => {
            app.should_quit = true;
        }
        _ => {}
    }
}
