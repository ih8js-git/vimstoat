use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Server {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct DirectMessageChannel {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub has_unread: bool,
    #[serde(skip)]
    pub typing_users: std::collections::HashSet<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
pub enum UserStatus {
    Online,
    Idle,
    Focus,
    DoNotDisturb,
    Invisible,
    #[default]
    Offline,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct User {
    pub id: String,
    pub username: String,
    #[serde(default)]
    pub status: UserStatus,
    #[serde(default)]
    pub status_text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    pub id: String,
    pub author_id: String,
    pub author_name: String,
    pub content: String,
}
