use crossterm::event::{KeyCode, KeyEvent};

use crate::ui::app::App;

pub enum InputOutcome {
    Continue,
    Quit,
}

pub fn handle_key(app: &mut App, key: KeyEvent) -> InputOutcome {
    match key.code {
        KeyCode::Char(c) => {
            let tab = &mut app.tabs[app.active_tab];
            tab.input.push(c);
            InputOutcome::Continue
        }
        KeyCode::Backspace => {
            let tab = &mut app.tabs[app.active_tab];
            tab.input.pop();
            InputOutcome::Continue
        }
        KeyCode::Enter => {
            app.submit_input();
            InputOutcome::Continue
        }
        KeyCode::Esc => InputOutcome::Quit,
        KeyCode::Up => {
            if let Some(prev) = app.history.previous() {
                let tab = &mut app.tabs[app.active_tab];
                tab.input.clear();
                tab.input.push_str(prev);
            }
            InputOutcome::Continue
        }
        KeyCode::Down => {
            let replacement = app.history.next().unwrap_or_default();
            let tab = &mut app.tabs[app.active_tab];
            tab.input.clear();
            tab.input.push_str(replacement);
            InputOutcome::Continue
        }
        _ => InputOutcome::Continue,
    }
}

