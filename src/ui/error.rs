use ratatui::{
    Frame,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};

pub fn render(f: &mut Frame, message: &str) {
    let title = if message.contains("keyring") || message.contains("Keyring") {
        " Keyring Error "
    } else if message.contains("connect") || message.contains("internet") {
        " Connection Error "
    } else if message.contains("Invalid token") {
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
