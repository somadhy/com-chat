use std::path::PathBuf;

use clap::Parser;

/// COMChat command line options.
#[derive(Debug, Parser)]
#[command(name = "COMChat", version, about = "TUI serial chat and batch tool")]
pub struct Cli {
    /// Run in batch mode using commands from the given file.
    #[arg(long)]
    pub batch: Option<PathBuf>,

    /// Serial port name for batch mode (e.g. COM3, /dev/ttyUSB0).
    #[arg(long)]
    pub port: Option<String>,

    /// Baud rate for batch mode.
    #[arg(long, default_value_t = 115_200)]
    pub baud: u32,

    /// Optional delay between commands in milliseconds.
    #[arg(long)]
    pub delay_ms: Option<u64>,
}

