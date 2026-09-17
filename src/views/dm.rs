use crate::app::App;
use ratatui::{
    Frame,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

// --- Render ---

pub fn render(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let border_color = app.input_state.input_mode.color();

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

    let input_border_color = app.input_state.input_mode.color();

    let input_title = if let Some(channel) = app.store.dm_channels.get(app.selected_dm_index)
        && !channel.typing_users.is_empty()
    {
        let mut typing_names = Vec::new();
        for uid in &channel.typing_users {
            let name = app
                .store
                .users
                .get(uid)
                .map(|u| u.username.as_str())
                .unwrap_or("Someone");
            typing_names.push(name);
        }
        let typing_str = typing_names.join(", ");
        format!(
            " Message (Type 'i' to insert, ESC for normal) - {} is typing... ",
            typing_str
        )
    } else {
        " Message (Type 'i' to insert, ESC for normal) ".to_string()
    };

    let input_block = Block::default()
        .title(input_title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(input_border_color));

    let input_paragraph = Paragraph::new(app.input_text.as_str())
        .block(input_block)
        .wrap(ratatui::widgets::Wrap { trim: false });

    f.render_widget(input_paragraph, input_area);

    if matches!(
        app.input_state.input_mode,
        crate::input::InputMode::Insert | crate::input::InputMode::Normal
    ) {
        let mut cursor_line_idx = 0;
        let mut cursor_char_idx = 0;
        let mut chars_counted = 0;

        for (i, line) in split_lines.iter().enumerate() {
            let line_len = line.chars().count();
            let len_with_nl = if i == split_lines.len() - 1 {
                line_len
            } else {
                line_len + 1
            };

            if app.input_cursor >= chars_counted && app.input_cursor < chars_counted + len_with_nl {
                cursor_line_idx = i;
                cursor_char_idx = app.input_cursor - chars_counted;
                break;
            } else if i == split_lines.len() - 1 && app.input_cursor >= chars_counted + len_with_nl
            {
                cursor_line_idx = i;
                cursor_char_idx = line_len;
            }
            chars_counted += len_with_nl;
        }

        let mut base_y_offset = 0;
        for line in split_lines.iter().take(cursor_line_idx) {
            let chars = line.chars().count();
            if chars == 0 {
                base_y_offset += 1;
            } else {
                base_y_offset += chars.div_ceil(text_width);
            }
        }

        let cursor_x = input_area.x + 1 + (cursor_char_idx % text_width) as u16;
        let cursor_y =
            input_area.y + 1 + base_y_offset as u16 + (cursor_char_idx / text_width) as u16;

        let clamped_x = cursor_x.min(input_area.x + input_area.width.saturating_sub(2));
        let clamped_y = cursor_y.min(input_area.y + input_area.height.saturating_sub(2));

        f.set_cursor_position(ratatui::layout::Position::new(clamped_x, clamped_y));
    }
}

// --- Handle ---

use ratatui::crossterm::event::KeyEvent;

use crate::{action::Action, input::InputMode};

pub fn handle(app: &mut App, key: KeyEvent) {
    let action = app.input_state.process_key_event(key);
    match action {
        Some(Action::CursorLeft) => {
            let chars: Vec<char> = app.input_text.chars().collect();
            if app.input_cursor > 0 && chars.get(app.input_cursor - 1) != Some(&'\n') {
                app.input_cursor -= 1;
            }
        }
        Some(Action::CursorRight) => {
            let chars: Vec<char> = app.input_text.chars().collect();
            let max_for_line = {
                let mut end = app.input_cursor;
                while end < chars.len() && chars[end] != '\n' {
                    end += 1;
                }
                if matches!(app.input_state.input_mode, InputMode::Insert) {
                    end
                } else if end > 0 && chars.get(end - 1) != Some(&'\n') {
                    end - 1
                } else {
                    end
                }
            };
            if app.input_cursor < max_for_line {
                app.input_cursor += 1;
            }
        }
        Some(Action::CursorUp) => {
            let chars: Vec<char> = app.input_text.chars().collect();
            let mut line_start = 0;
            for i in (0..app.input_cursor).rev() {
                if chars.get(i) == Some(&'\n') {
                    line_start = i + 1;
                    break;
                }
            }
            if line_start > 0 {
                let col = app.input_cursor - line_start;
                let mut prev_line_start = 0;
                for i in (0..line_start - 1).rev() {
                    if chars.get(i) == Some(&'\n') {
                        prev_line_start = i + 1;
                        break;
                    }
                }
                let prev_line_len = (line_start - 1) - prev_line_start;

                let is_normal = matches!(app.input_state.input_mode, InputMode::Normal);
                let max_col = if is_normal && prev_line_len > 0 {
                    prev_line_len - 1
                } else {
                    prev_line_len
                };

                app.input_cursor = prev_line_start + col.min(max_col);
            }
        }
        Some(Action::CursorDown) => {
            let chars: Vec<char> = app.input_text.chars().collect();
            let mut line_start = 0;
            for i in (0..app.input_cursor).rev() {
                if chars.get(i) == Some(&'\n') {
                    line_start = i + 1;
                    break;
                }
            }
            let col = app.input_cursor - line_start;

            let mut next_line_start = None;
            for (i, c) in chars.iter().enumerate().skip(app.input_cursor) {
                if *c == '\n' {
                    next_line_start = Some(i + 1);
                    break;
                }
            }
            if let Some(start) = next_line_start {
                let mut next_line_len = 0;
                for c in chars.iter().skip(start) {
                    if *c == '\n' {
                        break;
                    }
                    next_line_len += 1;
                }
                let is_normal = matches!(app.input_state.input_mode, InputMode::Normal);
                let max_col = if is_normal && next_line_len > 0 {
                    next_line_len - 1
                } else {
                    next_line_len
                };
                app.input_cursor = start + col.min(max_col);
            }
        }
        Some(Action::Quit) => app.go_back_or_quit(),
        Some(Action::EnterCommandMode) => {
            app.command_text.clear();
            app.set_input_mode(InputMode::Command);
        }
        Some(Action::EnterInsertMode) => {
            app.set_input_mode(InputMode::Insert);
        }
        Some(Action::EnterInsertModeAfter) => {
            if app.input_cursor < app.input_text.chars().count() {
                app.input_cursor += 1;
            }
            app.set_input_mode(InputMode::Insert);
        }
        Some(Action::EnterInsertModeLineStart) => {
            let chars: Vec<char> = app.input_text.chars().collect();
            let mut line_start = 0;
            for i in (0..app.input_cursor).rev() {
                if chars.get(i) == Some(&'\n') {
                    line_start = i + 1;
                    break;
                }
            }
            app.input_cursor = line_start;
            app.set_input_mode(InputMode::Insert);
        }
        Some(Action::EnterInsertModeLineEnd) => {
            let chars: Vec<char> = app.input_text.chars().collect();
            let mut line_end = chars.len();
            for (i, c) in chars.iter().enumerate().skip(app.input_cursor) {
                if *c == '\n' {
                    line_end = i;
                    break;
                }
            }
            app.input_cursor = line_end;
            app.set_input_mode(InputMode::Insert);
        }
        Some(Action::OpenNewLineBelow) => {
            let mut chars: Vec<char> = app.input_text.chars().collect();
            let mut insert_idx = chars.len();
            for (i, c) in chars.iter().enumerate().skip(app.input_cursor) {
                if *c == '\n' {
                    insert_idx = i;
                    break;
                }
            }
            chars.insert(insert_idx, '\n');
            app.input_text = chars.into_iter().collect();
            app.input_cursor = insert_idx + 1;
            app.set_input_mode(InputMode::Insert);
        }
        Some(Action::OpenNewLineAbove) => {
            let mut chars: Vec<char> = app.input_text.chars().collect();
            let mut insert_idx = 0;
            for i in (0..app.input_cursor).rev() {
                if chars.get(i) == Some(&'\n') {
                    insert_idx = i + 1;
                    break;
                }
            }
            chars.insert(insert_idx, '\n');
            app.input_text = chars.into_iter().collect();
            app.input_cursor = insert_idx;
            app.set_input_mode(InputMode::Insert);
        }
        Some(Action::DeleteLine) => {
            let chars: Vec<char> = app.input_text.chars().collect();
            if !chars.is_empty() {
                let mut line_start = 0;
                for i in (0..app.input_cursor).rev() {
                    if chars.get(i) == Some(&'\n') {
                        line_start = i + 1;
                        break;
                    }
                }

                let mut line_end = chars.len();
                for (i, c) in chars.iter().enumerate().skip(app.input_cursor) {
                    if *c == '\n' {
                        line_end = i;
                        break;
                    }
                }

                let mut delete_start = line_start;
                let mut delete_end = line_end;

                if line_end < chars.len() && chars[line_end] == '\n' {
                    delete_end += 1;
                } else if line_start > 0 && chars[line_start - 1] == '\n' {
                    delete_start -= 1;
                }

                let yank_content: String = chars[line_start..line_end].iter().collect();
                app.yank_buffer = Some(format!("{}\n", yank_content));

                let mut new_chars = Vec::new();
                new_chars.extend_from_slice(&chars[0..delete_start]);
                new_chars.extend_from_slice(&chars[delete_end..chars.len()]);

                app.input_text = new_chars.into_iter().collect();

                let chars_after: Vec<char> = app.input_text.chars().collect();
                if chars_after.is_empty() {
                    app.input_cursor = 0;
                } else if delete_start < chars_after.len() {
                    app.input_cursor = delete_start;
                } else {
                    let mut new_start = 0;
                    for i in (0..chars_after.len()).rev() {
                        if chars_after[i] == '\n' {
                            new_start = i + 1;
                            break;
                        }
                    }
                    app.input_cursor = new_start;
                }
            }
        }
        Some(Action::AppendCharacter(c)) => {
            let mut chars: Vec<char> = app.input_text.chars().collect();
            if app.input_cursor <= chars.len() {
                chars.insert(app.input_cursor, c);
                app.input_text = chars.into_iter().collect();
                app.input_cursor += 1;
            }
        }
        Some(Action::RemoveCharacter) => {
            if app.input_cursor > 0 {
                let mut chars: Vec<char> = app.input_text.chars().collect();
                chars.remove(app.input_cursor - 1);
                app.input_text = chars.into_iter().collect();
                app.input_cursor -= 1;
            }
        }
        Some(Action::Escape) => {
            if matches!(app.input_state.input_mode, InputMode::Insert) && app.input_cursor > 0 {
                let chars: Vec<char> = app.input_text.chars().collect();
                if chars.get(app.input_cursor - 1) != Some(&'\n') {
                    app.input_cursor -= 1;
                }
            }
            app.set_input_mode(InputMode::Normal);
        }
        Some(Action::Enter)
            if matches!(
                app.input_state.input_mode,
                InputMode::Insert | InputMode::Normal
            ) =>
        {
            let content = app.input_text.trim().to_string();
            if !content.is_empty()
                && let Some(channel) = app.store.dm_channels.get(app.selected_dm_index)
            {
                let channel_id = channel.id.clone();
                let api_client = app.api_client.clone();

                tokio::spawn(async move {
                    #[derive(serde::Serialize)]
                    struct SendMessagePayload {
                        content: String,
                        nonce: String,
                    }

                    let payload = SendMessagePayload {
                        content,
                        nonce: ulid::Ulid::generate().to_string(),
                    };

                    if let Err(e) = api_client
                        .post::<serde_json::Value, _>(
                            crate::api::client::Endpoint::SendMessage(channel_id),
                            &payload,
                        )
                        .await
                    {
                        log::error!("Failed to send message: {}", e);
                    }
                });
            }
            app.input_text.clear();
            app.input_cursor = 0;
            app.set_input_mode(InputMode::Normal);
        }
        _ => {}
    }
}
