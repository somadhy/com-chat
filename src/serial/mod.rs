use std::io::{Read, Write};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

use serialport::{DataBits, FlowControl as SpFlow, Parity as SpParity, SerialPort, StopBits as SpStopBits};

use crate::core::{AppEvent, AppEventSender, PortId, SerialConfig};
use crate::error::{AppError, Result};

#[derive(Debug)]
pub enum SerialCommand {
    Write(Vec<u8>),
    Close,
}

pub struct SerialWorkerHandle {
    pub id: PortId,
    pub name: String,
    pub command_tx: Sender<SerialCommand>,
    pub join_handle: thread::JoinHandle<()>,
}

pub fn spawn_serial_worker(
    port_id: PortId,
    config: SerialConfig,
    app_tx: AppEventSender,
) -> Result<SerialWorkerHandle> {
    let port_name = config.port_name.clone();
    let timeout = Duration::from_millis(config.timeout_ms);

    let mut builder = serialport::new(&config.port_name, config.baud_rate)
        .timeout(timeout)
        .data_bits(match config.data_bits {
            5 => DataBits::Five,
            6 => DataBits::Six,
            7 => DataBits::Seven,
            _ => DataBits::Eight,
        })
        .stop_bits(match config.stop_bits {
            crate::core::StopBits::One => SpStopBits::One,
            crate::core::StopBits::Two => SpStopBits::Two,
        })
        .parity(match config.parity {
            crate::core::Parity::None => SpParity::None,
            crate::core::Parity::Even => SpParity::Even,
            crate::core::Parity::Odd => SpParity::Odd,
        })
        .flow_control(match config.flow_control {
            crate::core::FlowControl::None => SpFlow::None,
            crate::core::FlowControl::Hardware => SpFlow::Hardware,
            crate::core::FlowControl::Software => SpFlow::Software,
        });

    let port = builder
        .open()
        .map_err(|e| AppError::Serial(format!("Failed to open {}: {e}", config.port_name)))?;

    let (tx, rx): (Sender<SerialCommand>, Receiver<SerialCommand>) = mpsc::channel();

    let join_handle = thread::spawn(move || worker_loop(port_id, port, rx, app_tx, timeout));

    Ok(SerialWorkerHandle {
        id: port_id,
        name: port_name,
        command_tx: tx,
        join_handle,
    })
}

fn worker_loop(
    port_id: PortId,
    mut port: Box<dyn SerialPort>,
    rx: Receiver<SerialCommand>,
    app_tx: AppEventSender,
    timeout: Duration,
) {
    let mut buf = [0u8; 1024];

    loop {
        // Handle outgoing commands, non-blocking.
        while let Ok(cmd) = rx.try_recv() {
            match cmd {
                SerialCommand::Write(data) => {
                    if let Err(e) = port.write_all(&data) {
                        let _ = app_tx.send(AppEvent::SerialError {
                            port_id,
                            error: format!("write error: {e}"),
                        });
                    }
                }
                SerialCommand::Close => {
                    let _ = app_tx.send(AppEvent::PortClosed { port_id });
                    return;
                }
            }
        }

        // Read incoming data with timeout.
        match port.read(&mut buf) {
            Ok(n) if n > 0 => {
                let data = buf[..n].to_vec();
                let _ = app_tx.send(AppEvent::SerialData { port_id, data });
            }
            Ok(_) => {
                // No data read; just continue.
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                // Expected timeout; loop again.
            }
            Err(e) => {
                let _ = app_tx.send(AppEvent::SerialError {
                    port_id,
                    error: format!("read error: {e}"),
                });
                // On persistent error, break out and consider port closed.
                let _ = app_tx.send(AppEvent::PortClosed { port_id });
                return;
            }
        }

        // Avoid tight loop if neither commands nor data.
        std::thread::sleep(timeout);
    }
}

