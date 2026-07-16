use crate::app::App;
use ratatui::{
    Frame,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};

pub fn render(f: &mut Frame, _app: &App) {
    let msg = Paragraph::new("Server list is not yet implemented.")
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().title(" Servers ").borders(Borders::ALL));
    f.render_widget(msg, f.area());
}
