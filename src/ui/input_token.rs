use crate::app::App;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};

pub fn render(f: &mut Frame, app: &App) {
    let area = f.area();
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

    // Clamp cursor X to the input box interior
    let cursor_x = (chunks[1].x + 1 + app.input_cursor as u16)
        .min(chunks[1].x + chunks[1].width.saturating_sub(2));
    f.set_cursor_position(ratatui::layout::Position::new(cursor_x, chunks[1].y + 1));
}
