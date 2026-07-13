use crate::app::{App, AppState};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};

pub fn render(f: &mut Frame, app: &App) {
    let area = f.area();

    match &app.state {
        AppState::InputToken => {
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

            let input_block = Paragraph::new(app.input_text.as_str())
                .block(Block::default().title(" User Token ").borders(Borders::ALL));
            f.render_widget(input_block, chunks[1]);
        }
        AppState::ValidatingToken => {
            let msg = Paragraph::new("Validating token with Stoat API...")
                .style(Style::default().fg(Color::Cyan))
                .block(
                    Block::default()
                        .title(" Authenticating ")
                        .borders(Borders::ALL),
                );
            f.render_widget(msg, area);
        }
        AppState::LoggedIn => {
            let success_msg = Paragraph::new(format!(
                "Logged in as: {}\n\nPress 'q' to quit.",
                app.username
            ))
            .block(Block::default().title(" VimStoat ").borders(Borders::ALL));
            f.render_widget(success_msg, area);
        }
        AppState::Error(err_msg) => {
            let error_text = format!(
                "CRITICAL SYSTEM ERROR:\n\n{}\n\nPress any key to return...",
                err_msg
            );
            let error_msg = Paragraph::new(error_text)
                .style(Style::default().fg(Color::Red))
                .block(
                    Block::default()
                        .title(" Keyring Failure ")
                        .borders(Borders::ALL),
                );
            f.render_widget(error_msg, area);
        }
    }
}
