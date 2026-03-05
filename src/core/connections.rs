use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::thread;

use serialport::SerialPortInfo;

use crate::core::{AppEventSender, PortId, SerialConfig};
use crate::error::{AppError, Result};
use crate::serial::{spawn_serial_worker, SerialCommand};

#[allow(dead_code)]
pub struct PortHandle {
    pub id: PortId,
    pub name: String,
    command_tx: Sender<SerialCommand>,
    join_handle: thread::JoinHandle<()>,
}

impl PortHandle {
    pub fn send(&self, cmd: SerialCommand) -> Result<()> {
        self.command_tx
            .send(cmd)
            .map_err(|e| AppError::ChannelSend(e.to_string()))
    }
}

#[allow(dead_code)]
pub struct ConnectionManager {
    next_id: PortId,
    ports: HashMap<PortId, PortHandle>,
    app_tx: AppEventSender,
}

#[allow(dead_code)]
impl ConnectionManager {
    pub fn new(app_tx: AppEventSender) -> Self {
        Self {
            next_id: 1,
            ports: HashMap::new(),
            app_tx,
        }
    }

    pub fn list_available_ports() -> Result<Vec<SerialPortInfo>> {
        serialport::available_ports().map_err(|e| AppError::Serial(e.to_string()))
    }

    pub fn open_port(&mut self, config: SerialConfig) -> Result<PortId> {
        let id = self.next_id;
        self.next_id += 1;

        let worker_handle = spawn_serial_worker(id, config, self.app_tx.clone())?;

        let handle = PortHandle {
            id: worker_handle.id,
            name: worker_handle.name,
            command_tx: worker_handle.command_tx,
            join_handle: worker_handle.join_handle,
        };

        self.ports.insert(id, handle);
        Ok(id)
    }

    pub fn close_port(&mut self, id: PortId) -> Result<()> {
        if let Some(handle) = self.ports.remove(&id) {
            handle.send(SerialCommand::Close)?;
            // We intentionally detach here; worker will exit on its own.
            Ok(())
        } else {
            Err(AppError::Serial(format!("Port {id} not found")))
        }
    }

    pub fn write_to_port(&self, id: PortId, data: Vec<u8>) -> Result<()> {
        if let Some(handle) = self.ports.get(&id) {
            handle.send(SerialCommand::Write(data))
        } else {
            Err(AppError::Serial(format!("Port {id} not found")))
        }
    }
}

