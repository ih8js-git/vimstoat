use ratatui::crossterm::event::{KeyCode, KeyEvent};

use crate::action::Action;
use crate::api::auth::Auth;
use crate::input::InputState;
use crate::{Result, cache::CacheStore};

pub enum AppState {
    InputToken,
    ValidatingToken,
    LoggedIn,
    Error(String),
}

pub struct App {
    pub state: AppState,
    pub input_text: String,
    pub auth: Auth,
    pub username: String,
    pub should_quit: bool,
    pub input_state: InputState,
    #[allow(unused)]
    pub cache: CacheStore,
}

impl App {
    pub async fn new() -> Result<Self> {
        let auth = Auth::new().map_err(|e| anyhow::anyhow!(e))?;

        let mut username = String::new();

        let state = if let Ok(token) = auth.token_entry.get_secret().await {
            match auth.validate_token(&token).await {
                Ok(user_info) => {
                    username = user_info.name().to_string();
                    AppState::LoggedIn
                }
                Err(e) => AppState::Error(format!("Stored token is invalid: {}", e)),
            }
        } else {
            AppState::InputToken
        };

        let cache = CacheStore::new()?;

        Ok(Self {
            state,
            input_text: String::new(),
            auth,
            username,
            should_quit: false,
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
                        match self.auth.validate_token(&self.input_text).await {
                            Ok(user_info) => match self.auth.store_token(&self.input_text).await {
                                Ok(_) => {
                                    self.username = user_info.name().to_string();
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
}
