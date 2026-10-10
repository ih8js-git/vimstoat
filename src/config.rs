use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

use log::{info, warn};
use serde::Deserialize;

use crate::{
    api::{API_BASE_URL, WS_BASE_URL},
    views::error::ConfigError,
};

pub const CONFIG_FILE: &str = "config.toml";

/// Written to disk when no config file exists. Must parse to `Config::default()`.
pub const DEFAULT_CONFIG: &str = r#"# vimstoat configuration

[instance]
# REST API base URL
url = "https://api.stoat.chat"
# Websocket events URL
ws_url = "wss://events.stoat.chat"
"#;

#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub instance: InstanceConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct InstanceConfig {
    pub url: String,
    pub ws_url: String,
}

impl Default for InstanceConfig {
    fn default() -> Self {
        Self {
            url: API_BASE_URL.to_string(),
            ws_url: WS_BASE_URL.to_string(),
        }
    }
}

impl Config {
    pub fn validate(&self) -> Result<(), ConfigError> {
        check_url("instance.url", &self.instance.url, &["https://", "http://"])?;
        check_url(
            "instance.ws_url",
            &self.instance.ws_url,
            &["wss://", "ws://"],
        )?;
        Ok(())
    }

    /// Strips trailing slashes, since endpoint paths are appended with a leading `/`.
    fn normalize(mut self) -> Self {
        let trim = |s: &mut String| s.truncate(s.trim_end_matches('/').len());
        trim(&mut self.instance.url);
        trim(&mut self.instance.ws_url);
        self
    }
}

fn check_url(key: &str, value: &str, schemes: &[&str]) -> Result<(), ConfigError> {
    let has_host = schemes.iter().any(|scheme| {
        value
            .strip_prefix(scheme)
            .is_some_and(|rest| !rest.is_empty())
    });

    if has_host {
        Ok(())
    } else {
        Err(ConfigError::Invalid(format!(
            "`{key}` must be a URL starting with {}, got {value:?}",
            schemes.join(" or ")
        )))
    }
}

pub fn config_path(config_dir: Option<PathBuf>) -> Option<PathBuf> {
    config_dir.map(|p| p.join(env!("CARGO_PKG_NAME")).join(CONFIG_FILE))
}

fn parse(contents: &str) -> Result<Config, ConfigError> {
    let config = toml::from_str::<Config>(contents)
        .map_err(ConfigError::Parse)?
        .normalize();
    config.validate()?;
    Ok(config)
}

/// Creates the file with `DEFAULT_CONFIG`, refusing to overwrite an existing one.
fn write_default(path: &Path) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?
        .write_all(DEFAULT_CONFIG.as_bytes())
}

fn read_or_create(path: &Path) -> Result<Config, ConfigError> {
    match fs::read_to_string(path) {
        Ok(contents) => parse(&contents),
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            write_default(path).map_err(ConfigError::Write)?;
            info!("Created default config at {}", path.display());
            Ok(Config::default())
        }
        Err(e) => Err(ConfigError::Read(e)),
    }
}

fn fallback(warning: String) -> (Config, Option<String>) {
    warn!("{warning}");
    (Config::default(), Some(warning))
}

/// Loads the config at `path`, generating a default one if it doesn't exist.
/// Any failure falls back to `Config::default()` and returns a warning for the user.
pub fn load_from(path: &Path) -> (Config, Option<String>) {
    match read_or_create(path) {
        Ok(config) => (config, None),
        Err(e) => fallback(format!("{}: {e}", path.display())),
    }
}

