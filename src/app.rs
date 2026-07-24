use std::time::{Duration, SystemTime, UNIX_EPOCH};

use log::{debug, error, info, warn};
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use tokio::sync::mpsc::Receiver;
use tokio::time;

use crate::{
    Result,
    action::Action,
    api::{
        API_BASE_URL,
        auth::Auth,
        client::ApiClient,
        events::{ClientEvent, ServerEvent, WsClient},
    },
    cache::CacheStore,
    input::InputState,
};

pub enum AppState {
    InputToken,
    ValidatingToken,
    LoggedIn,
    Error(anyhow::Error),
}

pub struct App {
    pub state: AppState,
    pub input_text: String,
    pub auth: Auth,
    pub should_quit: bool,
    pub input_state: InputState,
    pub api_base_url: String,
    pub api_client: ApiClient,
    pub ws_client: WsClient,
    pub ws_rx: Receiver<ServerEvent>,
    #[allow(unused)]
    pub cache: CacheStore,
}

impl App {
    pub async fn new(api_base_url: Option<String>, ws_base_url: Option<String>) -> Result<Self> {
        let auth = Auth::new().map_err(|e| anyhow::anyhow!(e))?;

        let mut api_client = ApiClient::new(String::new(), api_base_url.clone());

        let state = if let Ok(token) = auth.token_entry.get_secret().await {
            match auth.validate_token(&token, api_base_url.clone()).await {
                Ok(authenticated_client) => {
                    api_client = authenticated_client;
                    AppState::LoggedIn
                }
                Err(e) => AppState::Error(e),
            }
        } else {
            AppState::InputToken
        };

        let (ws_client, ws_rx) = WsClient::connect(ws_base_url).await?;

        let cache = CacheStore::new()?;

        Ok(Self {
            state,
            input_text: String::new(),
            auth,
            should_quit: false,
            api_base_url: api_base_url.unwrap_or(API_BASE_URL.to_string()),
            api_client,
            ws_client,
            ws_rx,
            cache,
            input_state: InputState::default(),
        })
    }

    pub async fn handle_key_event(&mut self, key: KeyEvent) -> Result<()> {
        match self.state {
            AppState::InputToken => match key.code {
                KeyCode::Enter => {
                    if !self.input_text.is_empty() {
                        self.state = AppState::ValidatingToken;
                        match self
                            .auth
                            .validate_token(&self.input_text, Some(self.api_base_url.clone()))
                            .await
                        {
                            Ok(client) => match self.auth.store_token(&self.input_text).await {
                                Ok(_) => {
                                    self.api_client = client;
                                    self.state = AppState::LoggedIn;
                                }
                                Err(detailed_err) => {
                                    self.state = AppState::Error(detailed_err);
                                }
                            },
                            Err(e) => {
                                self.state = AppState::Error(e);
                            }
                        }
                    }
                }
                KeyCode::Char(c) => {
                    self.input_text.push(c);
                }
                KeyCode::Backspace => {
                    self.input_text.pop();
                }
                KeyCode::Esc => {
                    self.should_quit = true;
                }
                _ => {}
            },
            AppState::ValidatingToken => {}
            AppState::LoggedIn => {
                let action = self.input_state.process_key_event(key);
                if let Some(Action::Quit) = action {
                    self.should_quit = true;
                };
            }
            AppState::Error(_) => {
                if matches!(key.code, KeyCode::Char(_) | KeyCode::Esc | KeyCode::Enter) {
                    self.state = AppState::InputToken;
                }
            }
        }
        Ok(())
    }

    pub async fn authenticate_ws(&mut self, token: &str) -> Result<()> {
        self.ws_client
            .send_event(ClientEvent::Authenticate {
                token: token.into(),
            })
            .await?;

        let mut is_authenticated = false;
        while let Some(event) = self.ws_rx.recv().await {
            match event {
                ServerEvent::Authenticated => {
                    info!("Successfully authenticated!");
                    is_authenticated = true;
                    break;
                }
                ServerEvent::Error { error } => {
                    error!("Error authenticating: {error}");
                    return Ok(());
                }
                _ => {}
            }
        }

        if is_authenticated {
            let tx_ping = self.ws_client.clone_sender();

            tokio::spawn(async move {
                let mut interval = time::interval(Duration::from_secs(20));

                loop {
                    interval.tick().await;

                    let timestamp = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u64;

                    if tx_ping
                        .send(ClientEvent::Ping { data: timestamp })
                        .await
                        .is_err()
                    {
                        warn!("Stopped pinging: channel closed.");
                        break;
                    }
                }
            });

            debug!("Started pinging every 20s.");
        }

        Ok(())
    }
}
