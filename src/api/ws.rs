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

pub fn handle(app: &mut App, event: ServerEvent) {
    debug!("Received WebSocket event: {event:?}");

    match event {
        ServerEvent::Ready { servers, .. } => handle_ready(app, servers),
        ServerEvent::Message(msg_val) => handle_message(app, msg_val),
        ServerEvent::MessageUpdate { id, channel, data } => {
            handle_message_update(app, id, channel, data)
        }
        ServerEvent::MessageDelete { id, channel } => handle_message_delete(app, id, channel),
        ServerEvent::ChannelStartTyping { id, user } => {
            app.app_tx
                .try_send(AppEvent::TypingStart {
                    channel_id: id,
                    user_id: user,
                })
                .ok();
        }
        ServerEvent::ChannelStopTyping { id, user } => {
            app.app_tx
                .try_send(AppEvent::TypingStop {
                    channel_id: id,
                    user_id: user,
                })
                .ok();
        }
        _ => {}
    }
}

fn handle_ready(app: &mut App, servers: Option<Vec<serde_json::Value>>) {
    if let Some(servers) = servers {
        for server_val in servers {
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
                app.store.servers.retain(|s| s.id != id_str);
                app.store.servers.push(server);
                info!("Stored server in memory: {id_str} => {name_str}");
            }
        }
    }
}

fn handle_message(app: &mut App, msg_val: serde_json::Value) {
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
                    .get::<serde_json::Value>(crate::api::client::Endpoint::User(author_id.clone()))
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

            let content = if let Some(content_val) = msg_val.get("content").and_then(|v| v.as_str())
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

fn handle_message_update(app: &mut App, id: String, channel: String, data: serde_json::Value) {
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

fn handle_message_delete(app: &mut App, id: String, channel: String) {
    app.app_tx
        .try_send(AppEvent::MessageDeleted {
            channel_id: channel,
            message_id: id,
        })
        .ok();
}
