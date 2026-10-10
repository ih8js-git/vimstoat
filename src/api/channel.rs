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
    let mut path = format!("/channels/{channel_id}/messages");
    if let Some(q) = query {
        let mut params = Vec::new();
        if let Some(limit) = q.limit {
            params.push(format!("limit={limit}"));
        }
        if let Some(before) = &q.before {
            params.push(format!("before={before}"));
        }
        if let Some(after) = &q.after {
            params.push(format!("after={after}"));
        }
        if let Some(sort) = &q.sort {
            params.push(format!("sort={sort}"));
        }
        if let Some(nearby) = &q.nearby {
            params.push(format!("nearby={nearby}"));
        }
        if !params.is_empty() {
            path.push('?');
            path.push_str(&params.join("&"));
        }
    }

    api_client.get(Endpoint::Custom(path)).await
}

pub async fn ack_message(api_client: &ApiClient, channel_id: &str, message_id: &str) -> Result<()> {
    api_client
        .put_empty(Endpoint::AckMessage {
            channel_id: channel_id.to_string(),
            message_id: message_id.to_string(),
        })
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{json_str, keyring_client};
    use serde_json::Value;
    use tokio::sync::OnceCell;

    /// A real DM with at least `MIN_MESSAGES` messages, plus its latest page.
    struct Live {
        client: ApiClient,
        channel_id: String,
        last_message_id: String,
        latest: Vec<Value>,
    }

    const MIN_MESSAGES: usize = 5;

    static LIVE: OnceCell<Live> = OnceCell::const_new();

    async fn live() -> &'static Live {
        LIVE.get_or_init(|| async {
            let client = keyring_client("channel").await;
            let dms = client
                .get::<Vec<Value>>(Endpoint::Dms)
                .await
                .expect("GET /users/dms should succeed with the real token");

            let query = MessageHistoryQuery {
                limit: Some(MIN_MESSAGES),
                before: None,
                after: None,
                sort: None,
                nearby: None,
            };
            for dm in &dms {
                let Some(last_message_id) = dm.get("last_message_id").and_then(Value::as_str)
                else {
                    continue;
                };
                let channel_id = json_str(dm, "_id").to_string();
                let latest = fetch_message_history(&client, &channel_id, Some(&query))
                    .await
                    .expect("fetching history of a real DM should succeed");

                if latest.len() == MIN_MESSAGES {
                    return Live {
                        client,
                        channel_id,
                        last_message_id: last_message_id.to_string(),
                        latest,
                    };
                }
            }
            panic!("channel tests need a DM with at least {MIN_MESSAGES} messages");
        })
        .await
    }

    fn query() -> MessageHistoryQuery {
        MessageHistoryQuery {
            limit: None,
            before: None,
            after: None,
            sort: None,
            nearby: None,
        }
    }

    fn ids(messages: &[Value]) -> Vec<&str> {
        messages.iter().map(|m| json_str(m, "_id")).collect()
    }

    #[tokio::test]
    async fn test_default_order_is_newest_first() {
        let live = live().await;
        let ids = ids(&live.latest);

        // The DM view acks and displays `messages[0]` as the latest message.
        assert_eq!(ids[0], live.last_message_id);
        assert!(
            ids.windows(2).all(|w| w[0] > w[1]),
            "default history is not sorted newest first"
        );
    }

    #[tokio::test]
    async fn test_messages_have_fields_the_dm_view_reads() {
        let live = live().await;

        for msg in &live.latest {
            assert_eq!(json_str(msg, "channel"), live.channel_id);
            assert!(!json_str(msg, "author").is_empty(), "`author` is empty");
            assert!(
                msg.get("content").is_some_and(Value::is_string) || msg.get("system").is_some(),
                "message {} has neither `content` nor `system`",
                json_str(msg, "_id")
            );
        }
    }

    #[tokio::test]
    async fn test_limit_caps_page_size() {
        let live = live().await;
        let q = MessageHistoryQuery {
            limit: Some(2),
            ..query()
        };

        let page = fetch_message_history(&live.client, &live.channel_id, Some(&q))
            .await
            .unwrap();

        assert_eq!(ids(&page), ids(&live.latest)[..2]);
    }

    #[tokio::test]
    async fn test_before_pages_backwards() {
        let live = live().await;
        let pivot = json_str(&live.latest[1], "_id");
        let q = MessageHistoryQuery {
            limit: Some(2),
            before: Some(pivot.to_string()),
            ..query()
        };

        let page = fetch_message_history(&live.client, &live.channel_id, Some(&q))
            .await
            .unwrap();

        assert_eq!(ids(&page), ids(&live.latest)[2..4]);
    }

    #[tokio::test]
    async fn test_after_with_oldest_sort_pages_forwards() {
        let live = live().await;
        let pivot = json_str(&live.latest[3], "_id");
        let q = MessageHistoryQuery {
            limit: Some(2),
            after: Some(pivot.to_string()),
            sort: Some("Oldest".to_string()),
            ..query()
        };

        let page = fetch_message_history(&live.client, &live.channel_id, Some(&q))
            .await
            .unwrap();

        let expected: Vec<&str> = ids(&live.latest)[1..3].iter().rev().copied().collect();
        assert_eq!(ids(&page), expected);
    }

    #[tokio::test]
    async fn test_nearby_includes_target_message() {
        let live = live().await;
        let target = json_str(&live.latest[2], "_id");
        let q = MessageHistoryQuery {
            limit: Some(3),
            nearby: Some(target.to_string()),
            ..query()
        };

        let page = fetch_message_history(&live.client, &live.channel_id, Some(&q))
            .await
            .unwrap();

        assert!(ids(&page).contains(&target));
    }

    #[tokio::test]
    async fn test_empty_query_matches_no_query() {
        let live = live().await;

        let none = fetch_message_history(&live.client, &live.channel_id, None)
            .await
            .unwrap();
        let empty = fetch_message_history(&live.client, &live.channel_id, Some(&query()))
            .await
            .expect("an all-None query must not produce a malformed URL");

        assert_eq!(ids(&empty), ids(&none));
        assert_eq!(ids(&none)[..MIN_MESSAGES], ids(&live.latest));
    }

    #[tokio::test]
    async fn test_fetch_history_of_unknown_channel_fails() {
        let live = live().await;

        let result = fetch_message_history(&live.client, "01AAAAAAAAAAAAAAAAAAAAAAAA", None).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_ack_already_read_message_keeps_read_state() {
        let live = live().await;
        let unreads_of = |unreads: &[Value]| -> Option<Value> {
            unreads
                .iter()
                .find(|u| json_str(&u["_id"], "channel") == live.channel_id)
                .cloned()
        };

        let before = unreads_of(
            &live
                .client
                .get::<Vec<Value>>(Endpoint::SyncUnreads)
                .await
                .unwrap(),
        );

        // Only ack when the channel is already fully read, so the real read
        // state is left exactly as it was.
        let Some(before) = before.filter(|u| {
            u["last_id"].as_str() == Some(&live.last_message_id)
                && u["mentions"].as_array().is_none_or(Vec::is_empty)
        }) else {
            return;
        };

        ack_message(&live.client, &live.channel_id, &live.last_message_id)
            .await
            .expect("acking a real message should succeed");

        let after = unreads_of(
            &live
                .client
                .get::<Vec<Value>>(Endpoint::SyncUnreads)
                .await
                .unwrap(),
        );
        assert_eq!(
            after.as_ref().map(|u| &u["last_id"]),
            Some(&before["last_id"])
        );
    }

    #[tokio::test]
    async fn test_ack_unknown_channel_fails() {
        let live = live().await;

        let result = ack_message(
            &live.client,
            "01AAAAAAAAAAAAAAAAAAAAAAAA",
            &live.last_message_id,
        )
        .await;

        assert!(result.is_err());
    }
}
