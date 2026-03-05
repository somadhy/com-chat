# COMChat Developer Guide

This document describes the internal architecture of COMChat and should help you extend or modify the application safely.

## Overview

COMChat is a Rust TUI application that presents serial communication as a chat between the user and a device.

High‑level layers:

- **Core**: domain types, connection management, batch execution
- **Serial layer**: thin wrapper around `serialport` with worker threads
- **UI layer**: TUI state, rendering, and input handling
- **Storage**: configs, command history, and logs

The main entry point is `src/main.rs`.

## Crate layout

- `src/main.rs` — CLI parsing, mode selection (TUI vs batch), TUI event loop bootstrap
- `src/cli.rs` — command‑line arguments definition (`clap`)
- `src/core/mod.rs` — shared domain types:
  - `SerialConfig`, `FlowControl`, `Parity`, `StopBits`
  - `MessageKind`, `ChatMessage`
  - `AppEvent` and `AppEventSender`
- `src/core/connections.rs` — `ConnectionManager` and `PortHandle`
- `src/core/batch.rs` — non‑interactive batch execution
- `src/serial/mod.rs` — serial worker abstraction
- `src/ui/mod.rs` — UI module root
- `src/ui/app.rs` — `App` state and `CommandHistory`
- `src/ui/view.rs` — TUI rendering with `ratatui`
- `src/ui/input.rs` — keyboard handling and shortcuts
- `src/storage/mod.rs` — storage module root
- `src/storage/config.rs` — config (TOML) handling
- `src/storage/history.rs` — persistent command history
- `src/storage/logging.rs` — command/response logging
- `src/error.rs` — `AppError` and `Result` alias

## Core domain model

### Serial configuration

`core::SerialConfig` describes how to open a serial port:

- Port name
- Baud rate
- Data bits / stop bits / parity / flow control
- Timeout
- Echo flag
- Optional command/response log paths

It is serializable (`serde`) so it can be stored inside config files.

### Messages and events

- `MessageKind`:
  - `UserCommand`
  - `DeviceResponse`
  - `SystemInfo`
  - `Error`
- `ChatMessage`:
  - `timestamp` (`SystemTime`)
  - `kind`
  - `text`

`AppEvent` connects the serial workers with the UI:

- `SerialData { port_id, data }`
- `SerialError { port_id, error }`
- `PortClosed { port_id }`

`AppEventSender` is a `std::sync::mpsc::Sender<AppEvent>`.

## Serial layer

File: `src/serial/mod.rs`

- `SerialCommand`:
  - `Write(Vec<u8>)`
  - `Close`
- `SerialWorkerHandle`:
  - `id`, `name`, `command_tx`, and worker thread handle

`spawn_serial_worker`:

1. Builds a `serialport` configuration from `SerialConfig`.
2. Opens the port.
3. Spawns a worker thread that:
   - Reads commands from a channel (`SerialCommand`).
   - Writes data to the serial port.
   - Continuously reads from the port and sends `AppEvent::SerialData` / `SerialError` / `PortClosed` to the UI.

The worker loop is careful not to block the UI: it communicates via channels and uses timeouts for reads.

## Connection management

File: `src/core/connections.rs`

- `PortHandle`:
  - Wraps `SerialWorkerHandle` and a `Sender<SerialCommand>`.
- `ConnectionManager`:
  - Tracks open ports (`HashMap<PortId, PortHandle>`).
  - Methods:
    - `list_available_ports` — wrapper around `serialport::available_ports`.
    - `open_port` — spawn serial worker and register `PortHandle`.
    - `write_to_port` — send `SerialCommand::Write`.
    - `close_port` — send `SerialCommand::Close` and remove the handle.

At present, the UI does not yet expose full multi‑port management, but the infrastructure is prepared.

## UI layer

### App state (`ui::app`)

Key types:

- `CommandHistory`:
  - In‑memory list of commands (`entries`) plus a cursor index.
  - Supports `.push`, `.previous`, `.next`.
  - Has unit tests for navigation behavior.
- `Tab`:
  - `title`, `port_id`, `messages`, `input`.
- `App`:
  - `tabs`, `active_tab`
  - `history`
  - `echo` flag
  - `connections: ConnectionManager`
  - `logger: LogHandles`

Important methods:

- `next_tab` / `previous_tab` — cycle through tabs.
- `toggle_echo` — flip echo mode.
- `handle_serial_event(AppEvent)` — route serial events into the proper tab as `ChatMessage`s.
- `submit_input() -> Option<String>`:
  - Takes the input from the active tab.
  - Pushes it into command history.
  - Logs it via `LogHandles::log_command`.
  - Optionally echoes into the chat (if `echo == true`).
  - If a `port_id` is associated, writes data to the port via `ConnectionManager`.

