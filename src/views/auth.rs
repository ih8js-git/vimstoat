use ratatui::{
    Frame,
    crossterm::event::{KeyCode, KeyEvent},
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};

use crate::app::{App, AppState};

// --- Render: Input Token ---

pub fn render(f: &mut Frame, app: &App) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(area);

    let explanation = Paragraph::new(
        "We couldn't find your user_token. Please paste it below and press Enter to save it securely:",
    )
    .style(Style::default().fg(Color::Yellow));
    f.render_widget(explanation, chunks[0]);
    let masked_token = "•".repeat(app.input_text.chars().count());
    let input_block = Paragraph::new(masked_token)
        .block(Block::default().title(" User Token ").borders(Borders::ALL));
    f.render_widget(input_block, chunks[1]);

    // Clamp cursor X to the input box interior
    let cursor_x = (chunks[1].x + 1 + app.input_cursor as u16)
        .min(chunks[1].x + chunks[1].width.saturating_sub(2));
    f.set_cursor_position(ratatui::layout::Position::new(cursor_x, chunks[1].y + 1));
}

// --- Render: Validating Token ---

pub fn render_validating(f: &mut Frame) {
    let msg = Paragraph::new("Validating token with Stoat API...")
        .style(Style::default().fg(Color::Cyan))
        .block(
            Block::default()
                .title(" Authenticating ")
                .borders(Borders::ALL),
        );
    f.render_widget(msg, f.area());
}

// --- Handle: Input Token ---

pub async fn handle(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => {
            if !app.input_text.is_empty() {
                app.state = AppState::ValidationToken;
                match app
                    .auth
                    .validate_token(&app.input_text, Some(app.api_base_url.clone()))
                    .await
                {
                    Ok(client) => match app.auth.store_token(&app.input_text).await {
                        Ok(_) => {
                            app.api_client = client;
                            if let Err(e) = app.setup_session().await {
                                app.state = AppState::Error(e);
                                return;
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
