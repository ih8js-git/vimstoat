use ratatui::crossterm::event::{KeyCode, KeyEvent};

use crate::app::{App, AppState};

pub fn handle(app: &mut App, key: KeyEvent) {
    if matches!(key.code, KeyCode::Char(_) | KeyCode::Esc | KeyCode::Enter) {
        app.state = AppState::InputToken;
    }
}
