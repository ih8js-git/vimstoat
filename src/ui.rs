use crate::{
    app::{App, AppState},
    state::{Channel, Server},
};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    prelude::Rect,
    style::{Color, Style, Stylize},
    text::Span,
    widgets::{Block, Borders, List, ListItem, Paragraph, Widget},
};

pub fn render(f: &mut Frame, app: &App) {
    let area = f.area();

    match &app.state {
        AppState::InputToken => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(2)
                .constraints([
                    Constraint::Length(2),
                    Constraint::Length(3),
                    Constraint::Min(0),
                ])
                .split(area);

            let explanation = Paragraph::new(
                "We couldn't find your user_token. Please paste it below and press Enter to save it securely:",
            )
            .style(Style::default().fg(Color::Yellow));
            f.render_widget(explanation, chunks[0]);

            let input_block = Paragraph::new(app.input_text.as_str())
                .block(Block::default().title(" User Token ").borders(Borders::ALL));
            f.render_widget(input_block, chunks[1]);
        }
        AppState::LoggedIn => {
            let success_msg =
                Paragraph::new("Token found! You are logged in.\n\nPress 'q' to quit.")
                    .block(Block::default().title(" VimStoat ").borders(Borders::ALL));
            f.render_widget(success_msg, area);
        }
        AppState::Error(err_msg) => {
            let error_text = format!(
                "CRITICAL SYSTEM ERROR:\n\n{}\n\nPress any key to return...",
                err_msg
            );
            let error_msg = Paragraph::new(error_text)
                .style(Style::default().fg(Color::Red))
                .block(
                    Block::default()
                        .title(" Keyring Failure ")
                        .borders(Borders::ALL),
                );
            f.render_widget(error_msg, area);
        }
    }
}

pub fn render_state(f: &mut Frame, servers: &[Server]) {
    f.render_widget(
        AppWidget {
            servers,
            focused_server: 0,
            focused_channel: 1,
        },
        f.area(),
    )
}

struct AppWidget<'state> {
    // list of servers to show on the left hand side
    servers: &'state [Server],
    // of the servers, which one are we focused on for the pannel in the center
    focused_server: usize,
    // which channel inside the server are we looking at
    focused_channel: usize,
}

const SERVER_LIST_WIDTH: u16 = 20;

impl Widget for AppWidget<'_> {
    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let mut server_list_area = area;
        server_list_area.width = SERVER_LIST_WIDTH;
        let mut server_content_area = area;
        server_content_area.x += SERVER_LIST_WIDTH;
        server_content_area.width -= SERVER_LIST_WIDTH;

        List::new(self.servers.iter().enumerate().map(|(i, s)| {
            let item = ListItem::new(s.name.as_str());
            if self.focused_server == i {
                item.reversed()
            } else {
                item
            }
        }))
        .block(Block::bordered())
        .render(server_list_area, buf);
        ServerWidget {
            server: &self.servers[self.focused_server],
            focused_channel: self.focused_channel,
        }
        .render(server_content_area, buf);
    }
}

struct ServerWidget<'server> {
    server: &'server Server,
    focused_channel: usize,
}

impl Widget for ServerWidget<'_> {
    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let mut channel_list_area = area;
        channel_list_area.width = 20;
        let mut channel_area = area;
        channel_area.x += 20;
        channel_area.width -= 20;

        List::new(self.server.channels.iter().enumerate().map(|(i, c)| {
            let item = ListItem::new(c.name.as_str());
            if i == self.focused_channel {
                item.reversed()
            } else {
                item
            }
        }))
        .block(Block::bordered().title(self.server.name.as_str()))
        .render(channel_list_area, buf);
        ChannelWidget {
            channel: &self.server.channels[self.focused_channel],
        }
        .render(channel_area, buf);
    }
}

struct ChannelWidget<'channel> {
    channel: &'channel Channel,
}

impl Widget for ChannelWidget<'_> {
    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let mut messages_area = area;
        messages_area.height -= 5;
        let mut input_area = area;
        input_area.y = messages_area.height;
        input_area.height = 5;

        List::new(
            self.channel
                .messages
                .iter()
                .rev()
                .map(|msg| ListItem::new(msg.as_str())),
        )
        .direction(ratatui::widgets::ListDirection::BottomToTop)
        .block(Block::bordered().title(self.channel.name.as_str()))
        .render(messages_area, buf);
        Paragraph::new(self.channel.buffer.as_str())
            .block(Block::bordered())
            .render(input_area, buf);
    }
}
