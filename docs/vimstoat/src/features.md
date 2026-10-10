# Features

VimStoat is currently in early development. Here are the features implemented so far. Anything described as **Planned** is not implemented yet.

## Authentication
- **Session token**: VimStoat logs in with a Stoat session token rather than a username and password.
- **OS keyring storage**: The token is stored in your operating system's keyring (under the name `vimstoat`). It is never written to the cache or the log file.
- **First run**: If no token is stored, VimStoat asks you to paste one and press `Enter`. The input is masked (`•`). The token is checked against the Stoat API *before* it is saved, so a bad token is never stored. `Esc` quits.
- **Later runs**: The stored token is read from the keyring and validated against the API on startup, so you go straight to the server list.
- **Errors**: If the keyring is unavailable, the server can't be reached, or the token is rejected, an error screen is shown (titled *Keyring*, *Connection*, or *Authentication* error). Press a character key, `Enter`, or `Esc` to return to the token prompt. A rejected token stays in the keyring until you enter a valid one, which replaces it.

## Server List
- **Landing screen**: After logging in you see the server list. The first row is always **Direct Messages**; the servers you are a member of follow it.
- **Relative line numbers**: Like Vim's `relativenumber`, the selected row shows its own index and every other row shows its distance from the selection.
- **Navigation**: `j`/`k` (or the arrow keys) move the selection, `gg` jumps to the top, `G` jumps to the bottom, and `Enter` opens the selected item.
- **Servers are not browsable yet**: Servers are listed, but opening one does nothing. Server channels are **Planned**.

## Direct Messages
- **DM List View**: Displays all active Direct Message channels, including Saved Messages and group DMs.
- **Chat Interface**: View message history (the latest 50 messages) and send new messages via the REST API.
- **Message content**: System messages are shown as `[System message: <type>]`, and anything VimStoat can't display as `[Unsupported message]`.

## Presence
- **Status bubble**: Each DM shows a coloured bubble after the name, in both the DM list and the conversation title:

  | Bubble | Status         |
  | ------ | -------------- |
  | `●`    | Online         |
  | `◐`    | Idle           |
  | `◆`    | Focus          |
  | `■`    | Do Not Disturb |
  | `○`    | Offline / Invisible |

- **Status text**: A user's custom status text is shown after the bubble (` - text`), unless they are offline.
- **Always live**: Presence comes from the real-time connection. The `Ready` event sets everyone's status when VimStoat connects, and `UserUpdate` events keep it current from then on, so the bubble reflects the latest state. A user who disconnects is shown as offline.
- **Group DMs**: Group DMs have no single recipient, so they don't show a bubble.

## Typing Indicators
- **Incoming typing**: When someone in the open conversation is typing, the title of the message box reads `… - <name> is typing...`. Several people are listed separated by commas.
- **Not sent yet**: VimStoat does not yet tell other people when *you* are typing (**Planned**).

## Unread Indicators
- **Marker**: Channels with unread messages are marked with a red `[*]` before the name.
- **Synced with the server**: On startup VimStoat fetches your unread state from Stoat. A channel is unread if you were mentioned in it or its latest message is newer than the last one you read. If the unread state can't be fetched, nothing is marked unread.
- **Live**: A message arriving in a conversation you are not viewing sets the marker.
- **Acknowledged on read**: Opening a conversation clears its marker and tells the server you've read it, and messages that arrive while it is open are acknowledged as they come in. Reading a channel on another device clears the marker here too.
- **No mention marker**: Mentions and ordinary unread messages look the same.

## Real-time Events
VimStoat keeps a WebSocket connection to Stoat open (a ping is sent every 20 seconds to keep it alive) and reacts to these events:

| Event                                 | Effect                                                           |
| ------------------------------------- | ---------------------------------------------------------------- |
| `Ready`                               | Loads the initial users (and their presence) and your servers    |
| `Message`                             | Appends the message if the conversation is open; otherwise marks the channel unread |
| `MessageUpdate`                       | Applies the edited text (open conversation only)                 |
| `MessageDelete`                       | Removes the message (open conversation only)                     |
| `UserUpdate`                          | Updates a user's presence and status text                        |
| `ChannelStartTyping` / `ChannelStopTyping` | Shows or hides the typing indicator                         |
| `ChannelAck`                          | Clears the unread marker when another client read the channel    |

Every other event (reactions, channel, server and member changes, …) is received but ignored; it is only recorded in the log file. This means a few things are **not** reflected until you restart: DMs created after startup, username changes, and reactions.

If the connection to Stoat drops, VimStoat does not reconnect yet. The app keeps running but stops receiving updates, so restart it to resume live updates.

## Caching
- **Dynamic Author Resolution**: Transparently fetches and caches unknown user profiles to display real usernames instead of raw IDs in the UI.
- **Persistent cache**: User profiles and DM message history are kept in a cache file in your OS cache directory (on Linux, `~/.cache/vimstoat/cache.db`). The cache is held in memory while the app runs and is written to disk every 10 minutes and when you quit, so a crash can lose up to 10 minutes of cache updates. It is safe to delete the file; it is rebuilt as you use the app.
- **Instant conversations**: Opening a DM shows the cached history immediately while the latest 50 messages are fetched, then replaces it with the fresh copy.

### The cache is only a starting point
What you see in the UI is the live state built from the server and the real-time events above, not the cache file. The cache only fills in the first screen while fresh data loads:

- **Message history** is fetched from the server every time you open a conversation, so the cached copy is on screen only until that request completes.
- **Presence** is replaced by the `Ready` event as soon as VimStoat connects.
- **Failed fetches**: if fetching a conversation's history fails (for example, you are offline), the conversation shows as empty rather than falling back to the cached copy.

## Vim-Native Interface
- **Mode-Specific UI Themes**: Border colors dynamically shift based on the current mode (Blue for UI/Normal, Yellow for Insert, Green for Command).
- **Cursor Shaping**: Hardware cursor shape changes depending on mode (blinking bar in Insert and Command mode, block otherwise).
- **Input Motions**: Features vim-like text composition motions like `i`, `I`, `a`, `A`, `o`, `O`, and `dd` (delete line) and `p` (paste), with an in-memory yank buffer.
- **Multi-line messages**: `Shift+Enter` or `Alt+Enter` inserts a newline; `Enter` sends.
- **Screen Navigation**: `:q` goes back one screen at a time, `q` goes back from a conversation, and `:qa` quits globally. See [Keybinds](./keybinds.md).

## Planned
- Fuzzy finder (`fs`, `fc`, `ff`, `fm`)
- Search (`/`)
- Visual mode (`v`)
- Browsing server channels
- Broadcasting your own typing indicator
