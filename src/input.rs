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
                ui: HashMap::from([(
                    vec![KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE)],
                    Action::Quit,
                )]),
                normal: HashMap::new(),
                visual: HashMap::new(),
                typing: HashMap::from([
                    (
                        vec![KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE)],
                        Action::RemoveCharacter,
                    ),
                    (
                        vec![KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)],
                        Action::Enter,
                    ),
                    (
                        vec![KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)],
                        Action::CursorLeft,
                    ),
                    (
                        vec![KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)],
                        Action::CursorRight,
                    ),
                    (
                        vec![KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)],
                        Action::CursorUp,
                    ),
                    (
                        vec![KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)],
                        Action::CursorDown,
                    ),
                    (
                        vec![KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)],
                        Action::Escape,
                    ),
                ]),
            },
        }
    }
}

impl InputState {
    /// Gets respective keymap based on input mode
    fn key_map(&self) -> &HashMap<Vec<KeyEvent>, Action> {
        match self.input_mode {
            InputMode::UI => &self.key_maps.ui,
            InputMode::Normal => &self.key_maps.normal,
            InputMode::Visual => &self.key_maps.visual,
            InputMode::Insert => &self.key_maps.typing,
            InputMode::Command => &self.key_maps.typing,
        }
    }

    /// Checks if the pending keys might lead to an action
    fn has_potential_pending_key_bindings(&self) -> bool {
        let key_map = self.key_map();
        key_map.iter().any(|(sequence, _)| {
            self.pending_keys.len() < sequence.len()
                && self.pending_keys == sequence[0..self.pending_keys.len()]
        })
    }

    pub fn process_key_event(&mut self, key_event: KeyEvent) -> Option<Action> {
        self.pending_keys.push(key_event);
        let key_map = self.key_map();

        let action =
            key_map
                .get(&self.pending_keys)
                .cloned()
                .or(match (self.input_mode, key_event.code) {
                    (InputMode::Insert | InputMode::Command, KeyCode::Char(c)) => {
                        Some(Action::AppendCharacter(c))
                    }
                    _ => None,
                });
        if action.is_some() || !self.has_potential_pending_key_bindings() {
            self.pending_keys.clear();
        }
        action
    }
}
