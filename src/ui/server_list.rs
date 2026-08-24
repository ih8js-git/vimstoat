use crate::app::App;
use ratatui::{
    Frame,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
};

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
