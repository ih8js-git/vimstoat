/// Action based on user input
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Action {
    #[allow(unused)]
    GoToTopUI,
    #[allow(unused)]
    AppendCharacter(char),
    RemoveCharacter,
    EnterCommandMode,
    Enter,
    CursorLeft,
    CursorRight,
    CursorUp,
    CursorDown,
    Escape,
    #[allow(unused)]
    Quit,
}
