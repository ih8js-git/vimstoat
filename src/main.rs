mod app;
mod state;
mod ui;

use std::time::Duration;

use app::App;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let servers = &*state::MOCK_SERVERS;
    ratatui::run(|terminal| {
        loop {
            terminal.draw(|f| ui::render_state(f, servers)).unwrap();
            if event::poll(Duration::from_millis(16)).unwrap()
                && let Event::Key(k) = event::read().unwrap()
                && k.code == KeyCode::Char('q')
            {
                break;
            }
        }
    });

    // let mut terminal = ratatui::init();
    // let mut app = App::new().await?;

    // loop {
    //     terminal.draw(|f| ui::render(f, &app))?;

    //     // Handle Keyboard Events
    //     if event::poll(std::time::Duration::from_millis(16))?
    //         && let Event::Key(key) = event::read()?
    //     {
    //         app.handle_key_event(key).await?;

    //         if app.should_quit {
    //             break;
    //         }
    //     }
    // }

    // ratatui::restore();
    Ok(())
}
