# Keybinds

VimStoat has three screens you navigate with the keyboard: the **server list**, the **DM list**, and a **conversation**. The list screens use *UI mode*; a conversation starts in *Normal mode* so you can edit your message like a Vim buffer. Both look the same (blue border).

Keys marked **(Planned)** are not implemented yet.

## Basic Navigation

| Key       | Action                                                          |
| --------- | --------------------------------------------------------------- |
| `h` / `←` | Move left (in the message box)                                  |
| `j` / `↓` | Move down (in lists, or down a line in the message box)         |
| `k` / `↑` | Move up (in lists, or up a line in the message box)             |
| `l` / `→` | Move right (in the message box)                                 |
| `gg`      | Go to the top of the list (server list and DM list only)        |
| `G`       | Go to the bottom of the list (server list and DM list only)     |
| `Enter`   | Open the selected item in a list; send the message in a conversation |

## Mode Switching

| Key   | Action                                                           |
| ----- | ---------------------------------------------------------------- |
| `i`   | Enter Insert mode to the left of the block cursor                |
| `I`   | Enter Insert mode, at the start of the line                      |
| `a`   | Enter Insert mode to the right of the block cursor               |
| `A`   | Enter Insert mode, at the end of the line                        |
| `o`   | Enter Insert mode, and create a new line below the current line. |
| `O`   | Enter Insert mode, and create a new line above the current line. |
| `:`   | Enter Command mode                                               |
| `Esc` | Leave Insert or Command mode                                     |
| `/`   | Search **(Planned)**                                             |
| `v`   | Enter Visual mode **(Planned)**                                  |

## Editing (Normal mode)

| Key  | Action                                                                                       |
| ---- | -------------------------------------------------------------------------------------------- |
| `dd` | Delete the current line of the message. The line is saved to the yank buffer. |
| `yy` | Copy the current line of the message to the yank buffer.                                     |
| `p`  | Paste the yank buffer as a new line below the current line.                                  |

## Insert Mode

| Key                      | Action                                      |
| ------------------------ | ------------------------------------------- |
| any character            | Insert it at the cursor                     |
| `Backspace`              | Delete the character before the cursor      |
| `Enter`                  | Send the message                            |
| `Shift+Enter`, `Alt+Enter` | Insert a newline without sending          |
| `←` `→` `↑` `↓`          | Move the cursor                             |
| `Esc`                    | Return to Normal mode                       |

## Going Back and Quitting

| Key  | Where                  | Action                                        |
| ---- | ---------------------- | --------------------------------------------- |
| `q`  | Conversation (Normal)  | Go back to the DM list                        |
| `q`  | Server list, DM list   | Quit the application                          |

`:q` always goes back exactly one screen (conversation → DM list → server list), and quits from the server list. See the commands below.

## Commands (Command Mode)

Press `:` to enter Command mode, type a command, then press `Enter`. `Esc` cancels and `Backspace` edits the command.

| Command                                                                | Action                                                      |
| ---------------------------------------------------------------------- | ----------------------------------------------------------- |
| `:q`, `:quit`, `:q!`, `:wq`, `:wq!`                                    | Go back to the previous screen, or quit if at the top level |
| `:qa`, `:qall`, `:qa!`, `:qall!`, `:wqa`, `:wqall`, `:wqa!`, `:wqall!` | Quit the application immediately                            |

`:wq` and `:wqa` are aliases for `:q` and `:qa`, so Vim muscle memory works. There is nothing to write, so they behave exactly like their quit counterparts.

Unknown commands are ignored (a warning is written to the log file).

## Fuzzy Finder (Planned)

None of these are implemented yet.

| Key  | Action       |
| ---- | ------------ |
| `fs` | Find server  |
| `fc` | Find channel |
| `ff` | Find friend  |
| `fm` | Find message |
