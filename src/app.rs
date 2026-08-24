use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use log::{debug, error, info, warn};
use ratatui::crossterm::event::KeyEvent;
use tokio::sync::Mutex;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::time;

use crate::{
    Result,
    api::{
        API_BASE_URL,
        auth::Auth,
        client::ApiClient,
        events::{ClientEvent, ServerEvent},
        ws::WsClient,
    },
    cache::CacheStore,
    input::{InputMode, InputState},
    models::{DirectMessageChannel, Server},
};

pub enum AppEvent {
    DmsLoaded(Vec<DirectMessageChannel>, Vec<crate::models::User>),
    DmMessagesLoaded(Vec<crate::models::Message>, Vec<crate::models::User>),
    NewMessage {
        channel_id: String,
        message: crate::models::Message,
        new_user: Option<crate::models::User>,
    },
    MessageUpdated {
        channel_id: String,
        message_id: String,
        content: String,
    },
    MessageDeleted {
        channel_id: String,
        message_id: String,
    },
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
    pub users: std::collections::HashMap<String, crate::models::User>,
}

pub struct App {
    pub state: AppState,
    pub input_text: String,
    pub input_cursor: usize,
    pub yank_buffer: Option<String>,
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
            input_cursor: 0,
            yank_buffer: None,
            command_text: String::new(),
            auth,
            should_quit: false,
            api_base_url: api_base_url.unwrap_or(API_BASE_URL.to_string()),
            api_client,
            ws_client,
            ws_rx,
            cache: cache.clone(),
            store: AppStore {
                users: cache.lock().await.get_all_users(),
                ..Default::default()
            },
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
            AppEvent::DmsLoaded(dms, new_users) => {
                for user in new_users {
                    self.store.users.insert(user.id.clone(), user);
                }
                self.store.dm_channels = dms;
                self.is_loading_dms = false;
            }
            AppEvent::DmMessagesLoaded(messages, new_users) => {
                for user in new_users {
                    self.store.users.insert(user.id.clone(), user);
                }
                self.store.current_dm_messages = messages;
                self.is_loading_messages = false;
            }
            AppEvent::NewMessage {
                channel_id,
                message,
                new_user,
            } => {
                if let Some(user) = new_user {
                    self.store.users.insert(user.id.clone(), user);
                }

                let is_active_channel = matches!(self.state, AppState::Dm)
                    && self
                        .store
                        .dm_channels
                        .get(self.selected_dm_index)
                        .map(|c| &c.id)
                        == Some(&channel_id);

                if is_active_channel {
                    self.store.current_dm_messages.insert(0, message.clone()); // newest is at 0 (rev order in UI)
                }

                if let Some(channel) = self
                    .store
                    .dm_channels
                    .iter_mut()
                    .find(|c| c.id == channel_id)
                {
                    if !is_active_channel {
                        channel.has_unread = true;
                    }
                    channel.last_message_preview = Some(message.content);
                }
            }
            AppEvent::MessageUpdated {
                channel_id,
                message_id,
                content,
            } => {
                let is_active_channel = matches!(self.state, AppState::Dm)
                    && self
                        .store
                        .dm_channels
                        .get(self.selected_dm_index)
                        .map(|c| &c.id)
                        == Some(&channel_id);

                if is_active_channel {
                    if let Some(msg) = self
                        .store
                        .current_dm_messages
                        .iter_mut()
                        .find(|m| m.id == message_id)
                    {
                        msg.content = content;
                    }
                }
            }
            AppEvent::MessageDeleted {
                channel_id,
                message_id,
            } => {
                let is_active_channel = matches!(self.state, AppState::Dm)
                    && self
                        .store
                        .dm_channels
                        .get(self.selected_dm_index)
                        .map(|c| &c.id)
                        == Some(&channel_id);

                if is_active_channel {
                    self.store
                        .current_dm_messages
                        .retain(|m| m.id != message_id);
                }
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

    pub fn set_input_mode(&mut self, new_mode: InputMode) {
        self.input_state.change_input_mode(new_mode);
        let style = match new_mode {
            InputMode::Insert | InputMode::Command => {
                ratatui::crossterm::cursor::SetCursorStyle::BlinkingBar
            }
            _ => ratatui::crossterm::cursor::SetCursorStyle::BlinkingBlock,
        };
        let _ = ratatui::crossterm::execute!(std::io::stdout(), style);
    }

    pub async fn handle_key_event(&mut self, key: KeyEvent) -> Result<()> {
        if matches!(self.input_state.input_mode, InputMode::Command) {
            crate::handlers::command::handle(self, key);
            return Ok(());
        }

        match self.state {
            AppState::InputToken => crate::handlers::input_token::handle(self, key).await,
            AppState::ValidatingToken => {}
            AppState::LoggedIn => crate::handlers::logged_in::handle(self, key),
            AppState::DmList => crate::handlers::dm_list::handle(self, key),
            AppState::Dm => crate::handlers::dm::handle(self, key),
            AppState::Error(_) => crate::handlers::error::handle(self, key),
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
