use crate::app::App;
use ratatui::{
    Frame,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
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

    let mut message_lines = Vec::new();

    // Revolt API returns messages in descending order (newest first).
    // We reverse to render oldest at top and newest at bottom.
    for msg in app.store.current_dm_messages.iter().rev() {
        let mut first = true;
        for line_str in msg.content.split('\n') {
            if first {
                let author_span = Span::styled(
                    format!("{}: ", msg.author_name),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                );
                let content_span = Span::raw(line_str);
                message_lines.push(Line::from(vec![author_span, content_span]));
                first = false;
            } else {
                message_lines.push(Line::from(vec![Span::raw(line_str)]));
            }
        }
    }

    let text_width = area.width.saturating_sub(2).max(1) as usize;
    let mut input_lines = 0;
    let split_lines: Vec<&str> = app.input_text.split('\n').collect();

    for (i, line) in split_lines.iter().enumerate() {
        let chars = line.chars().count();
        if i == split_lines.len() - 1 {
            input_lines += (chars / text_width) + 1;
        } else if chars == 0 {
            input_lines += 1;
        } else {
            input_lines += chars.div_ceil(text_width);
        }
    }

    let input_height = (input_lines as u16) + 2; // +2 for borders

    let chunks = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Min(0),
            ratatui::layout::Constraint::Length(input_height),
        ])
        .split(area);

    let messages_area = chunks[0];
    let input_area = chunks[1];

    let msg_text_width = messages_area.width.saturating_sub(2).max(1) as usize;
    let mut total_msg_lines = 0;

    for line in &message_lines {
        let chars = line
            .spans
            .iter()
            .map(|s| s.content.chars().count())
            .sum::<usize>();
        if chars == 0 {
            total_msg_lines += 1;
        } else {
            total_msg_lines += chars.div_ceil(msg_text_width);
        }
    }

    let scroll =
        total_msg_lines.saturating_sub(messages_area.height.saturating_sub(2) as usize) as u16;

    let msg_paragraph = Paragraph::new(message_lines)
        .block(block)
        .wrap(ratatui::widgets::Wrap { trim: false })
        .scroll((scroll, 0));

    f.render_widget(msg_paragraph, messages_area);

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

    let input_paragraph = Paragraph::new(app.input_text.as_str())
        .block(input_block)
        .wrap(ratatui::widgets::Wrap { trim: false });

    f.render_widget(input_paragraph, input_area);

    if matches!(
        app.input_state.input_mode,
        crate::input::InputMode::Insert | crate::input::InputMode::UI
    ) {
        let current_line = split_lines.last().unwrap_or(&"");
        let current_line_chars = current_line.chars().count();

        let cursor_x = input_area.x + 1 + (current_line_chars % text_width) as u16;
        let cursor_y = input_area.y + 1 + (input_lines as u16).saturating_sub(1);

        let clamped_x = cursor_x.min(input_area.x + input_area.width.saturating_sub(2));
        let clamped_y = cursor_y.min(input_area.y + input_area.height.saturating_sub(2));

        f.set_cursor_position(ratatui::layout::Position::new(clamped_x, clamped_y));
    }
}
