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

    let title = if let Some(channel) = app.dm_channels.get(app.selected_dm_index) {
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

    if app.current_dm_messages.is_empty() {
        let msg = Paragraph::new("No messages found. (Type :q to return)")
            .style(Style::default().fg(Color::DarkGray))
            .block(block);
        f.render_widget(msg, area);
        return;
    }

    let mut items = Vec::new();

    // Revolt API returns messages in descending order (newest first).
    // We reverse to render oldest at top and newest at bottom.
    for msg in app.current_dm_messages.iter().rev() {
        let author_id = msg
            .get("author")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown");

        let mut display_name = author_id.to_string();
        if let Ok(uid) = crate::cache::Id::<crate::models::User>::new(author_id)
            && let Ok(cache_lock) = app.cache.try_lock()
            && let Some(cached_user) = cache_lock.get(uid)
        {
            display_name = cached_user.username;
        }

        let content_str = if let Some(content) = msg.get("content").and_then(|v| v.as_str()) {
            content.to_string()
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

        let author_span = Span::styled(
            format!("{}: ", display_name),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
        let content_span = Span::raw(content_str);

        items.push(ListItem::new(Line::from(vec![author_span, content_span])));
    }

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}
