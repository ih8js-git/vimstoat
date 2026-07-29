use serde::{Deserialize, Serialize};
use serde_json::Value;

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
