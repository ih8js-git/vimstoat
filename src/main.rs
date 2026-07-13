mod api;
mod app;
mod state;

use app::App;
use ratatui::crossterm::event::{self, Event};
use state::ui;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
