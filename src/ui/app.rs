use std::collections::HashMap;
use std::time::SystemTime;

use crate::core::connections::ConnectionManager;
use crate::core::{
    AppEvent, ChatMessage, FlowControl, MessageKind, Parity, PortId, SerialConfig, StopBits,
};
use crate::storage::config::{AppConfig, load_config};
use crate::storage::logging::LogHandles;

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

    pub fn from_entries(entries: Vec<String>) -> Self {
        Self {
            entries,
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

#[cfg(test)]
mod tests {
    use super::CommandHistory;

    #[test]
    fn history_navigation_basic() {
        let mut h = CommandHistory::new();
        h.push("one".into());
        h.push("two".into());
        h.push("three".into());

        assert_eq!(h.previous(), Some("three"));
        assert_eq!(h.previous(), Some("two"));
        assert_eq!(h.previous(), Some("one"));
        // Stays on first when going further back.
        assert_eq!(h.previous(), Some("one"));

        // Forward
        assert_eq!(h.next(), Some("two"));
        assert_eq!(h.next(), Some("three"));
        // After last, next clears selection.
        assert_eq!(h.next(), None);
    }
}

pub struct Tab {
    pub title: String,
    pub port_id: Option<PortId>,
    pub messages: Vec<ChatMessage>,
    pub input: String,
}

#[derive(Clone)]
pub struct PortChoice {
    pub name: String,
    pub label: String,
}

#[derive(Clone)]
pub struct PortSelectorState {
    pub ports: Vec<PortChoice>,
    pub selected: usize,
    pub baud_rate: u32,
    pub stop_bits: StopBits,
    pub parity: Parity,
    pub flow_control: FlowControl,
    pub echo: bool,
}

impl PortSelectorState {
    const BAUD_RATES: &'static [u32] = &[9600, 19200, 38400, 57600, 115200, 230400];

    pub fn increase_baud(&mut self) {
        let current = self.baud_rate;
        if let Some(&next) = Self::BAUD_RATES.iter().find(|&&b| b > current) {
            self.baud_rate = next;
        } else if let Some(&last) = Self::BAUD_RATES.last() {
            self.baud_rate = last;
        }
    }

    pub fn decrease_baud(&mut self) {
        let current = self.baud_rate;
        if let Some(&prev) = Self::BAUD_RATES.iter().rev().find(|&&b| b < current) {
            self.baud_rate = prev;
        } else if let Some(&first) = Self::BAUD_RATES.first() {
            self.baud_rate = first;
        }
    }

    pub fn next_parity(&mut self) {
        self.parity = match self.parity {
            Parity::None => Parity::Even,
            Parity::Even => Parity::Odd,
            Parity::Odd => Parity::None,
        };
    }

    pub fn next_stop_bits(&mut self) {
        self.stop_bits = match self.stop_bits {
            StopBits::One => StopBits::Two,
            StopBits::Two => StopBits::One,
        };
    }

    pub fn next_flow_control(&mut self) {
        self.flow_control = match self.flow_control {
            FlowControl::None => FlowControl::Hardware,
            FlowControl::Hardware => FlowControl::Software,
            FlowControl::Software => FlowControl::None,
        };
    }
}

pub enum UiMode {
    Normal,
    PortSelector(PortSelectorState),
}

pub struct App {
    pub tabs: Vec<Tab>,
    pub active_tab: usize,
    pub history: CommandHistory,
    pub echo: bool,
    pub connections: ConnectionManager,
    pub logger: LogHandles,
    pub mode: UiMode,
    /// Per-port receive buffers to assemble complete lines from chunks.
    recv_buffers: HashMap<PortId, String>,
    /// Last command sent per port (for suppressing device echo when echo is off).
    last_commands: HashMap<PortId, String>,
}

impl App {
    pub fn new(connections: ConnectionManager, logger: LogHandles) -> Self {
        let default_tab = Tab {
            title: "No Port".to_string(),
            port_id: None,
            messages: Vec::new(),
            input: String::new(),
        };

        Self {
            tabs: vec![default_tab],
            active_tab: 0,
            history: CommandHistory::new(),
            echo: false,
            connections,
            logger,
            mode: UiMode::Normal,
            recv_buffers: HashMap::new(),
            last_commands: HashMap::new(),
        }
    }

    pub fn next_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.active_tab = (self.active_tab + 1) % self.tabs.len();
        }
    }

    pub fn previous_tab(&mut self) {
        if !self.tabs.is_empty() {
            if self.active_tab == 0 {
                self.active_tab = self.tabs.len() - 1;
            } else {
                self.active_tab -= 1;
            }
        }
    }

    pub fn toggle_echo(&mut self) {
        self.echo = !self.echo;
    }

    pub fn start_port_selection(&mut self) {
        match crate::core::connections::ConnectionManager::list_available_ports() {
            Ok(infos) if !infos.is_empty() => {
                let cfg: AppConfig = load_config().unwrap_or_default();

                let ports: Vec<PortChoice> = infos
                    .into_iter()
                    .map(|info| PortChoice {
                        name: info.port_name.clone(),
                        label: format!("{:?}: {}", info.port_type, info.port_name),
                    })
                    .collect();

                // Try to pre-fill settings from first matching profile if available.
                let default_baud = 115_200;
                let default_stop = StopBits::One;
                let default_parity = Parity::None;
                let default_flow = FlowControl::None;

                // For simplicity use the first port's profile, user can change afterward.
                let (baud_rate, stop_bits, parity, flow_control, echo) =
                    if let Some(first) = ports.first() {
                        if let Some(p) = cfg.profile_for_port(&first.name) {
                            (
                                p.baud_rate,
                                p.stop_bits.clone(),
                                p.parity.clone(),
                                p.flow_control.clone(),
                                p.echo,
                            )
                        } else {
                            (
                                default_baud,
                                default_stop,
                                default_parity,
                                default_flow,
                                false,
                            )
                        }
                    } else {
                        (
                            default_baud,
                            default_stop,
                            default_parity,
                            default_flow,
                            false,
                        )
                    };

                self.mode = UiMode::PortSelector(PortSelectorState {
                    ports,
                    selected: 0,
                    baud_rate,
                    stop_bits,
                    parity,
                    flow_control,
                    echo,
                });
            }
            Ok(_) => {
                // No ports found: show system message in active tab.
                if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                    tab.messages.push(ChatMessage {
                        timestamp: SystemTime::now(),
                        port_id: None,
                        kind: MessageKind::SystemInfo,
                        text: "No serial ports found".to_string(),
                    });
                }
            }
            Err(err) => {
                if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                    tab.messages.push(ChatMessage {
                        timestamp: SystemTime::now(),
                        port_id: None,
                        kind: MessageKind::Error,
                        text: format!("Failed to list ports: {err}"),
                    });
                }
            }
        }
    }

    pub fn cancel_port_selection(&mut self) {
        self.mode = UiMode::Normal;
    }

    pub fn confirm_port_selection(&mut self) {
        let state = match &self.mode {
            UiMode::PortSelector(s) => s.clone(),
            UiMode::Normal => return,
        };

        if state.ports.is_empty() {
            self.mode = UiMode::Normal;
            return;
        }

        let choice = &state.ports[state.selected];
        let config = SerialConfig {
            port_name: choice.name.clone(),
            baud_rate: state.baud_rate,
            data_bits: 8,
            stop_bits: state.stop_bits.clone(),
            parity: state.parity.clone(),
            flow_control: state.flow_control.clone(),
            timeout_ms: 100,
            echo: state.echo,
            commands_log_path: None,
            responses_log_path: None,
            profile_name: None,
        };

        // Persist profile for this port.
        let mut cfg = load_config().unwrap_or_default();
        cfg.upsert_profile(crate::storage::config::PortProfile {
            name: choice.name.clone(),
            port_name: choice.name.clone(),
            baud_rate: state.baud_rate,
            data_bits: 8,
            stop_bits: state.stop_bits.clone(),
            parity: state.parity.clone(),
            flow_control: state.flow_control.clone(),
            timeout_ms: 100,
            echo: state.echo,
            commands_log_path: None,
            responses_log_path: None,
        });
        let _ = crate::storage::config::save_config(&cfg);

        match self.connections.open_port(config) {
            Ok(port_id) => {
                if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                    tab.port_id = Some(port_id);
                    tab.title = choice.name.clone();
                    tab.messages.push(ChatMessage {
                        timestamp: SystemTime::now(),
                        port_id: Some(port_id),
                        kind: MessageKind::SystemInfo,
                        text: format!("Opened port {} at {} baud", choice.name, state.baud_rate),
                    });
                }
                self.mode = UiMode::Normal;
            }
            Err(err) => {
                if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                    tab.messages.push(ChatMessage {
                        timestamp: SystemTime::now(),
                        port_id: None,
                        kind: MessageKind::Error,
                        text: format!("Failed to open port {}: {err}", choice.name),
                    });
                }
            }
        }
    }

    pub fn handle_serial_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::SerialData { port_id, data } => {
                let chunk = String::from_utf8_lossy(&data).into_owned();
                let buf = self.recv_buffers.entry(port_id).or_default();
                buf.push_str(&chunk);

                // We'll collect complete lines first to avoid borrowing `self` reentrantly.
                let mut completed: Vec<String> = Vec::new();

                while let Some(idx) = buf.find(['\n', '\r']) {
                    let line = buf[..idx].trim_end_matches(['\r', '\n']).to_string();
                    // Remove this line + its terminator(s) from buffer.
                    let mut remove_len = 1;
                    if buf.len() > idx + 1 {
                        let next = buf.as_bytes()[idx + 1];
                        if (buf.as_bytes()[idx] == b'\r' && next == b'\n')
                            || (buf.as_bytes()[idx] == b'\n' && next == b'\r')
                        {
                            remove_len = 2;
                        }
                    }
                    buf.drain(..idx + remove_len);

                    if !line.is_empty() {
                        completed.push(line);
                    }
                }

                for line in completed {
                    // Always log responses.
                    let _ = self.logger.log_response(&line);

                    // Suppress device echo that exactly repeats the last command
                    // sent to this port. This works both when local echo is on
                    // (we already showed the command as `> ...`) and off.
                    if let Some(last) = self.last_commands.get(&port_id)
                        && last.trim() == line.trim()
                    {
                        continue;
                    }

                    self.push_message_for_port(port_id, MessageKind::DeviceResponse, line);
                }
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
                if let Some(tab) = self.tabs.iter_mut().find(|t| t.port_id == Some(port_id)) {
                    tab.port_id = None;
                    tab.title = format!("{} (closed)", tab.title);
                }
            }
        }
    }

    fn push_message_for_port(&mut self, port_id: PortId, kind: MessageKind, text: String) {
        // Prefer the tab that is explicitly attached to this port.
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.port_id == Some(port_id)) {
            tab.messages.push(ChatMessage {
                timestamp: SystemTime::now(),
                port_id: Some(port_id),
                kind,
                text,
            });
            return;
        }

        // Fallback: if no tab is bound to this port (for any reason),
        // show the message in the currently active tab so that data
        // from the device is never silently lost.
        if let Some(tab) = self.tabs.get_mut(self.active_tab) {
            tab.messages.push(ChatMessage {
                timestamp: SystemTime::now(),
                port_id: Some(port_id),
                kind,
                text,
            });
        }
    }

    pub fn submit_input(&mut self) -> Option<String> {
        let active_index = self.active_tab;
        if self.tabs[active_index].input.is_empty() {
            return None;
        }

        let cmd = std::mem::take(&mut self.tabs[active_index].input);
        self.history.push(cmd.clone());

        // Log command and echo into chat if enabled.
        let _ = self.logger.log_command(&cmd);

        // Echo user command into chat if enabled.
        if self.echo {
            let port_id = self.tabs[active_index].port_id;
            self.tabs[active_index].messages.push(ChatMessage {
                timestamp: SystemTime::now(),
                port_id,
                kind: MessageKind::UserCommand,
                text: cmd.clone(),
            });
        }

        if let Some(port_id) = self.tabs[active_index].port_id {
            // Remember last command for this port (to optionally suppress device echo).
            self.last_commands.insert(port_id, cmd.clone());

            // Use CRLF (\\r\\n) to match PuTTY and many device CLIs.
            let mut data = cmd.clone().into_bytes();
            data.push(b'\r');
            data.push(b'\n');

            if let Err(err) = self.connections.write_to_port(port_id, data) {
                let port_id = self.tabs[active_index].port_id;
                self.tabs[active_index].messages.push(ChatMessage {
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

        Some(cmd)
    }
}