pub fn load() -> (Config, Option<String>) {
    match config_path(dirs::config_dir()) {
        Some(path) => load_from(&path),
        None => fallback(ConfigError::NoConfigDir.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    fn with_urls(url: &str, ws_url: &str) -> Config {
        Config {
            instance: InstanceConfig {
                url: url.to_string(),
                ws_url: ws_url.to_string(),
            },
        }
    }

    /// Writes `contents` to a config file in a fresh temp dir.
    fn write_config(contents: &str) -> (TempDir, PathBuf) {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join(CONFIG_FILE);
        fs::write(&path, contents).unwrap();
        (tmp, path)
    }

    /// Asserts the file falls back to defaults with a warning and is left untouched.
    fn assert_rejected(contents: &str) -> String {
        let (_tmp, path) = write_config(contents);

        let (config, warning) = load_from(&path);

        assert_eq!(config, Config::default());
        assert_eq!(fs::read_to_string(&path).unwrap(), contents);
        warning.expect("invalid config should produce a warning")
    }

    // --- Paths ---

    #[test]
    fn config_path_uses_package_name_and_config_file() {
        let base = PathBuf::from("/some/config");

        assert_eq!(
            config_path(Some(base.clone())),
            Some(base.join(env!("CARGO_PKG_NAME")).join(CONFIG_FILE))
        );
    }

    #[test]
    fn config_path_without_config_dir_is_none() {
        assert_eq!(config_path(None), None);
    }

    // --- Defaults ---

    #[test]
    fn default_uses_api_constants() {
        assert_eq!(Config::default(), with_urls(API_BASE_URL, WS_BASE_URL));
    }

    #[test]
    fn default_config_template_matches_default() {
        assert_eq!(parse(DEFAULT_CONFIG).unwrap(), Config::default());
    }

    #[test]
    fn default_is_valid() {
        assert!(Config::default().validate().is_ok());
    }

    // --- Generation ---

    #[test]
    fn missing_file_generates_default_config() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join(CONFIG_FILE);

        let (config, warning) = load_from(&path);

        assert_eq!(config, Config::default());
        assert!(warning.is_none());
        assert_eq!(fs::read_to_string(&path).unwrap(), DEFAULT_CONFIG);
    }

    #[test]
    fn missing_file_creates_parent_directories() {
        let tmp = TempDir::new().unwrap();
        let path = config_path(Some(tmp.path().join("nested").join("config"))).unwrap();
        assert!(!path.parent().unwrap().exists());

        let (_, warning) = load_from(&path);

        assert!(warning.is_none());
        assert!(path.is_file());
    }

    #[test]
    fn generated_config_loads_on_next_run() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join(CONFIG_FILE);

        load_from(&path);
        let (config, warning) = load_from(&path);

        assert_eq!(config, Config::default());
        assert!(warning.is_none());
    }

    #[test]
    fn write_default_does_not_overwrite_existing_file() {
        let (_tmp, path) = write_config("existing");

        let err = write_default(&path).unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(fs::read_to_string(&path).unwrap(), "existing");
    }

    #[test]
    fn write_default_fails_when_parent_is_a_file() {
        let (tmp, blocker) = write_config("");
        let path = blocker.join(CONFIG_FILE);

        assert!(write_default(&path).is_err());
        assert!(tmp.path().join(CONFIG_FILE).is_file());
    }

    // --- Valid configs ---

    #[test]
    fn valid_config_is_loaded() {
        let contents = r#"
[instance]
url = "https://api.example.com"
ws_url = "wss://events.example.com"
"#;
        let (_tmp, path) = write_config(contents);

        let (config, warning) = load_from(&path);

        assert_eq!(
            config,
            with_urls("https://api.example.com", "wss://events.example.com")
        );
        assert!(warning.is_none());
        assert_eq!(fs::read_to_string(&path).unwrap(), contents);
    }

    #[test]
    fn partial_config_fills_missing_keys_with_defaults() {
        let (_tmp, path) = write_config("[instance]\nurl = \"http://localhost:8000\"\n");

        let (config, warning) = load_from(&path);

        assert_eq!(config, with_urls("http://localhost:8000", WS_BASE_URL));
        assert!(warning.is_none());
    }

    #[test]
    fn empty_file_uses_defaults() {
        let (_tmp, path) = write_config("");

        let (config, warning) = load_from(&path);

        assert_eq!(config, Config::default());
        assert!(warning.is_none());
    }

    #[test]
    fn trailing_slashes_are_trimmed() {
        let config = parse(
            "[instance]\nurl = \"https://api.example.com/\"\nws_url = \"wss://events.example.com//\"\n",
        )
        .unwrap();

        assert_eq!(
            config,
            with_urls("https://api.example.com", "wss://events.example.com")
        );
    }

    // --- Invalid configs ---

    #[test]
    fn malformed_toml_is_rejected() {
        assert_rejected("[instance\nurl = ");
    }

    #[test]
    fn wrong_type_is_rejected() {
        assert_rejected("[instance]\nurl = 5\n");
    }

    #[test]
    fn unknown_key_is_rejected() {
        assert_rejected("[instance]\nurll = \"https://api.example.com\"\n");
    }

    #[test]
    fn unknown_table_is_rejected() {
        assert_rejected("[instanse]\nurl = \"https://api.example.com\"\n");
    }

    #[test]
    fn ws_scheme_on_api_url_is_rejected() {
        let warning = assert_rejected("[instance]\nurl = \"wss://api.example.com\"\n");
        assert!(warning.contains("instance.url"));
    }

    #[test]
    fn http_scheme_on_ws_url_is_rejected() {
        let warning = assert_rejected("[instance]\nws_url = \"https://events.example.com\"\n");
        assert!(warning.contains("instance.ws_url"));
    }

    #[test]
    fn empty_url_is_rejected() {
        assert_rejected("[instance]\nurl = \"\"\n");
    }

    #[test]
    fn warning_contains_config_path() {
        let (_tmp, path) = write_config("not toml at all");

        let (_, warning) = load_from(&path);

        assert!(warning.unwrap().contains(&path.display().to_string()));
    }

    // --- I/O failures ---

    #[test]
    fn unreadable_path_falls_back_to_default() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join(CONFIG_FILE);
        fs::create_dir(&path).unwrap();

        let (config, warning) = load_from(&path);

        assert_eq!(config, Config::default());
        assert!(warning.is_some());
    }

    #[test]
    fn uncreatable_path_falls_back_to_default() {
        let (_tmp, blocker) = write_config("");
        let path = blocker.join("nested").join(CONFIG_FILE);

        let (config, warning) = load_from(&path);

        assert_eq!(config, Config::default());
        assert!(warning.is_some());
    }

    // --- Validation ---

    #[test]
    fn validate_accepts_http_and_https_api_urls() {
        assert!(
            with_urls("http://localhost", WS_BASE_URL)
                .validate()
                .is_ok()
        );
        assert!(
            with_urls("https://api.example.com", WS_BASE_URL)
                .validate()
                .is_ok()
        );
    }

    #[test]
    fn validate_accepts_ws_and_wss_urls() {
        assert!(with_urls(API_BASE_URL, "ws://localhost").validate().is_ok());
        assert!(
            with_urls(API_BASE_URL, "wss://events.example.com")
                .validate()
                .is_ok()
        );
    }

    #[test]
    fn validate_rejects_missing_scheme() {
        assert!(
            with_urls("api.example.com", WS_BASE_URL)
                .validate()
                .is_err()
        );
        assert!(
            with_urls(API_BASE_URL, "events.example.com")
                .validate()
                .is_err()
        );
    }

    #[test]
    fn validate_rejects_scheme_without_host() {
        assert!(with_urls("https://", WS_BASE_URL).validate().is_err());
        assert!(with_urls(API_BASE_URL, "wss://").validate().is_err());
    }

    #[test]
    fn validate_rejects_empty_urls() {
        assert!(with_urls("", WS_BASE_URL).validate().is_err());
        assert!(with_urls(API_BASE_URL, "").validate().is_err());
    }

    #[test]
    fn validate_error_is_invalid_variant() {
        assert!(matches!(
            with_urls("ftp://x", WS_BASE_URL).validate(),
            Err(ConfigError::Invalid(_))
        ));
    }
}
