use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

use crate::error::{AppError, Result};

const HISTORY_FILE_NAME: &str = "history.txt";

fn history_path() -> Result<PathBuf> {
    let mut dir = crate::storage::config::config_dir()?;
    dir.push(HISTORY_FILE_NAME);
    Ok(dir)
}

pub fn load_history() -> Result<Vec<String>> {
    let path = history_path()?;
    if !path.exists() {
        return Ok(Vec::new());
    }

    let file = fs::File::open(path)?;
    let reader = BufReader::new(file);
    let mut entries = Vec::new();
    for line in reader.lines() {
        let line = line.map_err(|e| AppError::Io(e))?;
        if !line.is_empty() {
            entries.push(line);
        }
    }
    Ok(entries)
}

pub fn append_command(cmd: &str) -> Result<()> {
    let path = history_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(file, "{cmd}")?;
    Ok(())
}

