use log::error;
use ratatui::{
    Frame,
    crossterm::event::KeyEvent,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

use crate::{
    action::Action,
    app::{App, AppEvent, AppState},
    input::InputMode,
};

// --- Render ---

pub fn render(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let total_items = app.store.dm_channels.len();

    let border_color = app.input_state.input_mode.color();

    if app.is_loading_dms && total_items == 0 {
        let msg = Paragraph::new("Loading Direct Messages...")
            .style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .block(
                Block::default()
                    .title(" Direct Messages ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color)),
            );
        f.render_widget(msg, area);
        return;
    }

    if total_items == 0 {
        let msg = Paragraph::new("No Direct Messages found. (Type :q to return)")
            .style(Style::default().fg(Color::DarkGray))
            .block(
                Block::default()
                    .title(" Direct Messages ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color)),
            );
        f.render_widget(msg, area);
        return;
    }

    let selected_index = app.selected_dm_index.min(total_items.saturating_sub(1));
    let num_digits = total_items.to_string().len();

    let mut items: Vec<ListItem> = Vec::new();

    for (i, channel) in app.store.dm_channels.iter().enumerate() {
        let is_selected = i == selected_index;
        let rel_num = (i as isize - selected_index as isize).unsigned_abs();
        let width = num_digits.max(2);

        let line_num_str = if is_selected {
            format!("{i:<width$} ")
        } else {
            format!("{rel_num:>width$} ")
        };

        let num_style = if is_selected {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let text_style = if is_selected {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Cyan)
        };

        let mut spans = vec![Span::styled(line_num_str, num_style)];
        if channel.has_unread {
            spans.push(Span::styled(
                "[*] ",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ));
        }

        spans.push(Span::styled(channel.name.as_str(), text_style));

        if let Some(user) = channel
            .recipient_id
            .as_ref()
            .and_then(|uid| app.store.users.get(uid))
        {
            let (sym, color) = user.status.bubble();
            spans.push(Span::styled(sym, Style::default().fg(color)));

            if user.status != crate::models::UserStatus::Offline
                && let Some(text) = &user.status_text
                && !text.trim().is_empty()
            {
                spans.push(Span::styled(
                    format!(" - {text}"),
                    Style::default().fg(Color::DarkGray),
                ));
            }
        }

        items.push(ListItem::new(Line::from(spans)));
    }

    let mut state = ListState::default();
    state.select(Some(selected_index));

    let list = List::new(items)
        .block(
            Block::default()
                .title(" Direct Messages ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color)),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    f.render_stateful_widget(list, area, &mut state);
}

// --- Handle ---

pub fn handle(app: &mut App, key: KeyEvent) {
    let action = app.input_state.process_key_event(key);
    match action {
        Some(Action::Quit) => app.should_quit = true,
        Some(Action::EnterCommandMode) => {
            app.command_text.clear();
            app.set_input_mode(InputMode::Command);
        }
        Some(Action::CursorUp) => {
            if app.selected_dm_index > 0 {
                app.selected_dm_index -= 1;
            }
        }
        Some(Action::CursorDown) => {
            let total_items = app.store.dm_channels.len();
            if total_items > 0 && app.selected_dm_index + 1 < total_items {
                app.selected_dm_index += 1;
            }
        }
        Some(Action::GoToTopUI) => {
            app.selected_dm_index = 0;
        }
        Some(Action::Enter) if !app.store.dm_channels.is_empty() => {
            let channel_id = app.store.dm_channels[app.selected_dm_index].id.clone();
            app.store.dm_channels[app.selected_dm_index].has_unread = false;
            app.state = AppState::Dm;
            app.set_input_mode(InputMode::Normal);
            app.is_loading_messages = true;
            app.store.current_dm_messages.clear();
            app.input_text.clear();

            let api_client = app.api_client.clone();
            let app_tx = app.app_tx.clone();
            let users = app.store.users.clone();

            tokio::spawn(async move {
                let query = crate::api::channel::MessageHistoryQuery {
                    limit: Some(50),
                    before: None,
                    after: None,
                    sort: None,
                    nearby: None,
                };
                match crate::api::channel::fetch_message_history(
                    &api_client,
                    &channel_id,
                    Some(&query),
                )
                .await
                {
                    Ok(messages_json) => {
                        let mut parsed_messages = Vec::with_capacity(messages_json.len());
                        let mut new_users_fetched = Vec::new();
                        let mut local_users = users.clone();

                        for msg in messages_json {
                            let id = msg
                                .get("_id")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown")
                                .to_string();

                            let author_id = msg
                                .get("author")
                                .and_then(|v| v.as_str())
                                .unwrap_or("Unknown")
                                .to_string();

                            let mut author_name = author_id.clone();
                            if let Some(user) = local_users.get(&author_id) {
                                author_name = user.username.clone();
                            } else if author_id != "Unknown"
                                && let Ok(user) =
                                    crate::api::user::fetch_user(&api_client, &author_id).await
                            {
                                author_name = user.username.clone();
                                local_users.insert(author_id.clone(), user.clone());
                                new_users_fetched.push(user);
                            }

                            let content = if let Some(content_val) =
                                msg.get("content").and_then(|v| v.as_str())
                            {
                                content_val.to_string()
                            } else if let Some(sys) = msg.get("system") {
                                format!(
                                    "[System message: {}]",
                                    sys.get("type")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("unknown")
                                )
                            } else {
                                "[Unsupported message]".to_string()
                            };

                            parsed_messages.push(crate::models::Message {
                                id,
                                author_id,
                                author_name,
                                content,
                            });
                        }

                        app_tx
                            .send(AppEvent::DmMessagesLoaded(
                                parsed_messages,
                                new_users_fetched,
                            ))
                            .await
                            .ok();
                    }
                    Err(e) => {
                        error!("Error fetching messages: {e}");
                        app_tx
                            .send(AppEvent::DmMessagesLoaded(Vec::new(), Vec::new()))
                            .await
                            .ok();
                    }
                }
            });
        }
        _ => {}
    }
}
