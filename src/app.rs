use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use log::{debug, error, info, warn};
use ratatui::crossterm::event::{Event, KeyEvent};
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
    cache::{CacheStore, Id},
    input::{InputMode, InputState},
    models::{DirectMessageChannel, Server},
};

pub enum AppEvent {
    DmsLoaded(Vec<DirectMessageChannel>, Vec<crate::models::User>),
    DmMessagesLoaded(
        String,
        Vec<crate::models::Message>,
        Vec<crate::models::User>,
    ),
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
    TypingStart {
        channel_id: String,
        user_id: String,
    },
    TypingStop {
        channel_id: String,
        user_id: String,
    },
    UsersRefreshed(Vec<crate::models::User>),
    ChannelAcked {
        channel_id: String,
        message_id: String,
    },
}

pub enum AppState {
    NeedsAuth,
    ValidationToken,
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
    pub async fn new() -> Result<Self> {
        let api_base_url = std::env::var("API_BASE_URL").ok();
        let ws_base_url = std::env::var("WS_BASE_URL").ok();
        Self::new_with_urls(api_base_url, ws_base_url).await
    }

    pub async fn new_with_urls(
        api_base_url: Option<String>,
        ws_base_url: Option<String>,
    ) -> Result<Self> {
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
            AppState::NeedsAuth
        };

        let (ws_client, ws_rx) = WsClient::connect(ws_base_url).await?;

        let cache = Arc::new(Mutex::new(CacheStore::new()?));
        let (app_tx, app_rx) = mpsc::channel::<AppEvent>(32);

        let mut app = Self {
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
        };

        if matches!(app.state, AppState::LoggedIn) {
            app.setup_session().await?;
        }

