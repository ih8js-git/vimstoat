mod action;
mod api;
mod app;
mod cache;
mod error;
mod input;
mod notification;
mod ui;

use std::{fs, path::PathBuf};

use app::App;
use log::debug;
use notify_rust::{CloseReason, NotificationResponse};
use ratatui::crossterm::event::{self, Event};

use crate::notification::NotifyHandler;

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

    let api_base_url = std::env::var("API_BASE_URL").ok();
    let ws_base_url = std::env::var("WS_BASE_URL").ok();

    let mut app = App::new(api_base_url.clone(), ws_base_url.clone()).await?;

    app.authenticate_ws(&app.api_client.clone_token()).await?;

    /* This is an example, for now we have no use for notifications */
    {
        let n = NotifyHandler::new().await?;
        let icon = n.clone_icon_path();

        n.send_notification(
            "This is a title".to_string(),
            "This is a body".to_string(),
            icon,
            vec![
                ("reply".to_string(), "Reply".to_string()),
                ("read".to_string(), "Mark as read".to_string()),
            ],
            |response: &NotificationResponse| match response {
                NotificationResponse::Default => log::info!("body clicked"),
                NotificationResponse::Action(key) => log::info!("button {key:?} clicked"),
                NotificationResponse::Reply(text) => log::info!("user replied: {text}"),
                NotificationResponse::Closed(CloseReason::Dismissed) => {
                    log::info!("dismissed by the user")
                }
                NotificationResponse::Closed(reason) => log::info!("closed: {reason:?}"),
            },
        )
        .await;
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

        if let Ok(event) = app.ws_rx.try_recv() {
            debug!("Received WebSocket event: {event:?}");
        }
    }

    ratatui::restore();
    Ok(())
}
