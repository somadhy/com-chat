use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph, Tabs},
    Frame,
};

use crate::core::{ChatMessage, MessageKind};
use crate::ui::app::App;

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
    let status_text = "COMChat - press Esc to quit";
    let status = Paragraph::new(status_text)
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    f.render_widget(status, chunks[3]);
}

fn format_message(msg: &ChatMessage) -> Line<'static> {
    let prefix = match msg.kind {
        MessageKind::UserCommand => "> ",
        MessageKind::DeviceResponse => "< ",
        MessageKind::SystemInfo => "i ",
        MessageKind::Error => "! ",
    };
    // Timestamp formatting can be added later; keep it simple for now.
    Line::from(format!("{prefix}{}", msg.text))
}

