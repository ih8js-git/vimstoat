mod action;
mod api;
mod app;
mod cache;
mod command;
mod error;
mod handlers;
mod input;
mod models;
mod notification;
mod ui;

use std::{fs, path::PathBuf};

use crate::{api::client::Endpoint, cache::Id};
use app::App;
use log::debug;
use ratatui::crossterm::event::{self, Event};

pub const LOG_FILE: &str = "logs";

pub type Result<T> = anyhow::Result<T>;

fn create_log_file() -> Result<fs::File> {
    let mut path = if let Some(mut p) = dirs::cache_dir() {
        p.push(env!("CARGO_PKG_NAME"));
        p
    } else {
        PathBuf::new()
    };

    fs::create_dir_all(&path)?;

    path.push(LOG_FILE);

    Ok(fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .expect("Failed to open log file"))
}

#[tokio::main]
async fn main() -> anyhow::Result<(), Box<dyn std::error::Error>> {
    let log_file = create_log_file()?;
    env_logger::builder()
        .target(env_logger::Target::Pipe(Box::new(log_file)))
        .filter_level(log::LevelFilter::Debug)
        .init();

    log::info!("Starting vimstoat.");

    let mut terminal = ratatui::init();

    ratatui::crossterm::execute!(
        std::io::stdout(),
        ratatui::crossterm::event::PushKeyboardEnhancementFlags(
            ratatui::crossterm::event::KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
        ),
        ratatui::crossterm::cursor::SetCursorStyle::BlinkingBlock
    )
    .ok();

    let api_base_url = std::env::var("API_BASE_URL").ok();
    let ws_base_url = std::env::var("WS_BASE_URL").ok();

    let mut app = App::new(api_base_url.clone(), ws_base_url.clone()).await?;

    app.authenticate_ws(&app.api_client.clone_token()).await?;

    if let Ok(me_val) = app
        .api_client
        .get::<serde_json::Value>(Endpoint::CurrentUser)
        .await
        && let (Some(my_id), Some(my_username)) = (
            me_val.get("_id").and_then(|v| v.as_str()),
            me_val.get("username").and_then(|v| v.as_str()),
        )
        && let Ok(uid) = Id::<crate::models::User>::new(my_id)
    {
        let user = crate::models::User {
            id: my_id.to_string(),
            username: my_username.to_string(),
        };
        let mut cache_locked = app.cache.lock().await;
        cache_locked.set(uid, &user).ok();
        app.store.users.insert(user.id.clone(), user);
    }

    loop {
        terminal.draw(|f| ui::render(f, &app))?;

        // Handle Keyboard Events
        // We limit the poll rate to about 60 frames per second.
        if event::poll(std::time::Duration::from_millis(16))?
            && let Event::Key(key) = event::read()?
        {
            app.handle_key_event(key).await?;

            if app.should_quit {
                break;
            }
        }

        if let Ok(event) = app.app_rx.try_recv() {
            app.handle_app_event(event);
        }

        if let Ok(event) = app.ws_rx.try_recv() {
            debug!("Received WebSocket event: {event:?}");
            api::ws::EventHandler::new(&mut app.store.servers).handle_event(&event);

            if let crate::api::events::ServerEvent::Message(msg_val) = event {
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
                            && let Some(username) =
                                user_val.get("username").and_then(|v| v.as_str())
                        {
                            author_name = username.to_string();
                            let new_user = crate::models::User {
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

                        let message = crate::models::Message {
                            id,
                            author_id,
                            author_name,
                            content,
                        };

                        app_tx
                            .send(app::AppEvent::NewMessage {
                                channel_id,
                                message,
                                new_user: new_user_fetched,
                            })
                            .await
                            .ok();
                    }
                });
            } else if let crate::api::events::ServerEvent::MessageUpdate { id, channel, data } =
                &event
            {
                if let Some(content) = data.get("content").and_then(|v| v.as_str()) {
                    app.app_tx
                        .try_send(app::AppEvent::MessageUpdated {
                            channel_id: channel.clone(),
                            message_id: id.clone(),
                            content: content.to_string(),
                        })
                        .ok();
                }
            } else if let crate::api::events::ServerEvent::MessageDelete { id, channel } = &event {
                app.app_tx
                    .try_send(app::AppEvent::MessageDeleted {
                        channel_id: channel.clone(),
                        message_id: id.clone(),
                    })
                    .ok();
            }
        }
    }

    {
        let mut cache_locked = app.cache.lock().await;
        for user in app.store.users.values() {
            if let Ok(uid) = crate::cache::Id::<crate::models::User>::new(&user.id) {
                let _ = cache_locked.set(uid, user);
            }
        }
        if let Err(e) = cache_locked.dump() {
            log::error!("Failed to dump cache to disk: {}", e);
        }
    }

    ratatui::crossterm::execute!(
        std::io::stdout(),
        ratatui::crossterm::event::PopKeyboardEnhancementFlags,
        ratatui::crossterm::cursor::SetCursorStyle::DefaultUserShape
    )
    .ok();

    ratatui::restore();
    Ok(())
}
