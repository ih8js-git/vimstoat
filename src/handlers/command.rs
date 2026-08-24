use ratatui::crossterm::event::KeyEvent;

use crate::{action::Action, app::App, command::Command, input::InputMode};

pub fn handle(app: &mut App, key: KeyEvent) {
    let action = app.input_state.process_key_event(key);
    match action {
        Some(Action::AppendCharacter(c)) => {
            app.command_text.push(c);
        }
        Some(Action::RemoveCharacter) => {
            app.command_text.pop();
        }
        Some(Action::Escape) => {
            app.command_text.clear();
            app.set_input_mode(InputMode::UI);
        }
        Some(Action::Enter) => {
            if let Some(cmd) = Command::parse(&app.command_text) {
                cmd.execute(app);
            }
            app.command_text.clear();
            app.set_input_mode(InputMode::UI);
        }
        _ => {}
    }
}
