use std::collections::HashMap;

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::action::Action;

#[derive(Clone, Copy)]
pub enum InputMode {
    #[allow(unused)]
    Normal,
    UI,
    #[allow(unused)]
    Insert,
    #[allow(unused)]
    Command,
    #[allow(unused)]
    Visual,
}

struct KeyMaps {
    ui: HashMap<Vec<KeyEvent>, Action>,
    normal: HashMap<Vec<KeyEvent>, Action>,
    visual: HashMap<Vec<KeyEvent>, Action>,
    typing: HashMap<Vec<KeyEvent>, Action>,
}

pub struct InputState {
    pub input_mode: InputMode,
    pending_keys: Vec<KeyEvent>,
    key_maps: KeyMaps,
}

impl Default for InputState {
    fn default() -> Self {
        Self {
            input_mode: InputMode::UI,
            pending_keys: Vec::with_capacity(2),
            key_maps: KeyMaps {
                ui: [(
                    vec![KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE)],
                    Action::Quit,
                )]
                .into_iter()
                .collect(),
                normal: HashMap::new(),
                visual: HashMap::new(),
                typing: HashMap::new(),
            },
        }
    }
}

impl InputState {
    /// Gets respective keymap based on iinput mode
    fn key_map(&self) -> &HashMap<Vec<KeyEvent>, Action> {
        match self.input_mode {
            InputMode::UI => &self.key_maps.ui,
            InputMode::Normal => &self.key_maps.normal,
            InputMode::Visual => &self.key_maps.visual,
            InputMode::Insert => &self.key_maps.typing,
            InputMode::Command => &self.key_maps.typing,
        }
    }

    /// Checks if the pending keys might lead to an action, else indicate input state should be cleared
    fn should_clear(&self) -> bool {
        let key_map = self.key_map();
        !key_map.iter().any(|(sequence, _)| {
            self.pending_keys.len() < sequence.len()
                && self.pending_keys == sequence[0..self.pending_keys.len()]
        })
    }

    pub fn process_key_event(&mut self, key_event: KeyEvent) -> Option<Action> {
        self.pending_keys.push(key_event);
        let key_map = self.key_map();
        let actions = key_map
            .get(&self.pending_keys)
            .cloned()
            .or_else(|| match self.input_mode {
                InputMode::Normal | InputMode::UI | InputMode::Visual => None,
                InputMode::Insert | InputMode::Command => type_action(key_event),
            });
        if actions.is_some() || self.should_clear() {
            self.pending_keys.clear();
        }
        actions
    }
}

/// Turn a key event to their respective action during typing
fn type_action(key_event: KeyEvent) -> Option<Action> {
    match key_event.code {
        KeyCode::Char(c) => Some(Action::AppendCharacter(c)),
        KeyCode::Backspace => Some(Action::RemoveCharacter),
        KeyCode::Delete => Some(Action::RemoveCharacter),
        KeyCode::Enter => Some(Action::Enter),
        KeyCode::Left => Some(Action::CursorLeft),
        KeyCode::Right => Some(Action::CursorRight),
        KeyCode::Up => Some(Action::CursorUp),
        KeyCode::Down => Some(Action::CursorDown),
        KeyCode::Esc => Some(Action::Escape),
        _ => None,
    }
}
