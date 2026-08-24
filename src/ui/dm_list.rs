use crate::app::App;
use ratatui::{
    Frame,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

pub fn render(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let total_items = app.store.dm_channels.len();

    let border_color = if matches!(app.input_state.input_mode, crate::input::InputMode::Command) {
        Color::Green
    } else {
        Color::Reset
    };

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

        let text_style = if is_selected {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Cyan)
        };

        items.push(ListItem::new(Line::from(vec![
            Span::styled(line_num_str, num_style),
            Span::styled(channel.name.as_str(), text_style),
        ])));
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
