#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Action {
    #[allow(unused)]
    GoToTopUI,
    #[allow(unused)]
    AppendCharacter(char),
    RemoveCharacter,
    EnterCommandMode,
    EnterInsertMode,
    EnterInsertModeAfter,
    Enter,
    CursorLeft,
    CursorRight,
    CursorUp,
    CursorDown,
    Escape,
    #[allow(unused)]
    Quit,
}
