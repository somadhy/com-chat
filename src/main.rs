use std::io;
use std::sync::mpsc;
use std::time::{Duration, Instant};

mod cli;
mod core;
mod error;
mod serial;
mod storage;
mod ui;

use cli::Cli;
use core::AppEvent;
use core::connections::ConnectionManager;

use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CEvent},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use ui::app::App;
use ui::input::{InputOutcome, handle_key};
use ui::view;

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    // Batch mode: no TUI, just run and exit.
    if cli.batch.is_some() {
        if let Err(err) = run_batch_only(&cli) {
            eprintln!("Batch error: {err}");
            std::process::exit(1);
        }
        return Ok(());
    }

    // Setup terminal for interactive TUI.
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_app(&mut terminal);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Error: {err}");
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend + 'static>(
    terminal: &mut Terminal<B>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (app_tx, app_rx) = mpsc::channel::<AppEvent>();
    let connections = ConnectionManager::new(app_tx);

    // Load app configuration (including log directory and port profiles).
    let cfg = storage::config::load_config().unwrap_or_default();

    use std::path::PathBuf;
    let log_dir: PathBuf = if let Some(dir) = &cfg.default_log_dir {
        PathBuf::from(dir)
    } else {
        let mut dir = storage::config::config_dir().unwrap_or_else(|_| PathBuf::from("."));
        dir.push("logs");
        dir
    };

    let commands_path = log_dir.join("commands.log");
    let responses_path = log_dir.join("responses.log");
    let logger = storage::logging::LogHandles::new(Some(commands_path), Some(responses_path));

    let mut app = App::new(connections, logger);

    // Load persisted command history.
    if let Ok(entries) = storage::history::load_history() {
        app.history = ui::app::CommandHistory::from_entries(entries);
    }
    let tick_rate = Duration::from_millis(250);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| view::draw(f, &app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)?
            && matches!(
                event::read()?,
                CEvent::Key(key) if matches!(handle_key(&mut app, key), InputOutcome::Quit)
            )
        {
            return Ok(());
        }

        // Process any pending serial events.
        while let Ok(ev) = app_rx.try_recv() {
            app.handle_serial_event(ev);
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }
}

fn run_batch_only(cli: &Cli) -> Result<(), Box<dyn std::error::Error>> {
    // Load app configuration to determine log directory.
    let cfg = storage::config::load_config().unwrap_or_default();

    use std::path::PathBuf;
    let log_dir: PathBuf = if let Some(dir) = &cfg.default_log_dir {
        PathBuf::from(dir)
    } else {
        let mut dir = storage::config::config_dir().unwrap_or_else(|_| PathBuf::from("."));
        dir.push("logs");
        dir
    };

    let commands_path = log_dir.join("commands.log");
    let responses_path = log_dir.join("responses.log");
    let logger = storage::logging::LogHandles::new(Some(commands_path), Some(responses_path));

    core::batch::run_batch(cli, &logger)?;
    Ok(())
}
