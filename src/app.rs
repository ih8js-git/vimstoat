use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use log::{debug, error, info, warn};
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use tokio::sync::Mutex;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::time;

use crate::{
    Result,
    action::Action,
    api::{
        API_BASE_URL,
        auth::Auth,
        client::ApiClient,
        events::{ClientEvent, ServerEvent},
        ws::WsClient,
    },
    cache::CacheStore,
    command::Command,
    input::{InputMode, InputState},
    models::{DirectMessageChannel, Server},
};

pub enum AppEvent {
    DmsLoaded(Vec<DirectMessageChannel>),
    DmMessagesLoaded(Vec<crate::models::Message>),
}

pub enum AppState {
    InputToken,
    ValidatingToken,
    LoggedIn,
    DmList,
    Dm,
    Error(anyhow::Error),
}

#[derive(Default)]
pub struct AppStore {
    pub servers: Vec<Server>,
    pub dm_channels: Vec<DirectMessageChannel>,
    pub current_dm_messages: Vec<crate::models::Message>,
}

pub struct App {
    pub state: AppState,
    pub input_text: String,
    pub command_text: String,
    pub auth: Auth,
    pub should_quit: bool,
    pub input_state: InputState,
    pub api_base_url: String,
    pub api_client: ApiClient,
    pub ws_client: WsClient,
    pub ws_rx: Receiver<ServerEvent>,
    pub cache: Arc<Mutex<CacheStore>>,
    pub store: AppStore,
    pub selected_index: usize,
    pub selected_dm_index: usize,
    pub is_loading_dms: bool,
    pub is_loading_messages: bool,
    pub app_tx: Sender<AppEvent>,
    pub app_rx: Receiver<AppEvent>,
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

        let cache = Arc::new(Mutex::new(CacheStore::new()?));
        let (app_tx, app_rx) = mpsc::channel::<AppEvent>(32);

