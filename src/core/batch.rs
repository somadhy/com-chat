use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::thread;
use std::time::Duration;

use serialport::{DataBits, FlowControl, Parity, StopBits};

use crate::cli::Cli;
use crate::error::{AppError, Result};
use crate::storage::logging::LogHandles;

pub fn run_batch(cli: &Cli, logger: &LogHandles) -> Result<()> {
    let batch_file = cli
        .batch
        .as_ref()
        .ok_or_else(|| AppError::Other("batch file path is required".into()))?;
    let port_name = cli
        .port
        .as_ref()
        .ok_or_else(|| AppError::Other("serial port is required for batch mode (--port)".into()))?;

    let delay = cli.delay_ms.unwrap_or(0);

    let file = File::open(batch_file)?;
    let reader = BufReader::new(file);

    let mut port = serialport::new(port_name, cli.baud)
        .data_bits(DataBits::Eight)
        .stop_bits(StopBits::One)
        .parity(Parity::None)
        .flow_control(FlowControl::None)
        .timeout(Duration::from_millis(200))
        .open()
        .map_err(|e| AppError::Serial(format!("Failed to open {port_name}: {e}")))?;

    // Assert DTR/RTS in batch mode as well to mirror PuTTY defaults.
    let _ = port.write_data_terminal_ready(true);
    let _ = port.write_request_to_send(true);

    for line in reader.lines() {
        let cmd = line?;
        if cmd.is_empty() {
            continue;
        }

        logger.log_command(&cmd)?;

        let mut to_send = cmd.into_bytes();
        // Match common terminal behavior (CRLF) so devices that
        // expect a carriage return will respond as in PuTTY.
        to_send.push(b'\r');
        to_send.push(b'\n');
        port.write_all(&to_send)?;

        // Collect responses until timeout without data.
        let mut buf = [0u8; 1024];
        loop {
            match port.read(&mut buf) {
                Ok(n) if n > 0 => {
                    let text = String::from_utf8_lossy(&buf[..n]).into_owned();
                    logger.log_response(&text)?;
                    print!("{text}");
                }
                Ok(_) => {}
                Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                    break;
                }
                Err(e) => {
                    return Err(AppError::Serial(format!("read error: {e}")));
                }
            }
        }

        if delay > 0 {
            thread::sleep(Duration::from_millis(delay));
        }
    }

    Ok(())
}
