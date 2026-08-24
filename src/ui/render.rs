use crate::{
    app::{App, AppState},
    input::InputMode,
};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph},
};

use super::{dm, dm_list, error, input_token, server_list, validating_token};

pub fn render(f: &mut Frame, app: &App) {
    let is_command_mode = matches!(app.input_state.input_mode, InputMode::Command);

    let (main_area, command_area) = if is_command_mode {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(3)])
            .split(f.area());
        (chunks[0], Some(chunks[1]))
    } else {
        (f.area(), None)
    };

    match &app.state {
        AppState::InputToken => input_token::render(f, app),
        AppState::ValidatingToken => validating_token::render(f),
        AppState::LoggedIn => server_list::render(f, app, main_area),
        AppState::DmList => dm_list::render(f, app, main_area),
        AppState::Dm => dm::render(f, app, main_area),
        AppState::Error(message) => error::render(f, &message.to_string()),
    }

    if let Some(cmd_area) = command_area {
        let cmd_widget = Paragraph::new(format!(":{}", app.command_text))
            .style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .block(
                Block::default()
                    .title(" Command ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(app.input_state.input_mode.color())),
            );
        f.render_widget(cmd_widget, cmd_area);

        // Ensure cursor is placed within the command input box
        f.set_cursor_position(ratatui::layout::Position::new(
            cmd_area.x + 2 + app.command_text.chars().count() as u16,
            cmd_area.y + 1,
        ));
    }
}
