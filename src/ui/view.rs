use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph, Tabs},
    Frame,
};

use crate::core::{ChatMessage, MessageKind};
use crate::ui::app::{App, PortSelectorState, UiMode};

pub fn draw(f: &mut Frame, app: &App) {
    let size = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Length(3), // tabs
                Constraint::Min(1),    // messages
                Constraint::Length(3), // input
                Constraint::Length(1), // status
            ]
            .as_ref(),
        )
        .split(size);

    // Tabs bar
    let tab_titles: Vec<Line> = app
        .tabs
        .iter()
        .map(|t| Line::from(t.title.clone()))
        .collect();
    let tabs_widget = Tabs::new(tab_titles)
        .select(app.active_tab)
        .block(Block::default().borders(Borders::ALL).title("Ports"))
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().fg(Color::Yellow));
    f.render_widget(tabs_widget, chunks[0]);

    // Messages area
    let messages_lines: Vec<Line> = app
        .tabs
        .get(app.active_tab)
        .map(|t| t.messages.iter().map(format_message).collect())
        .unwrap_or_default();
    let messages_widget = Paragraph::new(messages_lines)
        .block(Block::default().borders(Borders::ALL).title("Chat"));
    f.render_widget(messages_widget, chunks[1]);

    // Input
    let input_text = app
        .tabs
        .get(app.active_tab)
        .map(|t| t.input.as_str())
        .unwrap_or_default();
    let input_widget = Paragraph::new(Line::from(input_text))
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().borders(Borders::ALL).title("Input"));
    f.render_widget(input_widget, chunks[2]);

    // Status bar
    let active = app.tabs.get(app.active_tab);
    let port_label = active
        .and_then(|t| t.port_id.map(|_| t.title.clone()))
        .unwrap_or_else(|| "No Port".to_string());
    let echo_label = if app.echo { "On" } else { "Off" };
    let status_text = format!(
        "COMChat | Port: {port_label} | Echo: {echo_label} | Esc: quit, Ctrl+E: echo, Ctrl+P: ports, Tab/Shift+Tab: switch tab"
    );
    let status = Paragraph::new(status_text)
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    f.render_widget(status, chunks[3]);

    // Optional port selector overlay
    if let UiMode::PortSelector(state) = &app.mode {
        draw_port_selector(f, state);
    }
}

fn format_message(msg: &ChatMessage) -> Line<'static> {
    use chrono::{DateTime, Local};

    let dt: DateTime<Local> = msg.timestamp.into();
    let time_str = dt.format("%H:%M:%S").to_string();

    let prefix = match msg.kind {
        MessageKind::UserCommand => ">",
        MessageKind::DeviceResponse => "<",
        MessageKind::SystemInfo => "i",
        MessageKind::Error => "!",
    };

    Line::from(format!("[{time_str}] {prefix} {}", msg.text))
}

fn draw_port_selector(f: &mut Frame, state: &PortSelectorState) {
    let area = centered_rect(60, 60, f.area());

    // Clear underlying area
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title("Select Port (Up/Down ports, +/- baud, P parity, S stop bits, F flow, Enter=Open, Esc=Cancel)");

    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut lines = Vec::new();
    for (idx, port) in state.ports.iter().enumerate() {
        let marker = if idx == state.selected { ">" } else { " " };
        lines.push(Line::from(format!("{marker} {}", port.label)));
    }
    lines.push(Line::from("".to_string()));
    lines.push(Line::from(format!(
        "Baud: {} | Parity: {:?} | Stop: {:?} | Flow: {:?}",
        state.baud_rate, state.parity, state.stop_bits, state.flow_control
    )));

    let list = Paragraph::new(lines);
    f.render_widget(list, inner);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    let vertical = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1]);

    vertical[1]
}

