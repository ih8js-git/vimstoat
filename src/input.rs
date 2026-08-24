use std::collections::HashMap;

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::action::Action;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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

impl InputMode {
    pub fn color(&self) -> ratatui::style::Color {
        match self {
            InputMode::Normal | InputMode::UI => ratatui::style::Color::Blue,
            InputMode::Insert => ratatui::style::Color::Yellow,
            InputMode::Visual => ratatui::style::Color::Magenta,
            InputMode::Command => ratatui::style::Color::Green,
        }
    }
}

struct KeyMaps {
    ui: HashMap<Vec<KeyEvent>, Action>,
    normal: HashMap<Vec<KeyEvent>, Action>,
    visual: HashMap<Vec<KeyEvent>, Action>,
    typing: HashMap<Vec<KeyEvent>, Action>,
}

impl Default for KeyMaps {
    fn default() -> Self {
        Self {
            ui: HashMap::from([
                (
                    vec![KeyEvent::new(KeyCode::Char(':'), KeyModifiers::NONE)],
                    Action::EnterCommandMode,
                ),
                (
                    vec![KeyEvent::new(KeyCode::Char(':'), KeyModifiers::SHIFT)],
                    Action::EnterCommandMode,
                ),
                (
                    vec![KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)],
                    Action::Enter,
                ),
                (
                    vec![KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE)],
                    Action::EnterInsertMode,
                ),
                (
                    vec![KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE)],
                    Action::EnterInsertModeAfter,
                ),
                (
                    vec![KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE)],
                    Action::CursorLeft,
                ),
                (
                    vec![KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE)],
                    Action::CursorRight,
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
                    vec![KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)],
                    Action::Escape,
                ),
                (
                    vec![KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE)],
                    Action::CursorDown,
                ),
                (
                    vec![KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)],
                    Action::CursorDown,
                ),
                (
                    vec![KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE)],
                    Action::CursorUp,
                ),
                (
                    vec![KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)],
                    Action::CursorUp,
                ),
                (
                    vec![
                        KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE),
                        KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE),
                    ],
                    Action::GoToTopUI,
                ),
            ]),
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
                    vec![KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT)],
                    Action::AppendCharacter('\n'),
                ),
                (
                    vec![KeyEvent::new(KeyCode::Enter, KeyModifiers::ALT)],
                    Action::AppendCharacter('\n'),
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
        }
    }
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
            key_maps: KeyMaps::default(),
        }
    }
}

impl InputState {
    #[allow(unused)]
    pub fn change_input_mode(&mut self, new_mode: InputMode) {
        self.pending_keys.clear();
        self.input_mode = new_mode;
    }

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
        let action = key_map
            .get(&self.pending_keys)
            .cloned()
            .or(self.handle_typing_event(key_event));
        if action.is_some() || !self.has_potential_pending_key_bindings() {
            self.pending_keys.clear();
        }
        action
    }

    pub fn handle_typing_event(&self, key_event: KeyEvent) -> Option<Action> {
        match (self.input_mode, key_event.code) {
            (InputMode::Insert | InputMode::Command, KeyCode::Char(c)) => {
                match key_event.modifiers {
                    KeyModifiers::SHIFT => Some(Action::AppendCharacter(c.to_ascii_uppercase())),
                    KeyModifiers::NONE => Some(Action::AppendCharacter(c)),
                    _ => None,
                }
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod test {
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use crate::{
        action::Action,
        input::{InputMode, InputState},
    };

    #[test]
    fn single_key_motions() {
        let mut state = InputState::default();
        for _ in 0..2 {
            assert_eq!(
                state.process_key_event(KeyEvent::new(KeyCode::Null, KeyModifiers::NONE)),
                None,
                "Should have no action"
            );
            assert_eq!(
                state.process_key_event(KeyEvent::new(KeyCode::Char(':'), KeyModifiers::NONE)),
                Some(Action::EnterCommandMode),
                "Should have done the enter command mode action"
            );
        }
    }

    #[test]
    fn typing_motions() {
        let mut state = InputState::default();
        state.change_input_mode(InputMode::Insert);
        assert_eq!(
            state.process_key_event(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE)),
            Some(Action::AppendCharacter('q')),
            "Should have done appened q"
        );
        assert_eq!(
            state.process_key_event(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE)),
            Some(Action::AppendCharacter('r')),
            "Should have appened r"
        );
        assert_eq!(
            state.process_key_event(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::SHIFT)),
            Some(Action::AppendCharacter('R')),
            "Should have appened R"
        );
        assert_eq!(
            state.process_key_event(KeyEvent::new(KeyCode::Null, KeyModifiers::NONE)),
            None,
            "Should have no action"
        );
        assert_eq!(
            state.process_key_event(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)),
            Some(Action::CursorLeft),
            "Should have moved the cursor left"
        );
    }

    #[test]
    fn multi_keybinding_shortcuts() {
        let mut state = InputState::default();
        assert!(
            state.process_key_event(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE))
                != Some(Action::GoToTopUI),
            "Should not called action early"
        );
        assert_eq!(
            state.process_key_event(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE)),
            Some(Action::GoToTopUI),
            "Should have go to top UI after second g"
        );
        assert!(
            state.process_key_event(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE))
                != Some(Action::GoToTopUI),
            "Should have cleared pending keys"
        );
    }
}
