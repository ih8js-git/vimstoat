use crate::{
    Result,
    api::{
        WS_BASE_URL,
        events::{ClientEvent, ServerEvent},
    },
    app::{App, AppEvent},
    models::{Message, Server, User},
};
use futures_util::{SinkExt, StreamExt};
use log::{debug, error, info};
use serde_json::Value;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message as WsMessage};

const OUTGOING_BUFFER_SIZE: usize = 32;
const INCOMING_BUFFER_SIZE: usize = 100;

pub struct WsClient {
    tx_outgoing: mpsc::Sender<ClientEvent>,
}

impl WsClient {
    pub async fn connect(base_url: Option<String>) -> Result<(Self, mpsc::Receiver<ServerEvent>)> {
        let url = base_url.unwrap_or_else(|| WS_BASE_URL.to_string());
        let (ws_stream, _) = connect_async(&url).await?;
        let (mut write, mut read) = ws_stream.split();

        let (tx_outgoing, mut rx_outgoing) = mpsc::channel::<ClientEvent>(OUTGOING_BUFFER_SIZE);
        let (tx_incoming, rx_incoming) = mpsc::channel::<ServerEvent>(INCOMING_BUFFER_SIZE);

        tokio::spawn(async move {
            while let Some(msg) = read.next().await {
                match msg {
                    Ok(WsMessage::Text(text)) => match serde_json::from_str::<ServerEvent>(&text) {
                        Ok(event) => {
                            Self::dispatch_event(event, &tx_incoming).await;
                        }
                        Err(e) => {
                            error!("Error deserializing ServerEvent: {e}\nRaw data: {text}");
                            break;
                        }
                    },
                    Ok(WsMessage::Close(_)) => {
                        info!("WS Connection closed by server.");
                        break;
                    }
                    Err(e) => {
                        error!("WS Error: {e}");
                        break;
                    }
                    _ => {}
                }
            }
        });

        tokio::spawn(async move {
            while let Some(event) = rx_outgoing.recv().await {
                if let Ok(json) = serde_json::to_string(&event)
                    && let Err(e) = write.send(WsMessage::Text(json.into())).await
                {
                    error!("Error sending WsMessage: {e}");
                    break;
                }
            }
        });

        Ok((Self { tx_outgoing }, rx_incoming))
    }

    pub async fn send_event(&self, event: ClientEvent) -> Result<()> {
        self.tx_outgoing.send(event).await.map_err(|e| e.into())
    }

    pub fn clone_sender(&self) -> mpsc::Sender<ClientEvent> {
        self.tx_outgoing.clone()
    }

    pub async fn dispatch_event(event: ServerEvent, tx: &mpsc::Sender<ServerEvent>) {
        if let ServerEvent::Bulk { v } = event {
            for sub_event in v {
                Box::pin(Self::dispatch_event(sub_event, tx)).await;
            }
        } else {
            tx.send(event).await.ok();
        }
    }
}

pub struct EventHandler<'a> {
    servers: &'a mut Vec<Server>,
}

impl<'a> EventHandler<'a> {
    pub fn new(servers: &'a mut Vec<Server>) -> Self {
        Self { servers }
    }

    pub fn handle_event(&mut self, event: &ServerEvent) {
        #[allow(clippy::single_match)]
        match event {
            ServerEvent::Ready { servers, .. } => {
                self.handle_ready(servers.as_deref());
            }
            _ => {}
        }
    }

    fn handle_ready(&mut self, servers: Option<&[Value]>) {
        if let Some(servers) = servers {
            for server_val in servers {
                self.handle_server(server_val);
            }
        }
    }

    fn handle_server(&mut self, server_val: &Value) {
        let id = server_val
            .get("_id")
            .or_else(|| server_val.get("id"))
            .and_then(|v| v.as_str());
        let name = server_val.get("name").and_then(|v| v.as_str());
        let description = server_val
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        if let (Some(id_str), Some(name_str)) = (id, name) {
            let server = Server {
                id: id_str.to_string(),
                name: name_str.to_string(),
                description,
            };
            self.servers.retain(|s| s.id != id_str);
            self.servers.push(server);
            info!("Stored server in memory: {id_str} => {name_str}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_handle_ready_event_servers_in_memory() {
        let mut servers = Vec::new();

        let server_id = "01KXCTGX37FXG9CASWC35R3S21";
        let server_name = "vimstoat";

        let ready_event = ServerEvent::Ready {
            users: None,
            servers: Some(vec![json!({
                "_id": server_id,
                "name": server_name,
                "description": null
            })]),
            channels: None,
            members: None,
            emojis: None,
            user_settings: None,
            channel_unreads: None,
            policy_changes: None,
        };

        let mut handler = EventHandler::new(&mut servers);
        handler.handle_event(&ready_event);

        assert_eq!(servers.len(), 1, "Server should be stored in memory");
        assert_eq!(servers[0].id, server_id);
        assert_eq!(servers[0].name, server_name);
        assert_eq!(servers[0].description, None);
    }
}

pub fn handle(app: &mut App, event: ServerEvent) {
    debug!("Received WebSocket event: {event:?}");
    EventHandler::new(&mut app.store.servers).handle_event(&event);

    match event {
        ServerEvent::Message(msg_val) => {
            let api_client = app.api_client.clone();
            let app_tx = app.app_tx.clone();
            let local_users = app.store.users.clone();

            tokio::spawn(async move {
                if let Some(channel_id) = msg_val.get("channel").and_then(|v| v.as_str()) {
                    let id = msg_val
                        .get("_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string();
                    let author_id = msg_val
                        .get("author")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown")
                        .to_string();
                    let channel_id = channel_id.to_string();

                    let mut author_name = author_id.clone();
                    let mut new_user_fetched = None;

                    if let Some(user) = local_users.get(&author_id) {
                        author_name = user.username.clone();
                    } else if author_id != "Unknown"
                        && let Ok(user_val) = api_client
                            .get::<serde_json::Value>(crate::api::client::Endpoint::User(
                                author_id.clone(),
                            ))
                            .await
                        && let Some(username) = user_val.get("username").and_then(|v| v.as_str())
                    {
                        author_name = username.to_string();
                        let new_user = User {
                            id: author_id.clone(),
                            username: username.to_string(),
                        };
                        new_user_fetched = Some(new_user);
                    }

                    let content = if let Some(content_val) =
                        msg_val.get("content").and_then(|v| v.as_str())
                    {
                        content_val.to_string()
                    } else if let Some(sys) = msg_val.get("system") {
                        format!(
                            "[System message: {}]",
                            sys.get("type")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown")
                        )
                    } else {
                        "[Unsupported message]".to_string()
                    };

                    let message = Message {
                        id,
                        author_id,
                        author_name,
                        content,
                    };

                    app_tx
                        .send(AppEvent::NewMessage {
                            channel_id,
                            message,
                            new_user: new_user_fetched,
                        })
                        .await
                        .ok();
                }
            });
        }
        ServerEvent::MessageUpdate { id, channel, data } => {
            if let Some(content) = data.get("content").and_then(|v| v.as_str()) {
                app.app_tx
                    .try_send(AppEvent::MessageUpdated {
                        channel_id: channel,
                        message_id: id,
                        content: content.to_string(),
                    })
                    .ok();
            }
        }
        ServerEvent::MessageDelete { id, channel } => {
            app.app_tx
                .try_send(AppEvent::MessageDeleted {
                    channel_id: channel,
                    message_id: id,
                })
                .ok();
        }
        _ => {}
    }
}
