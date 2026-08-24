use log::error;
use ratatui::crossterm::event::KeyEvent;

use crate::{
    action::Action,
    app::{App, AppEvent, AppState},
    input::InputMode,
};

pub fn handle(app: &mut App, key: KeyEvent) {
    let action = app.input_state.process_key_event(key);
    match action {
        Some(Action::Quit) => app.should_quit = true,
        Some(Action::EnterCommandMode) => {
            app.command_text.clear();
            app.set_input_mode(InputMode::Command);
        }
        Some(Action::Enter) => {
            if app.selected_index == 0 {
                app.selected_dm_index = 0;
                app.state = AppState::DmList;
                app.is_loading_dms = true;

                let users = app.store.users.clone();
                let api_client = app.api_client.clone();
                let app_tx = app.app_tx.clone();

                tokio::spawn(async move {
                    match crate::api::dm::fetch_dms(&api_client, &users).await {
                        Ok((dms, new_users)) => {
                            app_tx.send(AppEvent::DmsLoaded(dms, new_users)).await.ok();
                        }
                        Err(e) => {
                            error!("Error fetching DMs in background: {e}");
                        }
                    }
                });
            }
        }
        Some(Action::CursorUp) => {
            if app.selected_index > 0 {
                app.selected_index -= 1;
            }
        }
        Some(Action::CursorDown) => {
            let total_items = 1 + app.store.servers.len();
            if total_items > 0 && app.selected_index + 1 < total_items {
                app.selected_index += 1;
            }
        }
        Some(Action::GoToTopUI) => {
            app.selected_index = 0;
        }
        _ => {}
    }
}
