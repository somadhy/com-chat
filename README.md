# COMChat

COMChat is a cross‑platform TUI application for interacting with serial ports (COM / TTY / USB‑Serial) in a modern chat‑like interface.

It is aimed at embedded engineers, hardware developers, and testers who need a convenient way to send commands and inspect device responses.

## Features

- **Cross‑platform serial support** (Windows, Linux, macOS) via `serialport`
- **Chat‑style TUI** built with `ratatui` + `crossterm`
- **Command history** with Up/Down navigation, persisted between runs
- **Echo mode** (show/hide sent commands in the chat, **off by default**)
- **Logging** of commands and responses to separate UTF‑8 log files
- **Batch mode** for running a list of commands from a file

## Installation

Prerequisites:

- Rust toolchain (Rust 1.93 or newer recommended)

Clone and build:

```bash
git clone <your-repo-url> comchat
cd comchat
cargo build --release
```

The compiled binary will be in `target/release/COMchat` (or `COMchat.exe` on Windows).

## Usage

### 1. Interactive TUI mode

#### Start the application

Run without arguments:

```bash
cargo run --release
```

Or run the compiled binary directly:

```bash
./COMchat
```

TUI layout:

- **Tab bar** (top): one tab per connection (or placeholder `No Port`).
- **Chat area** (center): shows device responses and (optionally) your commands.
- **Input line** (bottom‑middle): type a command and press Enter to send.
- **Status bar** (bottom): shows current port, echo state and key bindings.

#### Key bindings (normal mode)

- **Enter**: send current command from the input line.
- **Esc**: exit TUI.
- **Arrow Up / Arrow Down**: navigate command history (previous/next command).
- **Tab / Shift+Tab**: switch between tabs (multi‑port sessions).
- **Ctrl+E**: toggle echo mode (show/hide your commands in the chat).
- **Ctrl+P**: open the port selection and configuration dialog.

#### Selecting and configuring a port (in‑app)

Press **Ctrl+P** to open the **port selector** overlay:

- The dialog lists all detected serial ports with a human‑readable label.
- At the bottom it shows current serial settings for the selected port:

  ```text
  Baud: <...> | Parity: <None/Even/Odd> | Stop: <One/Two> | Flow: <None/Hardware/Software>
  ```

Controls inside the selector:

- **↑ / ↓**: move selection between available ports.
- **+ / →**: increase baud rate (cycles through `9600, 19200, 38400, 57600, 115200, 230400`).
- **- / ←**: decrease baud rate.
- **P**: cycle **parity** (`None → Even → Odd → None`).
- **S**: cycle **stop bits** (`One ↔ Two`).
- **F**: cycle **flow control** (`None → Hardware → Software → None`).
- **Enter**: open the selected port with the chosen settings for the **current tab**.
- **Esc**: cancel and return to normal mode.

When you press **Enter**:

- The active tab:
  - gets bound to the opened port,
  - is renamed to the port name (e.g. `COM3`, `/dev/ttyUSB0`),
  - receives a system message like `Opened port COM3 at 115200 baud`.
- A **port profile** for this port is saved into `config.toml` (see below), so that next time COMChat can pre‑fill the dialog with your last used settings for this port.

You can create multiple tabs (for now by opening ports sequentially and reusing the same tab) and switch between them with **Tab / Shift+Tab**.

#### Sending commands and viewing responses

- Type any text in the input line and press **Enter**:
  - The command is sent to the attached serial port (if any).
  - It is appended to:
    - the in‑memory chat (if echo is enabled),
    - the persistent command history,
    - the `commands.log` file.
- All data read from the serial port:
  - appears in the chat as device responses,
  - is written to `responses.log`.
- Each chat line is timestamped (`[HH:MM:SS]`) and marked with a prefix:
  - `>` for user commands,
  - `<` for device responses,
  - `i` for system info,
  - `!` for errors.

Line endings and echo behavior:

- Commands are terminated with **CRLF (`\r\n`)**, matching typical terminal settings (e.g. PuTTY default).
- Many devices echo back the command text before responding. COMChat:
  - shows your command as `> ...` only when **Echo: On**;
  - **always suppresses** the device echo line if it is exactly equal to the last command, so в чат попадает только реальный ответ.

#### Command history

- COMChat remembers all commands you send.
- Use **Arrow Up / Arrow Down** in normal mode to cycle through previous commands and re‑send them quickly.
- History is persisted between runs in a `history.txt` file under the configuration directory.

### 2. Batch mode

Batch mode executes commands from a text file line‑by‑line, writing both commands and responses to logs and stdout.

Example batch file:

```text
status
version
reset
```

Run batch mode:

```bash
./COMchat --batch commands.txt --port COM3 --baud 115200 --delay-ms 200
```

Options:

- `--batch <file>`: path to batch file (one command per line)
- `--port <name>`: serial port name (e.g. `COM3`, `/dev/ttyUSB0`)
- `--baud <rate>`: baud rate (default `115200`)
- `--delay-ms <n>`: optional delay in milliseconds between commands

Behavior:

- Commands are sent to the specified port one by one.
- For each command:
  - The command is written to `commands.log`.
  - All responses until a read timeout are written to `responses.log` and echoed to stdout.
- If `--delay-ms` is set, COMChat waits the given number of milliseconds between commands.

This mode never starts the TUI — it is suitable for automated tests or scripting.

## Logs and configuration

By default COMChat writes:

- **Command log**: `commands.log`
- **Response log**: `responses.log`

Location:

- If `default_log_dir` is set in the config file, logs are placed there
- Otherwise, logs are placed under the COMChat config directory in a `logs/` subdirectory

Configuration file (TOML) is located under the user config directory (for example `~/.config/comchat/config.toml` on Linux).

You can override the configuration directory for testing or portable setups via:

```bash
export COMCHAT_CONFIG_DIR=/path/to/custom/dir
```

The config file stores:

- Saved port profiles (name, serial parameters, echo, log paths)
- Default log directory

## Development

Run the TUI in debug mode:

```bash
cargo run
```

Run tests:

```bash
cargo test
```

The code is structured into clear layers:

- `core`: domain model (messages, events, serial config), connection management, and batch mode
- `serial`: serial‑port worker abstraction
- `ui`: TUI application state, rendering, and input handling
- `storage`: configuration, history, and logging

See `docs/DEVELOPER_GUIDE.md` for more architectural details.