        Ok(Self {
            state,
            input_text: String::new(),
            command_text: String::new(),
            auth,
            should_quit: false,
            api_base_url: api_base_url.unwrap_or(API_BASE_URL.to_string()),
            api_client,
            ws_client,
            ws_rx,
            cache,
            store: AppStore::default(),
            selected_index: 0,
            selected_dm_index: 0,
            is_loading_dms: false,
            is_loading_messages: false,
            app_tx,
            app_rx,
            input_state: InputState::default(),
        })
    }

    pub fn handle_app_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::DmsLoaded(dms) => {
                self.store.dm_channels = dms;
                self.is_loading_dms = false;
            }
            AppEvent::DmMessagesLoaded(messages) => {
                self.store.current_dm_messages = messages;
                self.is_loading_messages = false;
            }
        }
    }

    pub fn go_back_or_quit(&mut self) {
        match self.state {
            AppState::DmList => self.state = AppState::LoggedIn,
            AppState::Dm => self.state = AppState::DmList,
            _ => self.should_quit = true,
        }
    }

    pub async fn handle_key_event(&mut self, key: KeyEvent) -> Result<()> {
        if matches!(self.input_state.input_mode, InputMode::Command) {
            let action = self.input_state.process_key_event(key);
            match action {
                Some(Action::AppendCharacter(c)) => {
                    self.command_text.push(c);
                }
                Some(Action::RemoveCharacter) => {
                    self.command_text.pop();
                }
                Some(Action::Escape) => {
                    self.command_text.clear();
                    self.input_state.change_input_mode(InputMode::UI);
                }
                Some(Action::Enter) => {
                    if let Some(cmd) = Command::parse(&self.command_text) {
                        cmd.execute(self);
                    }
                    self.command_text.clear();
                    self.input_state.change_input_mode(InputMode::UI);
                }
                _ => {}
            }
            return Ok(());
        }
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
                match action {
                    Some(Action::Quit) => self.should_quit = true,
                    Some(Action::EnterCommandMode) => {
                        self.command_text.clear();
                        self.input_state.change_input_mode(InputMode::Command);
                    }
                    Some(Action::Enter) => {
                        if self.selected_index == 0 {
                            self.selected_dm_index = 0;
                            self.state = AppState::DmList;
                            self.is_loading_dms = true;

                            let cache = self.cache.clone();
                            let api_client = self.api_client.clone();
                            let app_tx = self.app_tx.clone();

                            tokio::spawn(async move {
                                match crate::api::dm::fetch_dms(&api_client, cache).await {
                                    Ok(dms) => {
                                        app_tx.send(AppEvent::DmsLoaded(dms)).await.ok();
                                    }
                                    Err(e) => {
                                        error!("Error fetching DMs in background: {e}");
                                    }
                                }
                            });
                        }
                    }
                    Some(Action::CursorUp) => {
                        if self.selected_index > 0 {
                            self.selected_index -= 1;
                        }
                    }
                    Some(Action::CursorDown) => {
                        let total_items = 1 + self.store.servers.len();
                        if total_items > 0 && self.selected_index + 1 < total_items {
                            self.selected_index += 1;
                        }
                    }
                    Some(Action::GoToTopUI) => {
                        self.selected_index = 0;
                    }
                    _ => {}
                }
            }
            AppState::DmList => {
                let action = self.input_state.process_key_event(key);
                match action {
                    Some(Action::Quit) => self.should_quit = true,
                    Some(Action::EnterCommandMode) => {
                        self.command_text.clear();
                        self.input_state.change_input_mode(InputMode::Command);
                    }
                    Some(Action::CursorUp) => {
                        if self.selected_dm_index > 0 {
                            self.selected_dm_index -= 1;
                        }
                    }
                    Some(Action::CursorDown) => {
                        let total_items = self.store.dm_channels.len();
                        if total_items > 0 && self.selected_dm_index + 1 < total_items {
                            self.selected_dm_index += 1;
                        }
                    }
                    Some(Action::GoToTopUI) => {
                        self.selected_dm_index = 0;
                    }
                    Some(Action::Enter) if !self.store.dm_channels.is_empty() => {
                        let channel_id = self.store.dm_channels[self.selected_dm_index].id.clone();
                        self.state = AppState::Dm;
                        self.is_loading_messages = true;
                        self.store.current_dm_messages.clear();

                        let api_client = self.api_client.clone();
                        let app_tx = self.app_tx.clone();
                        let cache = self.cache.clone();

                        tokio::spawn(async move {
                            let query = crate::api::channel::MessageHistoryQuery {
                                limit: Some(50),
                                before: None,
                                after: None,
                                sort: None,
                                nearby: None,
                            };
                            match crate::api::channel::fetch_message_history(
                                &api_client,
                                &channel_id,
                                Some(&query),
                            )
                            .await
                            {
                                Ok(messages_json) => {
                                    let mut parsed_messages =
                                        Vec::with_capacity(messages_json.len());
                                    let cache_locked = cache.lock().await;

                                    for msg in messages_json {
                                        let id = msg
                                            .get("_id")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("unknown")
                                            .to_string();

                                        let author_id = msg
                                            .get("author")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("Unknown")
                                            .to_string();

                                        let mut author_name = author_id.clone();
                                        if let Ok(uid) =
                                            crate::cache::Id::<crate::models::User>::new(&author_id)
                                            && let Some(cached_user) = cache_locked.get(uid)
                                        {
                                            author_name = cached_user.username;
                                        }

                                        let content = if let Some(content_val) =
                                            msg.get("content").and_then(|v| v.as_str())
                                        {
                                            content_val.to_string()
                                        } else if let Some(sys) = msg.get("system") {
                                            format!(
                                                "[System message: {}]",
                                                sys.get("type")
                                                    .and_then(|v| v.as_str())
                                                    .unwrap_or("unknown")
                                            )
                                        } else {
                                            "[Unsupported message]".to_string()
                                        };

                                        parsed_messages.push(crate::models::Message {
                                            id,
                                            author_id,
                                            author_name,
                                            content,
                                        });
                                    }

                                    app_tx
                                        .send(AppEvent::DmMessagesLoaded(parsed_messages))
                                        .await
                                        .ok();
                                }
                                Err(e) => {
                                    error!("Error fetching messages: {e}");
                                    app_tx
                                        .send(AppEvent::DmMessagesLoaded(Vec::new()))
                                        .await
                                        .ok();
                                }
                            }
                        });
                    }
                    _ => {}
                }
            }
            AppState::Dm => {
                let action = self.input_state.process_key_event(key);
                match action {
                    Some(Action::Quit) => self.should_quit = true,
                    Some(Action::EnterCommandMode) => {
                        self.command_text.clear();
                        self.input_state.change_input_mode(InputMode::Command);
                    }
                    _ => {}
                }
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
