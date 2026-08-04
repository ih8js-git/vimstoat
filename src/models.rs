use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Server {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}
