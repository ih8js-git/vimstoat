use ratatui::{
    Frame,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};

pub fn render(f: &mut Frame, message: &str) {
    let error_text = format!("CRITICAL SYSTEM ERROR:\n\n{message}\n\nPress any key to return...");
    let error_msg = Paragraph::new(error_text)
        .style(Style::default().fg(Color::Red))
        .block(
            Block::default()
                .title(" Keyring Failure ")
                .borders(Borders::ALL),
        );
    f.render_widget(error_msg, f.area());
}
