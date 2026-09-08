use ratatui::{
    Frame,
    crossterm::event::{KeyCode, KeyEvent},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};
use thiserror::Error;

use crate::app::{App, AppState};

#[derive(Error, Debug)]
pub enum CacheError {
    #[error("Database Error: {0:?}")]
    DbError(pickledb::error::Error),

    #[error("Directory not found: {0}")]
    DirNotFound(String),
}

#[derive(Error, Debug)]
pub enum IdError {
    #[error("Invalid size: expected 26, got {0}")]
    InvalidSize(usize),
}

impl From<pickledb::error::Error> for CacheError {
    fn from(value: pickledb::error::Error) -> Self {
        CacheError::DbError(value)
    }
}

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("Keyring Error: {0:?}")]
    KeyringError(keyring::Error),

    #[error("Invalid Token: {0}")]
    InvalidToken(String),

    #[error("Could not connect to the server. Please check your internet connection.")]
    ServerConnectionError,

    #[error("Request Error: {0}")]
    RequestError(String),
}

impl From<keyring::Error> for AuthError {
    fn from(value: keyring::Error) -> Self {
        AuthError::KeyringError(value)
    }
}

pub fn handle(app: &mut App, key: KeyEvent) {
    if matches!(key.code, KeyCode::Char(_) | KeyCode::Esc | KeyCode::Enter) {
        app.state = AppState::InputToken;
    }
}

pub fn render(f: &mut Frame, message: &str) {
    let title = if message.contains("keyring") || message.contains("Keyring") {
        " Keyring Error "
    } else if message.contains("connect") || message.contains("internet") {
        " Connection Error "
    } else if message.to_lowercase().contains("invalid token") {
        " Authentication Error "
    } else {
        " Error "
    };

    let error_text = format!("{message}\n\nPress any key to return...");
    let error_msg = Paragraph::new(error_text)
        .style(Style::default().fg(Color::Red))
        .block(Block::default().title(title).borders(Borders::ALL));
    f.render_widget(error_msg, f.area());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_error_display() {
        let err = AuthError::InvalidToken("bad_token".to_string());
        assert_eq!(err.to_string(), "Invalid Token: bad_token");

        let err = AuthError::ServerConnectionError;
        assert_eq!(
            err.to_string(),
            "Could not connect to the server. Please check your internet connection."
        );
    }

    #[test]
    fn test_id_error_display() {
        let err = IdError::InvalidSize(10);
        assert_eq!(err.to_string(), "Invalid size: expected 26, got 10");
    }

    #[test]
    fn test_cache_error_display() {
        let err = CacheError::DirNotFound("/test".to_string());
        assert_eq!(err.to_string(), "Directory not found: /test");
    }
}
