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
