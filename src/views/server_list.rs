use log::error;
use ratatui::{
    Frame,
    crossterm::event::KeyEvent,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
};

use crate::{
    action::Action,
    app::{App, AppEvent, AppState},
    input::InputMode,
};

// --- Render ---

pub fn render(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let total_items = 1 + app.store.servers.len();
    let selected_index = app.selected_index.min(total_items.saturating_sub(1));

    let num_digits = if total_items > 0 {
        total_items.to_string().len()
    } else {
        1
    };

    let mut items: Vec<ListItem> = Vec::new();

    for i in 0..total_items {
        let is_selected = i == selected_index;
        let rel_num = (i as isize - selected_index as isize).unsigned_abs();
        let width = num_digits.max(2);

        let line_num_str = if is_selected {
            format!("{:<width$} ", i, width = width)
        } else {
            format!("{:>width$} ", rel_num, width = width)
        };

        let num_style = if is_selected {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let text_span = if i == 0 {
            Span::styled(
                "Direct Messages",
                if is_selected {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                },
            )
        } else {
            let server_name = &app.store.servers[i - 1].name;
            Span::styled(
                server_name.as_str(),
                if is_selected {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                },
            )
        };

        items.push(ListItem::new(Line::from(vec![
            Span::styled(line_num_str, num_style),
            text_span,
        ])));
    }

    let mut state = ListState::default();
    state.select(Some(selected_index));

    let border_color = app.input_state.input_mode.color();

    let list = List::new(items)
        .block(
            Block::default()
                .title(" Servers ")
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
        Some(Action::Enter) => {
            if app.selected_index == 0 {
                app.selected_dm_index = 0;
                app.state = AppState::DmList;
                app.is_loading_dms = true;

                let users = app.store.users.clone();
                let api_client = app.api_client.clone();
                let app_tx = app.app_tx.clone();

                tokio::spawn(async move {
                    match crate::api::dm::fetch_dms(&api_client, &users).await {
                        Ok((dms, new_users)) => {
                            app_tx.send(AppEvent::DmsLoaded(dms, new_users)).await.ok();
                        }
                        Err(e) => {
                            error!("Error fetching DMs in background: {e}");
                        }
                    }
                });
            }
        }
        Some(Action::CursorUp) => {
            if app.selected_index > 0 {
                app.selected_index -= 1;
            }
        }
        Some(Action::CursorDown) => {
            let total_items = 1 + app.store.servers.len();
            if total_items > 0 && app.selected_index + 1 < total_items {
                app.selected_index += 1;
            }
        }
        Some(Action::GoToTopUI) => {
            app.selected_index = 0;
        }
        _ => {}
    }
}
