use crate::app::App;
use ratatui::{
    Frame,
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph},
};

pub fn render(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let border_color = if matches!(app.input_state.input_mode, crate::input::InputMode::Command) {
        Color::Green
    } else {
        Color::Reset
    };

    let title = if let Some(channel) = app.dm_channels.get(app.selected_dm_index) {
        format!(" Direct Message: {} ", channel.name)
    } else {
        " Direct Message ".to_string()
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let content = vec![
        Line::from("").style(Style::default()),
        Line::from("  No messages yet... (Press Esc to return to list)")
            .style(Style::default().fg(Color::DarkGray)),
    ];

    let paragraph = Paragraph::new(content).block(block);

    f.render_widget(paragraph, area);
}
