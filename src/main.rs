mod api;
mod app;
mod cache;
mod error;
mod state;

use app::App;
use ratatui::crossterm::event::{self, Event};
use state::ui;

pub type Result<T> = anyhow::Result<T>;

#[tokio::main]
async fn main() -> anyhow::Result<(), Box<dyn std::error::Error>> {
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("vimstoat.log")
        .expect("Failed to open log file");

    env_logger::builder()
        .target(env_logger::Target::Pipe(Box::new(log_file)))
        .filter_level(log::LevelFilter::Debug)
        .init();

    log::info!("Starting vimstoat.");

    let mut terminal = ratatui::init();

    let mut app = App::new().await?;

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
    }

    ratatui::restore();
    Ok(())
}
