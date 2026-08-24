use ratatui::crossterm::event::KeyEvent;

use crate::{action::Action, app::App, input::InputMode};

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
            if !content.is_empty() {
                if let Some(channel) = app.store.dm_channels.get(app.selected_dm_index) {
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
            }
            app.input_text.clear();
            app.input_cursor = 0;
            app.set_input_mode(InputMode::Normal);
        }
        _ => {}
    }
}
