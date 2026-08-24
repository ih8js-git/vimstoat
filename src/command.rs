use crate::app::App;

/// Represents commands entered in Vim command mode (e.g. `:q`, `:quit`)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Quit,
    QuitAll,
    Unknown(String),
}

impl Command {
    /// Parses a raw command string (without leading colon) into a `Command`.
    pub fn parse(input: &str) -> Option<Self> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return None;
        }

        let cmd = match trimmed {
            "q" | "quit" | "q!" => Command::Quit,
            "qa" | "qa!" | "qall" | "qall!" => Command::QuitAll,
            other => Command::Unknown(other.to_string()),
        };

        Some(cmd)
    }

    pub fn execute(&self, app: &mut App) {
        match self {
            Command::Quit => {
                app.go_back_or_quit();
            }
            Command::QuitAll => {
                app.should_quit = true;
            }
            Command::Unknown(cmd_name) => {
                log::warn!("Unknown command: :{cmd_name}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_quit_commands() {
        assert_eq!(Command::parse("q"), Some(Command::Quit));
        assert_eq!(Command::parse("quit"), Some(Command::Quit));
        assert_eq!(Command::parse("q!"), Some(Command::Quit));
        assert_eq!(Command::parse("  q  "), Some(Command::Quit));
        assert_eq!(Command::parse("qa"), Some(Command::QuitAll));
        assert_eq!(Command::parse("qa!"), Some(Command::QuitAll));
        assert_eq!(Command::parse("qall"), Some(Command::QuitAll));
    }

    #[test]
    fn test_parse_unknown_command() {
        assert_eq!(
            Command::parse("foo"),
            Some(Command::Unknown("foo".to_string()))
        );
    }

    #[test]
    fn test_parse_empty() {
        assert_eq!(Command::parse(""), None);
        assert_eq!(Command::parse("   "), None);
    }
}
