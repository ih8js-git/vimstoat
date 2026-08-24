# Features

VimStoat is currently in early development. Here are the features implemented so far:

## Real-time Messaging
- **WebSocket Integration**: Listens for real-time `Message`, `MessageUpdate`, and `MessageDelete` events from the server.
- **Dynamic Author Resolution**: Transparently fetches and caches unknown user profiles to display real usernames instead of raw IDs in the UI.
- **In-Memory Cache**: Uses an in-memory cache synchronized to disk upon application exit for maximum performance.

## Direct Messages
- **DM List View**: Displays all active Direct Message channels.
- **Unread Indicators**: Channels with new messages are automatically marked with a red `[*]` unread indicator.
- **Message Previews**: The DM list shows a truncated preview of the latest message for each channel.
- **Chat Interface**: View message history and send new messages seamlessly via the REST API.

## Vim-Native Interface
- **Mode-Specific UI Themes**: Border colors dynamically shift based on the current mode (e.g., Yellow for Insert mode, Blue for UI/Normal mode).
- **Cursor Shaping**: Hardware cursor shape changes depending on mode (e.g., blinking bar in Insert mode, block in Normal mode).
- **Input Motions**: Features robust vim-like text composition motions like `i`, `I`, `a`, `A`, `o`, `O`, `dd` (delete line), and an in-memory yank buffer.
- **Screen Navigation**: Use `q` or `:q` to navigate back through screens, and `:qa` to quit globally.
