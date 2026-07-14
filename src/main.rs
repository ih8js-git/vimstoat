mod app;
mod cache;
mod error;
mod ui;

use app::App;
use ratatui::crossterm::event::{self, Event};

pub type Result<T> = anyhow::Result<T>;

#[tokio::main]
async fn main() -> anyhow::Result<(), Box<dyn std::error::Error>> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .init();

    log::info!("Starting vimstoat.");

    let mut terminal = ratatui::init();

    let mut app = App::new().await?;

    loop {
        terminal.draw(|f| ui::render(f, &app))?;

        // Handle Keyboard Events
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
