use std::time::SystemTime;

use serde_derive::{Deserialize as DeriveDeserialize, Serialize as DeriveSerialize};

pub mod batch;
pub mod connections;

pub type PortId = u32;

#[derive(Debug, Clone, DeriveSerialize, DeriveDeserialize)]
pub enum FlowControl {
    None,
    Hardware,
    Software,
}

#[derive(Debug, Clone, DeriveSerialize, DeriveDeserialize)]
pub enum Parity {
    None,
    Even,
    Odd,
}

#[derive(Debug, Clone, DeriveSerialize, DeriveDeserialize)]
pub enum StopBits {
    One,
    Two,
}

#[allow(dead_code)]
#[derive(Debug, Clone, DeriveSerialize, DeriveDeserialize)]
pub struct SerialConfig {
    pub port_name: String,
    pub baud_rate: u32,
    pub data_bits: u8,
    pub stop_bits: StopBits,
    pub parity: Parity,
    pub flow_control: FlowControl,
    pub timeout_ms: u64,
    pub echo: bool,
    pub commands_log_path: Option<String>,
    pub responses_log_path: Option<String>,
    pub profile_name: Option<String>,
}

impl Default for SerialConfig {
    fn default() -> Self {
        Self {
            port_name: String::new(),
            baud_rate: 115_200,
            data_bits: 8,
            stop_bits: StopBits::One,
            parity: Parity::None,
            flow_control: FlowControl::None,
            timeout_ms: 100,
            echo: false,
            commands_log_path: None,
            responses_log_path: None,
            profile_name: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum MessageKind {
    UserCommand,
    DeviceResponse,
    SystemInfo,
    Error,
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub timestamp: SystemTime,
    #[allow(dead_code)]
    pub port_id: Option<PortId>,
    pub kind: MessageKind,
    pub text: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum AppEvent {
    SerialData { port_id: PortId, data: Vec<u8> },
    SerialError { port_id: PortId, error: String },
    PortClosed { port_id: PortId },
}

pub type AppEventSender = std::sync::mpsc::Sender<AppEvent>;
