# Configuration

VimStoat reads its settings from a TOML file in your OS config directory:

| OS      | Path                                                   |
| ------- | ------------------------------------------------------ |
| Linux   | `~/.config/vimstoat/config.toml` (or `$XDG_CONFIG_HOME/vimstoat/config.toml`) |
| macOS   | `~/Library/Application Support/vimstoat/config.toml`   |
| Windows | `%APPDATA%\vimstoat\config.toml`                       |

Anything described as **Planned** is not implemented yet.

## First Run
If the file doesn't exist, VimStoat creates it with the default settings, creating the `vimstoat` directory if needed. It never overwrites an existing file.

## Default Config

```toml
# vimstoat configuration

[instance]
# REST API base URL
url = "https://api.stoat.chat"
# Websocket events URL
ws_url = "wss://events.stoat.chat"
```

## Options

### `[instance]`
The Stoat server to connect to. Change these to use a self-hosted instance.

| Key      | Default                   | Description                                         |
| -------- | ------------------------- | --------------------------------------------------- |
| `url`    | `https://api.stoat.chat`  | REST API base URL. Must start with `http://` or `https://`. |
| `ws_url` | `wss://events.stoat.chat` | WebSocket events URL. Must start with `ws://` or `wss://`.  |

A trailing `/` on either URL is ignored.

The `API_BASE_URL` and `WS_BASE_URL` environment variables are no longer supported. Set these values in the config file instead.

## Missing and Invalid Settings
- **Missing keys**: Any key you leave out uses its default, so a config containing only `url` is valid.
- **Invalid file**: If the file can't be read, isn't valid TOML, has a value of the wrong type, contains an unknown key (for example a typo like `urll`), or has a URL with the wrong scheme, VimStoat:
  - shows a **Config Warning** pop-up explaining what is wrong and where, which any key dismisses,
  - writes the warning to the log file,
  - uses **all** default settings for that run,
  - leaves your file untouched so you can fix it.

## Planned
These options are not implemented yet:
- **Silent typing**: Keep typing without telling other people you are typing.
- **Notification message content**: Choose whether desktop notifications show the message text.
