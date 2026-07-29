use crate::{
    Result,
    api::{
        WS_BASE_URL,
        events::{ClientEvent, ServerEvent},
    },
};
use futures_util::{SinkExt, StreamExt};
use log::{error, info};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message as WsMessage};

const OUTGOING_BUFFER_SIZE: usize = 32;
const INCOMING_BUFFER_SIZE: usize = 100;

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
