use crate::app::{App, AppState};
use ratatui::Frame;

use super::{error, input_token, server_list, validating_token};

pub fn render(f: &mut Frame, app: &App) {
    match &app.state {
        AppState::InputToken => input_token::render(f, app),
        AppState::ValidatingToken => validating_token::render(f),
        AppState::LoggedIn => server_list::render(f, app),
        AppState::Error(message) => error::render(f, &message.to_string()),
    }
}