        Ok(app)
    }

    pub async fn setup_session(&mut self) -> Result<()> {
        let token = self.api_client.clone_token();
        self.authenticate_ws(&token).await?;

        if let Ok(user) = crate::api::user::fetch_current_user(&self.api_client).await
            && let Ok(uid) = Id::<crate::models::User>::new(&user.id)
        {
            let mut cache_locked = self.cache.lock().await;
            cache_locked.set(uid, &user).ok();
            self.store.users.insert(user.id.clone(), user);
        }

        let users = self.store.users.clone();
        let api_client = self.api_client.clone();
        let app_tx = self.app_tx.clone();
        tokio::spawn(async move {
            match crate::api::dm::fetch_dms(&api_client, &users).await {
                Ok((dms, new_users)) => {
                    app_tx.send(AppEvent::DmsLoaded(dms, new_users)).await.ok();
                }
                Err(e) => {
                    error!("Error pre-fetching DMs on startup: {e}");
                }
            }
        });

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
                    return Err(anyhow::anyhow!("WebSocket authentication failed: {error}"));
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

                    #[allow(clippy::cast_possible_truncation)]
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

    pub async fn run(&mut self, terminal: &mut ratatui::DefaultTerminal) -> Result<()> {
        let data_dump_timeout = 10;
        let mut delta = Instant::now();

        while !self.should_quit {
            terminal.draw(|f| crate::views::render(f, self))?;

            // Limit poll rate to ~60 FPS
            if ratatui::crossterm::event::poll(Duration::from_millis(16))?
                && let Event::Key(key) = ratatui::crossterm::event::read()?
            {
                self.handle_key_event(key).await?;

                if self.should_quit {
                    break;
                }
            }

            while let Ok(event) = self.app_rx.try_recv() {
                self.handle_app_event(event);
            }

            while let Ok(event) = self.ws_rx.try_recv() {
                self.handle_ws_event(event);
            }

            if delta.elapsed().as_secs() >= 60 * data_dump_timeout {
                if let Err(e) = self.cache.lock().await.dump() {
                    error!("Error dumping cache to disk: {e}");
                }
                delta = Instant::now();
            }
        }

        self.shutdown().await;
        Ok(())
    }

    pub async fn handle_key_event(&mut self, key: KeyEvent) -> Result<()> {
        if matches!(self.input_state.input_mode, InputMode::Command) {
            crate::views::command::handle(self, key);
            return Ok(());
        }

        match self.state {
            AppState::NeedsAuth => crate::views::auth::handle(self, key).await,
            AppState::ValidationToken => {}
            AppState::LoggedIn => crate::views::server_list::handle(self, key),
            AppState::DmList => crate::views::dm_list::handle(self, key),
            AppState::Dm => crate::views::dm::handle(self, key),
            AppState::Error(_) => crate::views::error::handle(self, key),
        }
        Ok(())
    }

    pub fn handle_app_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::DmsLoaded(dms, new_users) => {
                for user in new_users {
                    self.store.users.insert(user.id.clone(), user);
                }
                self.store.dm_channels = dms;
                self.is_loading_dms = false;

                // Spawn background revalidation to fetch fresh user status from REST API
                let recipient_ids: Vec<String> = self
                    .store
                    .dm_channels
                    .iter()
                    .filter_map(|ch| ch.recipient_id.clone())
                    .collect();
                let api_client = self.api_client.clone();
                let app_tx = self.app_tx.clone();
                tokio::spawn(async move {
                    log::info!(
                        "Starting user status revalidation for {} recipients",
                        recipient_ids.len()
                    );
                    let mut fresh_users = Vec::new();
                    for id in &recipient_ids {
                        match crate::api::user::fetch_user(&api_client, id).await {
                            Ok(user) => {
                                log::info!(
                                    "Revalidated user {}: status={:?}",
                                    user.username,
                                    user.status
                                );
                                fresh_users.push(user);
                            }
                            Err(e) => {
                                log::warn!("Failed to revalidate user {}: {}", id, e);
                            }
                        }
                    }
                    log::info!(
                        "Revalidation complete: {} users refreshed",
                        fresh_users.len()
                    );
                    if !fresh_users.is_empty() {
                        app_tx
                            .send(AppEvent::UsersRefreshed(fresh_users))
                            .await
                            .ok();
                    }
                });
            }
            AppEvent::UsersRefreshed(users) => {
                for user in users {
                    self.store.users.insert(user.id.clone(), user);
                }
            }
            AppEvent::DmMessagesLoaded(channel_id, messages, new_users) => {
                for user in new_users {
                    self.store.users.insert(user.id.clone(), user);
                }
                self.store.current_dm_messages = messages;
                self.is_loading_messages = false;

                // Write-through: persist fetched messages to cache
                let cache = self.cache.clone();
                let msgs_to_cache = self.store.current_dm_messages.clone();
                tokio::spawn(async move {
                    if let Ok(mut cache_locked) = cache.try_lock() {
                        let _ = cache_locked.set_messages(&channel_id, &msgs_to_cache);
                    }
                });
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
                    let api_client = self.api_client.clone();
                    let ch_id = channel_id.clone();
                    let msg_id = message.id.clone();
                    tokio::spawn(async move {
                        let _ =
                            crate::api::channel::ack_message(&api_client, &ch_id, &msg_id).await;
                    });

                    // Update cache with the new message included
                    let cache = self.cache.clone();
                    let ch_id = channel_id.clone();
                    let msgs_to_cache = self.store.current_dm_messages.clone();
                    tokio::spawn(async move {
                        if let Ok(mut cache_locked) = cache.try_lock() {
                            let _ = cache_locked.set_messages(&ch_id, &msgs_to_cache);
                        }
                    });
                }

                if let Some(channel) = self
                    .store
                    .dm_channels
                    .iter_mut()
                    .find(|c| c.id == channel_id)
                {
                    channel.last_message_id = Some(message.id.clone());
                    if !is_active_channel {
                        channel.has_unread = true;
                    }
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

                if is_active_channel
                    && let Some(msg) = self
                        .store
                        .current_dm_messages
                        .iter_mut()
                        .find(|m| m.id == message_id)
                {
                    msg.content = content;
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
            AppEvent::TypingStart {
                channel_id,
                user_id,
            } => {
                if let Some(channel) = self
                    .store
                    .dm_channels
                    .iter_mut()
                    .find(|c| c.id == channel_id)
                {
                    channel.typing_users.insert(user_id);
                }
            }
            AppEvent::TypingStop {
                channel_id,
                user_id,
            } => {
                if let Some(channel) = self
                    .store
                    .dm_channels
                    .iter_mut()
                    .find(|c| c.id == channel_id)
                {
                    channel.typing_users.remove(&user_id);
                }
            }
            AppEvent::ChannelAcked {
                channel_id,
                message_id,
            } => {
                if let Some(channel) = self
                    .store
                    .dm_channels
                    .iter_mut()
                    .find(|c| c.id == channel_id)
                {
                    channel.last_message_id = Some(message_id);
                    channel.has_unread = false;
                }
            }
        }
    }

    pub fn handle_ws_event(&mut self, event: ServerEvent) {
        crate::api::ws::handle(self, event);
    }

    pub async fn shutdown(&self) {
        let mut cache_locked = self.cache.lock().await;
        for user in self.store.users.values() {
            if let Ok(uid) = Id::<crate::models::User>::new(&user.id) {
                let _ = cache_locked.set(uid, user);
            }
        }

        // Persist current DM messages if we're viewing a conversation
        if matches!(self.state, AppState::Dm)
            && let Some(channel) = self.store.dm_channels.get(self.selected_dm_index)
            && !self.store.current_dm_messages.is_empty()
        {
            let _ = cache_locked.set_messages(&channel.id, &self.store.current_dm_messages);
        }

        if let Err(e) = cache_locked.dump() {
            error!("Failed to dump cache to disk: {e}");
        }
    }

    pub fn go_back_or_quit(&mut self) {
        match self.state {
            AppState::DmList => self.state = AppState::LoggedIn,
            AppState::Dm => {
                self.state = AppState::DmList;
                self.set_input_mode(InputMode::UI);
            }
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Message, User};
    use serde_json::Value;
    use std::collections::HashMap;
    use tempfile::TempDir;
    use tokio::sync::OnceCell;

    /// Real account data shared by every test, fetched once per run.
    struct Live {
        token: String,
        me: User,
        dms: Vec<DirectMessageChannel>,
        /// Stand-in users for every DM recipient, so `fetch_dms` makes no
        /// per-user requests and the suite stays under the API rate limit.
        placeholders: HashMap<String, User>,
        /// A fully read DM, so acking its latest message changes nothing.
        read_dm: DirectMessageChannel,
        /// The real recipient of `read_dm`.
        recipient: User,
        /// The latest messages of `read_dm`, newest first.
        messages: Vec<Message>,
    }

    static LIVE: OnceCell<Live> = OnceCell::const_new();

    async fn live() -> &'static Live {
        LIVE.get_or_init(|| async {
            let auth = Auth::new().expect("keyring must be available to run app tests");
            let token = auth
                .token_entry
                .get_secret()
                .await
                .expect("a vimstoat session token must be stored in the keyring");
            let client = ApiClient::new(token.clone(), None);

            let me = crate::api::user::fetch_current_user(&client)
                .await
                .expect("GET /users/@me should succeed with the real token");
            let raw_dms = client
                .get::<Vec<Value>>(crate::api::client::Endpoint::Dms)
                .await
                .expect("GET /users/dms should succeed with the real token");
            let placeholders: HashMap<String, User> = raw_dms
                .iter()
                .filter_map(|dm| dm["recipients"].as_array())
                .flatten()
                .filter_map(Value::as_str)
                .map(|id| {
                    let user = User {
                        id: id.to_string(),
                        username: format!("placeholder-{id}"),
                        status: crate::models::UserStatus::default(),
                        status_text: None,
                    };
                    (id.to_string(), user)
                })
                .collect();
            let (dms, _) = crate::api::dm::fetch_dms(&client, &placeholders)
                .await
                .expect("fetch_dms should succeed with the real token");
            let unreads = crate::api::dm::fetch_unreads(&client).await.unwrap();

            let query = crate::api::channel::MessageHistoryQuery {
                limit: Some(3),
                before: None,
                after: None,
                sort: None,
                nearby: None,
            };
            for dm in &dms {
                let fully_read = unreads.iter().any(|u| {
                    u["_id"]["channel"].as_str() == Some(&dm.id)
                        && u["last_id"].as_str() == dm.last_message_id.as_deref()
                        && u["mentions"].as_array().is_none_or(Vec::is_empty)
                });
                let Some(recipient_id) = &dm.recipient_id else {
                    continue;
                };
                if dm.last_message_id.is_none() || !fully_read {
                    continue;
                }
                let history =
                    crate::api::channel::fetch_message_history(&client, &dm.id, Some(&query))
                        .await
                        .unwrap();
                if history.len() == 3 {
                    let messages = history.iter().map(to_message).collect();
                    let recipient = crate::api::user::fetch_user(&client, recipient_id)
                        .await
                        .expect("DM recipient should be fetchable");
                    return Live {
                        token,
                        me,
                        read_dm: dm.clone(),
                        recipient,
                        dms,
                        placeholders,
                        messages,
                    };
                }
            }
            panic!("app tests need a fully read DM with at least 3 messages");
        })
        .await
    }

    fn to_message(raw: &Value) -> Message {
        let author_id = raw["author"].as_str().unwrap().to_string();
        Message {
            id: raw["_id"].as_str().unwrap().to_string(),
            author_name: author_id.clone(),
            author_id,
            content: raw["content"].as_str().unwrap_or_default().to_string(),
        }
    }

    /// A logged-in `App` on the real API and websocket, with a throwaway cache
    /// so the user's real cache file is never read or written.
    async fn test_app() -> (App, TempDir) {
        let live = live().await;
        let cache_dir = TempDir::new().expect("Failed to create a temporary directory");
        let (ws_client, ws_rx) = WsClient::connect(None)
            .await
            .expect("websocket should connect to the real Stoat server");
        let (app_tx, app_rx) = mpsc::channel::<AppEvent>(32);

        let app = App {
            state: AppState::LoggedIn,
            input_text: String::new(),
            input_cursor: 0,
            yank_buffer: None,
            command_text: String::new(),
            auth: Auth::new().unwrap(),
            should_quit: false,
            input_state: InputState::default(),
            api_base_url: API_BASE_URL.to_string(),
            api_client: ApiClient::new(live.token.clone(), None),
            ws_client,
            ws_rx,
            cache: Arc::new(Mutex::new(
                CacheStore::new_temporary(cache_dir.path()).unwrap(),
            )),
            store: AppStore::default(),
            selected_index: 0,
            selected_dm_index: 0,
            is_loading_dms: false,
            is_loading_messages: false,
            app_tx,
            app_rx,
        };
        (app, cache_dir)
    }

    /// An `App` viewing `live.read_dm` with its history loaded.
    async fn app_in_read_dm() -> (App, TempDir) {
        let live = live().await;
        let (mut app, dir) = test_app().await;
        app.store.dm_channels = live.dms.clone();
        app.selected_dm_index = live
            .dms
            .iter()
            .position(|dm| dm.id == live.read_dm.id)
            .unwrap();
        app.store.current_dm_messages = live.messages.clone();
        app.state = AppState::Dm;
        (app, dir)
    }

    fn other_dm(live: &Live) -> &DirectMessageChannel {
        live.dms
            .iter()
            .find(|dm| dm.id != live.read_dm.id)
            .expect("app tests need at least two DMs")
    }

    fn channel<'a>(app: &'a App, id: &str) -> &'a DirectMessageChannel {
        app.store.dm_channels.iter().find(|c| c.id == id).unwrap()
    }

    fn message_ids(messages: &[Message]) -> Vec<&str> {
        messages.iter().map(|m| m.id.as_str()).collect()
    }

    async fn next_app_event(app: &mut App) -> AppEvent {
        time::timeout(Duration::from_secs(30), app.app_rx.recv())
            .await
            .expect("timed out waiting for an AppEvent")
            .expect("app event channel closed")
    }

    /// Waits for a spawned write-through task to cache a channel's messages.
    /// Single-threaded runtime: the task only runs while we sleep, so our
    /// lock never makes its `try_lock` fail.
    async fn cached_messages(app: &App, channel_id: &str) -> Vec<Message> {
        for _ in 0..100 {
            time::sleep(Duration::from_millis(10)).await;
            if let Some(messages) = app.cache.lock().await.get_messages(channel_id) {
                return messages;
            }
        }
        panic!("messages for {channel_id} were never written to the cache");
    }

    #[tokio::test]
    async fn test_authenticate_ws_accepts_real_token() {
        let live = live().await;
        let (mut app, _dir) = test_app().await;

        app.authenticate_ws(&live.token)
            .await
            .expect("real token should authenticate the websocket");
    }

    #[tokio::test]
    async fn test_authenticate_ws_rejects_invalid_token() {
        let (mut app, _dir) = test_app().await;

        let result = app
            .authenticate_ws("definitely_not_a_real_session_token")
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_setup_session_caches_me_and_loads_dms() {
        let live = live().await;
        let (mut app, _dir) = test_app().await;
        // Seed users as if loaded from cache, so fetch_dms needn't refetch them.
        app.store.users = live.placeholders.clone();

        app.setup_session().await.unwrap();

        // Compare identity only: presence flips while other tests' websockets
        // authenticate as the same account.
        let stored_me = &app.store.users[&live.me.id];
        assert_eq!(stored_me.username, live.me.username);
        let cached_me = app
            .cache
            .lock()
            .await
            .get(Id::<User>::new(&live.me.id).unwrap())
            .expect("current user should be written to the cache");
        assert_eq!(&cached_me, stored_me);

        let AppEvent::DmsLoaded(dms, _) = next_app_event(&mut app).await else {
            panic!("setup_session should send DmsLoaded first");
        };
        assert_eq!(dms, live.dms);
    }

    #[tokio::test]
    async fn test_dms_loaded_stores_channels_and_refreshes_stale_users() {
        let live = live().await;
        let (mut app, _dir) = test_app().await;
        app.is_loading_dms = true;
        // One DM only: revalidation fetches every recipient, one request each.
        let dms = vec![live.read_dm.clone()];
        let stale = User {
            username: format!("stale-{}", live.recipient.id),
            ..live.recipient.clone()
        };

        app.handle_app_event(AppEvent::DmsLoaded(dms.clone(), vec![stale.clone()]));

        assert_eq!(app.store.dm_channels, dms);
        assert!(!app.is_loading_dms);
        assert_eq!(app.store.users[&stale.id], stale);

        let AppEvent::UsersRefreshed(fresh) = next_app_event(&mut app).await else {
            panic!("DmsLoaded should trigger a UsersRefreshed");
        };
        app.handle_app_event(AppEvent::UsersRefreshed(fresh));

        assert_eq!(
            app.store.users[&stale.id].username, live.recipient.username,
            "stale user was not refreshed from the API"
        );
    }

    #[tokio::test]
    async fn test_dm_messages_loaded_writes_through_to_cache() {
        let live = live().await;
        let (mut app, _dir) = test_app().await;
        app.is_loading_messages = true;

        app.handle_app_event(AppEvent::DmMessagesLoaded(
            live.read_dm.id.clone(),
            live.messages.clone(),
            vec![live.me.clone()],
        ));

        assert_eq!(app.store.current_dm_messages, live.messages);
        assert!(!app.is_loading_messages);
        assert!(app.store.users.contains_key(&live.me.id));
        assert_eq!(cached_messages(&app, &live.read_dm.id).await, live.messages);
    }

    #[tokio::test]
    async fn test_new_message_in_active_dm_is_shown_read_and_cached() {
        let live = live().await;
        let (mut app, _dir) = app_in_read_dm().await;
        let (newest, older) = live.messages.split_first().unwrap();
        app.store.current_dm_messages = older.to_vec();

        // Acks the already-read latest message, leaving real read state unchanged.
        app.handle_app_event(AppEvent::NewMessage {
            channel_id: live.read_dm.id.clone(),
            message: newest.clone(),
            new_user: None,
        });

        assert_eq!(app.store.current_dm_messages, live.messages);
        let dm = channel(&app, &live.read_dm.id);
        assert_eq!(dm.last_message_id.as_ref(), Some(&newest.id));
        assert!(!dm.has_unread, "message in the open DM marked it unread");
        assert_eq!(cached_messages(&app, &live.read_dm.id).await, live.messages);
    }

    #[tokio::test]
    async fn test_new_message_in_background_dm_marks_unread() {
        let live = live().await;
        let (mut app, _dir) = test_app().await;
        app.store.dm_channels = live.dms.clone();
        app.state = AppState::DmList;
        let newest = live.messages[0].clone();

        app.handle_app_event(AppEvent::NewMessage {
            channel_id: live.read_dm.id.clone(),
            message: newest.clone(),
            new_user: Some(live.me.clone()),
        });

        let dm = channel(&app, &live.read_dm.id);
        assert!(dm.has_unread);
        assert_eq!(dm.last_message_id.as_ref(), Some(&newest.id));
        assert!(app.store.current_dm_messages.is_empty());
        assert!(app.store.users.contains_key(&live.me.id));
    }

    #[tokio::test]
    async fn test_message_updated_only_edits_active_dm() {
        let live = live().await;
        let (mut app, _dir) = app_in_read_dm().await;
        let target = live.messages[1].id.clone();

        app.handle_app_event(AppEvent::MessageUpdated {
            channel_id: other_dm(live).id.clone(),
            message_id: target.clone(),
            content: "from another channel".to_string(),
        });
        assert_eq!(app.store.current_dm_messages, live.messages);

        app.handle_app_event(AppEvent::MessageUpdated {
            channel_id: live.read_dm.id.clone(),
            message_id: target.clone(),
            content: "edited".to_string(),
        });
        for (msg, original) in app.store.current_dm_messages.iter().zip(&live.messages) {
            let expected = if msg.id == target {
                "edited"
            } else {
                &original.content
            };
            assert_eq!(msg.content, expected);
        }
    }

    #[tokio::test]
    async fn test_message_deleted_only_removes_from_active_dm() {
        let live = live().await;
        let (mut app, _dir) = app_in_read_dm().await;
        let target = live.messages[1].id.clone();

        app.handle_app_event(AppEvent::MessageDeleted {
            channel_id: other_dm(live).id.clone(),
            message_id: target.clone(),
        });
        assert_eq!(app.store.current_dm_messages, live.messages);

        app.handle_app_event(AppEvent::MessageDeleted {
            channel_id: live.read_dm.id.clone(),
            message_id: target,
        });
        assert_eq!(
            message_ids(&app.store.current_dm_messages),
            [live.messages[0].id.as_str(), live.messages[2].id.as_str()]
        );
    }

    #[tokio::test]
    async fn test_typing_is_tracked_per_channel() {
        let live = live().await;
        let (mut app, _dir) = test_app().await;
        app.store.dm_channels = live.dms.clone();
        let typist = live.me.id.clone();

        app.handle_app_event(AppEvent::TypingStart {
            channel_id: live.read_dm.id.clone(),
            user_id: typist.clone(),
        });
        assert!(
            channel(&app, &live.read_dm.id)
                .typing_users
                .contains(&typist)
        );
        assert!(channel(&app, &other_dm(live).id).typing_users.is_empty());

        app.handle_app_event(AppEvent::TypingStop {
            channel_id: live.read_dm.id.clone(),
            user_id: typist,
        });
        assert!(channel(&app, &live.read_dm.id).typing_users.is_empty());
    }

    #[tokio::test]
    async fn test_channel_acked_clears_unread() {
        let live = live().await;
        let (mut app, _dir) = test_app().await;
        app.store.dm_channels = live.dms.clone();
        for dm in &mut app.store.dm_channels {
            dm.has_unread = true;
        }

        app.handle_app_event(AppEvent::ChannelAcked {
            channel_id: live.read_dm.id.clone(),
            message_id: live.messages[0].id.clone(),
        });

        let acked = channel(&app, &live.read_dm.id);
        assert!(!acked.has_unread);
        assert_eq!(acked.last_message_id.as_ref(), Some(&live.messages[0].id));
        assert!(channel(&app, &other_dm(live).id).has_unread);
    }

    #[tokio::test]
    async fn test_shutdown_persists_users_and_open_dm_to_disk() {
        let live = live().await;
        let (mut app, dir) = app_in_read_dm().await;
        app.store.users.insert(live.me.id.clone(), live.me.clone());

        app.shutdown().await;

        let on_disk = pickledb::PickleDb::load(
            dir.path().join(crate::cache::DB_FILE),
            pickledb::PickleDbDumpPolicy::NeverDump,
            pickledb::SerializationMethod::Bin,
        )
        .expect("shutdown should dump the cache to disk");
        assert_eq!(
            on_disk.get::<User>(&format!("user:{}", live.me.id)),
            Some(live.me.clone())
        );
        assert_eq!(
            on_disk.get::<Vec<Message>>(&format!("messages:{}", live.read_dm.id)),
            Some(live.messages.clone())
        );
    }

    #[tokio::test]
    async fn test_go_back_or_quit_walks_up_then_quits() {
        let (mut app, _dir) = test_app().await;
        app.state = AppState::Dm;
        app.set_input_mode(InputMode::Insert);

        app.go_back_or_quit();
        assert!(matches!(app.state, AppState::DmList));
        assert_eq!(app.input_state.input_mode, InputMode::UI);
        assert!(!app.should_quit);

        app.go_back_or_quit();
        assert!(matches!(app.state, AppState::LoggedIn));
        assert!(!app.should_quit);

        app.go_back_or_quit();
        assert!(app.should_quit);
    }
}
