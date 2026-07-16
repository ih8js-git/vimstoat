use ratatui::{
    Frame,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};

pub fn render(f: &mut Frame) {
    let msg = Paragraph::new("Validating token with Stoat API...")
        .style(Style::default().fg(Color::Cyan))
        .block(
            Block::default()
                .title(" Authenticating ")
                .borders(Borders::ALL),
        );
    f.render_widget(msg, f.area());
}
