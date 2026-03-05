use std::time::SystemTime;

use crate::core::connections::ConnectionManager;
use crate::core::{AppEvent, ChatMessage, MessageKind, PortId, TabId};

pub struct CommandHistory {
    entries: Vec<String>,
    cursor: Option<usize>,
}

impl CommandHistory {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            cursor: None,
        }
    }

    pub fn push(&mut self, cmd: String) {
        if !cmd.is_empty() {
            self.entries.push(cmd);
            self.cursor = None;
        }
    }

    pub fn previous(&mut self) -> Option<&str> {
        if self.entries.is_empty() {
            return None;
        }
        let idx = match self.cursor {
            Some(i) if i > 0 => i - 1,
            Some(i) => i,
            None => self.entries.len().saturating_sub(1),
        };
        self.cursor = Some(idx);
        self.entries.get(idx).map(|s| s.as_str())
    }

    pub fn next(&mut self) -> Option<&str> {
        if self.entries.is_empty() {
            return None;
        }
        let idx = match self.cursor {
            Some(i) if i + 1 < self.entries.len() => i + 1,
            _ => {
                self.cursor = None;
                return None;
            }
        };
        self.cursor = Some(idx);
        self.entries.get(idx).map(|s| s.as_str())
    }
}

pub struct Tab {
    pub id: TabId,
    pub title: String,
    pub port_id: Option<PortId>,
    pub messages: Vec<ChatMessage>,
    pub input: String,
}

pub struct App {
    pub tabs: Vec<Tab>,
    pub active_tab: usize,
    pub history: CommandHistory,
    pub echo: bool,
    pub connections: ConnectionManager,
}

impl App {
    pub fn new(connections: ConnectionManager) -> Self {
        let default_tab = Tab {
            id: 0,
            title: "No Port".to_string(),
            port_id: None,
            messages: Vec::new(),
            input: String::new(),
        };

        Self {
            tabs: vec![default_tab],
            active_tab: 0,
            history: CommandHistory::new(),
            echo: true,
            connections,
        }
    }

    pub fn active_tab_mut(&mut self) -> &mut Tab {
        &mut self.tabs[self.active_tab]
    }

    pub fn handle_serial_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::SerialData { port_id, data } => {
                let text = String::from_utf8_lossy(&data).into_owned();
                self.push_message_for_port(port_id, MessageKind::DeviceResponse, text);
            }
            AppEvent::SerialError { port_id, error } => {
                self.push_message_for_port(port_id, MessageKind::Error, error);
            }
            AppEvent::PortClosed { port_id } => {
                self.push_message_for_port(
                    port_id,
                    MessageKind::SystemInfo,
                    "Port closed".to_string(),
                );
                if let Some(tab) = self
                    .tabs
                    .iter_mut()
                    .find(|t| t.port_id == Some(port_id))
                {
                    tab.port_id = None;
                    tab.title = format!("{} (closed)", tab.title);
                }
            }
        }
    }

    fn push_message_for_port(&mut self, port_id: PortId, kind: MessageKind, text: String) {
        if let Some(tab) = self
            .tabs
            .iter_mut()
            .find(|t| t.port_id == Some(port_id))
        {
            tab.messages.push(ChatMessage {
                timestamp: SystemTime::now(),
                port_id: Some(port_id),
                kind,
                text,
            });
        }
    }

    pub fn submit_input(&mut self) {
        let active_index = self.active_tab;
        if self.tabs[active_index].input.is_empty() {
            return;
        }

        let cmd = std::mem::take(&mut self.tabs[active_index].input);
        self.history.push(cmd.clone());

        // Echo user command into chat if enabled.
        if self.echo {
            let port_id = self.tabs[active_index].port_id;
            self.tabs[active_index]
                .messages
                .push(ChatMessage {
                    timestamp: SystemTime::now(),
                    port_id,
                    kind: MessageKind::UserCommand,
                    text: cmd.clone(),
                });
        }

        if let Some(port_id) = self.tabs[active_index].port_id {
            if let Err(err) = self
                .connections
                .write_to_port(port_id, cmd.as_bytes().to_vec())
            {
                let port_id = self.tabs[active_index].port_id;
                self.tabs[active_index]
                    .messages
                    .push(ChatMessage {
                        timestamp: SystemTime::now(),
                        port_id,
                        kind: MessageKind::Error,
                        text: format!("Failed to send command: {err}"),
                    });
            }
        } else {
            self.tabs[active_index].messages.push(ChatMessage {
                timestamp: SystemTime::now(),
                port_id: None,
                kind: MessageKind::SystemInfo,
                text: "No port attached to this tab".to_string(),
            });
        }
    }
}

