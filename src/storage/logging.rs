use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::error::Result;

#[derive(Clone)]
pub struct LogHandles {
    commands: Option<PathBuf>,
    responses: Option<PathBuf>,
}

impl LogHandles {
    pub fn new(commands: Option<PathBuf>, responses: Option<PathBuf>) -> Self {
        Self {
            commands,
            responses,
        }
    }

    pub fn log_command(&self, line: &str) -> Result<()> {
        if let Some(path) = &self.commands {
            append_line(path, line)?;
        }
        Ok(())
    }

    pub fn log_response(&self, line: &str) -> Result<()> {
        if let Some(path) = &self.responses {
            append_line(path, line)?;
        }
        Ok(())
    }
}

fn append_line(path: &Path, line: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{line}")?;
    Ok(())
}
