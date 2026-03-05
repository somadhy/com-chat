use std::io;
use std::sync::mpsc;
use std::time::{Duration, Instant};

mod core;
mod error;
mod serial;
mod ui;

use core::connections::ConnectionManager;
use core::AppEvent;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use ui::app::App;
use ui::input::{handle_key, InputOutcome};
use ui::view;

fn main() -> io::Result<()> {
    // Setup terminal
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
    let mut app = App::new(connections);
    let tick_rate = Duration::from_millis(250);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| view::draw(f, &app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let CEvent::Key(key) = event::read()? {
                if matches!(handle_key(&mut app, key), InputOutcome::Quit) {
                    return Ok(());
                }
            }
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