### Rendering (`ui::view`)

Uses `ratatui` to draw:

- A **tab bar** with port/tab titles.
- A **message area** (each `ChatMessage` is formatted with a timestamp, direction prefix, and text).
- An **input area** with the current command text.
- A **status bar** with current port label, echo state, and keybinding hints.

Timestamps are rendered using `chrono` in `HH:MM:SS` format.

### Input handling (`ui::input`)

Transforms `crossterm` key events into actions on `App`:

- Plain text input, Backspace, Enter.
- History navigation with Arrow Up/Down.
- Echo toggle: `Ctrl+E`.
- Tab switching: `Tab` / `Shift+Tab`.
- `Esc` to quit.

On Enter:

- Calls `app.submit_input()`.
- If a command was sent, appends it to persistent history via `storage::history::append_command`.

## Storage

### Config (`storage::config`)

`AppConfig`:

- `profiles: Vec<PortProfile>`
- `default_log_dir: Option<String>`

`PortProfile` mirrors `SerialConfig` plus a human‑readable name and optional log paths.

Paths:

- `config_dir()`:
  - If `COMCHAT_CONFIG_DIR` env var is set, it is used directly (convenient for tests).
  - Otherwise, uses `dirs_next::config_dir()` and appends `comchat/`.
- `config_file_path()`:
  - Returns `<config_dir>/config.toml`.

API:

- `load_config()`:
  - Returns `AppConfig::default()` if file does not exist.
  - Uses `toml` for parsing; maps parse errors into `AppError::Config`.
- `save_config()`:
  - Serializes `AppConfig` back to TOML and writes it to disk.

There is a unit test that round‑trips a config using a temporary directory and the `COMCHAT_CONFIG_DIR` override.

### History (`storage::history`)

Implements persistent command history:

- History file: `<config_dir>/history.txt`.
- `load_history()`:
  - Reads non‑empty lines into a `Vec<String>`.
- `append_command(cmd)`:
  - Creates the directory if needed and appends the command with a newline.

The UI layer loads history at startup and appends on every sent command.

### Logging (`storage::logging`)

`LogHandles`:

- Holds optional paths for commands and responses logs.
- Methods:
  - `log_command(&str)`
  - `log_response(&str)`

Internally uses `append_line(path, line)` to create directories, open the file in append mode, and write a UTF‑8 line.

The log paths are chosen in `main` from `AppConfig.default_log_dir` or the default config directory.

## Batch mode

File: `src/core/batch.rs`

`run_batch(cli: &Cli, logger: &LogHandles)`:

1. Validates that `cli.batch` and `cli.port` are provided.
2. Opens the batch file and the target serial port.
3. For each non‑empty line in the batch file:
   - Logs the command.
   - Sends it to the device (with appended newline).
   - Reads responses until a read timeout, logging each response and echoing to stdout.
   - Applies an optional per‑command delay (`--delay-ms`).

This is used from `main` when `--batch` is supplied.

## Error handling

`src/error.rs` defines:

- `AppError`:
  - `Serial(String)`
  - `Io(std::io::Error)`
  - `Config(String)`
  - `ChannelSend(String)`
  - `Other(String)`
- `Result<T>` alias.

Serial‑related code (workers and batch) maps errors into `AppError::Serial`. Config parsing and serialization map into `AppError::Config`. Channel errors become `ChannelSend`.

The main entry points (`main`, `run_batch_only`) print human‑readable error messages and exit with a non‑zero code for batch failures.

## Testing

Existing tests:

- `ui::app::tests::history_navigation_basic`:
  - Verifies navigation semantics of `CommandHistory`.
- `storage::config::tests::config_roundtrip_in_temp_dir`:
  - Uses `tempfile` and `COMCHAT_CONFIG_DIR` to confirm config save/load round‑trip behavior.

You can add further tests:

- For message formatting in `ui::view`.
- For additional storage behaviors (e.g. history edge cases).
- For error mapping functions.

Run all tests with:

```bash
cargo test
```

## Extending COMChat

Some ideas and guidelines for extensions:

- **HEX mode**:
  - Add a toggle in `App` and format sent/received bytes accordingly in `ui::view`.
- **Binary sending**:
  - Extend batch mode to support hex / binary lines (e.g. `0x01 0x02`).
- **Port profiles in UI**:
  - Expose `AppConfig.profiles` in a TUI dialog for selecting ports and presets.
- **Plugins / macros**:
  - Introduce a higher‑level command layer on top of raw text commands.

When extending:

- Keep cross‑platform behavior in mind for paths and port names.
- Avoid blocking the UI thread; prefer channels and worker threads.
- Add tests for any new non‑trivial logic.

