use futures_util::{SinkExt, StreamExt};
use log::{error, info};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message as WsMessage};

use crate::{Result, api::WS_BASE_URL};

const OUTGOING_BUFFER_SIZE: usize = 32;
const INCOMING_BUFFER_SIZE: usize = 100;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
#[allow(unused)]
pub enum ClientEvent {
    Authenticate { token: String },
    BeginTyping { channel: String },
    EndTyping { channel: String },
    Ping { data: u64 },
    Subscribe { server_id: String },
}

#[derive(Debug, Clone, Deserialize)]
#[allow(unused)]
pub struct ServerMemberId {
    pub server: String,
    pub user: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "event_type")]
#[allow(unused)]
pub enum AuthEvent {
    DeleteSession {
        user_id: String,
        session_id: String,
    },
    DeleteAllSessions {
        user_id: String,
        exclude_session_id: Option<String>,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
#[allow(unused)]
pub enum ServerEvent {
    Error {
        error: String,
    },
    Authenticated,
    Logout,
    Bulk {
        v: Vec<ServerEvent>,
    },
    Pong {
        data: Value,
    },
    Ready {
        users: Option<Vec<Value>>,
        servers: Option<Vec<Value>>,
        channels: Option<Vec<Value>>,
        members: Option<Vec<Value>>,
        emojis: Option<Vec<Value>>,
        user_settings: Option<Vec<Value>>,
        channel_unreads: Option<Vec<Value>>,
        policy_changes: Option<Vec<Value>>,
    },
    Message(Value),
    MessageUpdate {
        id: String,
        channel: String,
        data: Value,
    },
    MessageAppend {
        id: String,
        channel: String,
        append: Value,
    },
    MessageDelete {
        id: String,
        channel: String,
    },
    MessageReact {
        id: String,
        channel_id: String,
        user_id: String,
        emoji_id: String,
    },
    MessageUnreact {
        id: String,
        channel_id: String,
        user_id: String,
        emoji_id: String,
    },
    MessageRemoveReaction {
        id: String,
        channel_id: String,
        emoji_id: String,
    },
    ChannelCreate(Value),
    ChannelUpdate {
        id: String,
        data: Value,
        clear: Option<Vec<String>>,
    },
    ChannelDelete {
        id: String,
    },
    ChannelGroupJoin {
        id: String,
        user: String,
    },
    ChannelGroupLeave {
        id: String,
        user: String,
    },
    ChannelStartTyping {
        id: String,
        user: String,
    },
    ChannelStopTyping {
        id: String,
        user: String,
    },
    ChannelAck {
        id: String,
        user: String,
        message_id: String,
    },
    ServerCreate(Value),
    ServerUpdate {
        id: String,
        data: Value,
        clear: Option<Vec<String>>,
    },
    ServerDelete {
        id: String,
    },
    ServerMemberUpdate {
        id: ServerMemberId,
        data: Value,
        clear: Option<Vec<String>>,
    },
    ServerMemberJoin {
        id: String,
        user: String,
        member: Value,
    },
    ServerMemberLeave {
        id: String,
        user: String,
    },
    ServerRoleUpdate {
        id: String,
        role_id: String,
        data: Value,
        clear: Option<Vec<String>>,
    },
    ServerRoleDelete {
        id: String,
        role_id: String,
    },
    UserUpdate {
        id: String,
        data: Value,
        clear: Option<Vec<String>>,
    },
    UserRelationship {
        id: String,
        user: Value,
        status: String,
    },
    UserPlatformWipe {
        user_id: String,
        flags: Value,
    },
    EmojiCreate(Value),
    EmojiUpdate {
        id: String,
        data: Value,
    },
    EmojiDelete {
        id: String,
    },
    Auth(AuthEvent),
}

pub struct WsClient {
    tx_outgoing: mpsc::Sender<ClientEvent>,
}

impl WsClient {
    pub async fn connect(base_url: Option<String>) -> Result<(Self, mpsc::Receiver<ServerEvent>)> {
        let (ws_stream, _) =
            connect_async(base_url.unwrap_or(WS_BASE_URL.to_string()).as_str()).await?;
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
                            error!("Error deserializing ServerEvent: {e}\nBrut data: {text}");
                            break;
                        }
                    },
                    Ok(WsMessage::Close(_)) => {
                        info!("WS Connexion closed by server.");
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
