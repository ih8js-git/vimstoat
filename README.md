# vimstoat

A lightweight TUI [Stoat](https://stoat.chat) client that feels like Vim.

> **Status:** Early development — see our [Documentation](docs/vimstoat/src/SUMMARY.md) for features and keybinds.

## What is this?

VimStoat is a terminal chat client for [Stoat.chat](https://stoat.chat) (formerly Revolt) built in Rust with [Ratatui](https://ratatui.rs). It uses Vim's modal editing paradigm — Normal mode for navigation, Insert mode for composing messages, Command mode for `:` commands — so everything is keyboard-driven and fast.

## Philosophy

- **Vim-native.** `j`/`k` to navigate. `i` to compose. `Esc` to stop. `:q` to quit.
- **Single binary.** No runtime dependencies. `cargo install` and go.
- **Token-based auth.** Stoat recommends third-party clients use session tokens rather than handling credentials directly. You paste your token once, it's stored securely in your OS keyring.
- **Minimal and fast.** This is a chat client, not an Electron app.

## Building

```bash
cargo build --release
```

## Configuration

VimStoat connects to `https://api.stoat.chat` by default. For self-hosted instances, configure via `~/.config/vimstoat/config.toml`:

```toml
[instance]
url = "https://api.your-instance.com"
ws_url = "wss://events.your-instance.com"
```

A default config file is generated on first run. Missing keys fall back to their defaults. If the file is invalid, VimStoat shows a warning and uses the defaults without modifying the file.

The `API_BASE_URL` and `WS_BASE_URL` environment variables are no longer supported; set these values in the config file instead.

## License

[GPL-3.0](LICENSE)
