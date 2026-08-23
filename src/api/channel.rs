use crate::{
    Result,
    api::client::{ApiClient, Endpoint},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageHistoryQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nearby: Option<String>,
}

pub async fn fetch_message_history(
    api_client: &ApiClient,
    channel_id: &str,
    query: Option<&MessageHistoryQuery>,
) -> Result<Vec<serde_json::Value>> {
    let mut path = format!("/channels/{}/messages", channel_id);
    if let Some(q) = query {
        let mut params = Vec::new();
        if let Some(limit) = q.limit {
            params.push(format!("limit={}", limit));
        }
        if let Some(before) = &q.before {
            params.push(format!("before={}", before));
        }
        if let Some(after) = &q.after {
            params.push(format!("after={}", after));
        }
        if let Some(sort) = &q.sort {
            params.push(format!("sort={}", sort));
        }
        if let Some(nearby) = &q.nearby {
            params.push(format!("nearby={}", nearby));
        }
        if !params.is_empty() {
            path.push_str("?");
            path.push_str(&params.join("&"));
        }
    }

    api_client.get(Endpoint::Custom(path)).await
}
