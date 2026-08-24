use crate::app::App;
use ratatui::{
    Frame,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

pub fn render(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let border_color = if matches!(app.input_state.input_mode, crate::input::InputMode::Command) {
        Color::Green
    } else {
        Color::Reset
    };

    let title = if let Some(channel) = app.store.dm_channels.get(app.selected_dm_index) {
        format!(" Direct Message: {} ", channel.name)
    } else {
        " Direct Message ".to_string()
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    if app.is_loading_messages {
        let msg = Paragraph::new("Loading messages...")
            .style(Style::default().fg(Color::Yellow))
            .block(block);
        f.render_widget(msg, area);
        return;
    }

    if app.store.current_dm_messages.is_empty() {
        let msg = Paragraph::new("No messages found. (Type :q to return)")
            .style(Style::default().fg(Color::DarkGray))
            .block(block);
        f.render_widget(msg, area);
        return;
    }

    let mut items = Vec::new();

    // Revolt API returns messages in descending order (newest first).
    // We reverse to render oldest at top and newest at bottom.
    for msg in app.store.current_dm_messages.iter().rev() {
        let author_span = Span::styled(
            format!("{}: ", msg.author_name),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
        let content_span = Span::raw(&msg.content);

        items.push(ListItem::new(Line::from(vec![author_span, content_span])));
    }

    // Calculate the height of the message input box based on lines of text
    let input_lines = (app.input_text.matches('\n').count() as u16) + 1;
    let input_height = input_lines + 2; // +2 for borders

    let chunks = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Min(0),
            ratatui::layout::Constraint::Length(input_height),
        ])
        .split(area);

    let messages_area = chunks[0];
    let input_area = chunks[1];

    let list = List::new(items).block(block);
    f.render_widget(list, messages_area);

    let input_border_color =
        if matches!(app.input_state.input_mode, crate::input::InputMode::Insert) {
            Color::Cyan
        } else {
            Color::Reset
        };

    let input_block = Block::default()
        .title(" Message [Visual Only - Not hooked to API] (Type 'i' to insert, ESC for normal) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(input_border_color));

    let input_paragraph = Paragraph::new(app.input_text.as_str()).block(input_block);

    f.render_widget(input_paragraph, input_area);

    if matches!(app.input_state.input_mode, crate::input::InputMode::Insert) {
        // Find the X and Y offsets for the cursor
        let lines: Vec<&str> = app.input_text.split('\n').collect();
        let current_line = lines.last().unwrap_or(&"");

        let cursor_x = input_area.x + 1 + current_line.chars().count() as u16;
        let cursor_y = input_area.y + 1 + (lines.len() as u16).saturating_sub(1);

        // Clamp inside the input_area (avoid panics if text overflows horizontally)
        let clamped_x = cursor_x.min(input_area.x + input_area.width.saturating_sub(2));
        let clamped_y = cursor_y.min(input_area.y + input_area.height.saturating_sub(2));

        f.set_cursor_position(ratatui::layout::Position::new(clamped_x, clamped_y));
    }
}
