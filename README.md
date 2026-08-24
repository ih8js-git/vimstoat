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
```

## License

[GPL-3.0](LICENSE)
