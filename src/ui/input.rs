use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::storage;
use crate::ui::app::{App, PortSelectorState, UiMode};

pub enum InputOutcome {
    Continue,
    Quit,
}

enum PortSelectorAction {
    None,
    Confirm,
    Cancel,
}

pub fn handle_key(app: &mut App, key: KeyEvent) -> InputOutcome {
    // Ignore key repeat and key release events; act only on key press.
    if key.kind != KeyEventKind::Press {
        return InputOutcome::Continue;
    }

    match &mut app.mode {
        UiMode::Normal => handle_key_normal(app, key),
        UiMode::PortSelector(state) => {
            match handle_key_port_selector(state, key) {
                PortSelectorAction::None => InputOutcome::Continue,
                PortSelectorAction::Confirm => {
                    app.confirm_port_selection();
                    InputOutcome::Continue
                }
                PortSelectorAction::Cancel => {
                    app.cancel_port_selection();
                    InputOutcome::Continue
                }
            }
        }
    }
}

fn handle_key_normal(app: &mut App, key: KeyEvent) -> InputOutcome {
    // Global shortcuts with modifiers
    if key.code == KeyCode::Char('e') && key.modifiers.contains(KeyModifiers::CONTROL) {
        app.toggle_echo();
        return InputOutcome::Continue;
    }

    if key.code == KeyCode::Char('p') && key.modifiers.contains(KeyModifiers::CONTROL) {
        app.start_port_selection();
        return InputOutcome::Continue;
    }

    if key.code == KeyCode::Tab && key.modifiers.contains(KeyModifiers::SHIFT) {
        app.previous_tab();
        return InputOutcome::Continue;
    } else if key.code == KeyCode::Tab {
        app.next_tab();
        return InputOutcome::Continue;
    }

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
            if let Some(cmd) = app.submit_input() {
                let _ = storage::history::append_command(&cmd);
            }
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

fn handle_key_port_selector(state: &mut PortSelectorState, key: KeyEvent) -> PortSelectorAction {
    match key.code {
        KeyCode::Up => {
            if state.selected > 0 {
                state.selected -= 1;
            }
            PortSelectorAction::None
        }
        KeyCode::Down => {
            if state.selected + 1 < state.ports.len() {
                state.selected += 1;
            }
            PortSelectorAction::None
        }
        KeyCode::Char('+') | KeyCode::Right => {
            state.increase_baud();
            PortSelectorAction::None
        }
        KeyCode::Char('-') | KeyCode::Left => {
            state.decrease_baud();
            PortSelectorAction::None
        }
        KeyCode::Char('p') | KeyCode::Char('P') => {
            state.next_parity();
            PortSelectorAction::None
        }
        KeyCode::Char('s') | KeyCode::Char('S') => {
            state.next_stop_bits();
            PortSelectorAction::None
        }
        KeyCode::Char('f') | KeyCode::Char('F') => {
            state.next_flow_control();
            PortSelectorAction::None
        }
        KeyCode::Enter => {
            PortSelectorAction::Confirm
        }
        KeyCode::Esc => {
            PortSelectorAction::Cancel
        }
        _ => PortSelectorAction::None,
    }
}

