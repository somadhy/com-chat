use std::fs;
use std::path::{Path, PathBuf};

use serde_derive::{Deserialize, Serialize};

use crate::core::{FlowControl, Parity, StopBits};
use crate::error::{AppError, Result};

const APP_DIR_NAME: &str = "comchat";
const CONFIG_FILE_NAME: &str = "config.toml";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortProfile {
    pub name: String,
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
}

impl Default for PortProfile {
    fn default() -> Self {
        Self {
            name: "Default".to_string(),
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
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub profiles: Vec<PortProfile>,
    pub default_log_dir: Option<String>,
}

impl AppConfig {
    pub fn upsert_profile(&mut self, profile: PortProfile) {
        if let Some(existing) = self
            .profiles
            .iter_mut()
            .find(|p| p.port_name == profile.port_name)
        {
            *existing = profile;
        } else {
            self.profiles.push(profile);
        }
    }

    pub fn profile_for_port<'a>(&'a self, port_name: &str) -> Option<&'a PortProfile> {
        self.profiles.iter().find(|p| p.port_name == port_name)
    }
}

pub fn config_dir() -> Result<PathBuf> {
    if let Ok(custom) = std::env::var("COMCHAT_CONFIG_DIR") {
        return Ok(PathBuf::from(custom));
    }

    let base = dirs_next::config_dir()
        .ok_or_else(|| AppError::Config("could not determine config directory".into()))?;
    Ok(base.join(APP_DIR_NAME))
}

pub fn config_file_path() -> Result<PathBuf> {
    Ok(config_dir()?.join(CONFIG_FILE_NAME))
}

pub fn load_config() -> Result<AppConfig> {
    let path = config_file_path()?;
    if !Path::new(&path).exists() {
        return Ok(AppConfig::default());
    }

    let contents = fs::read_to_string(&path)?;
    let cfg: AppConfig =
        toml::from_str(&contents).map_err(|e| AppError::Config(format!("parse error: {e}")))?;
    Ok(cfg)
}

#[allow(dead_code)]
pub fn save_config(cfg: &AppConfig) -> Result<()> {
    let dir = config_dir()?;
    fs::create_dir_all(&dir)?;
    let path = dir.join(CONFIG_FILE_NAME);
    let data = toml::to_string_pretty(cfg)
        .map_err(|e| AppError::Config(format!("serialize error: {e}")))?;
    fs::write(path, data)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_roundtrip_in_temp_dir() {
        let dir = tempfile::tempdir().expect("tempdir");
        unsafe {
            std::env::set_var("COMCHAT_CONFIG_DIR", dir.path());
        }

        let mut cfg = AppConfig::default();
        cfg.default_log_dir = Some("logs".to_string());
        cfg.profiles.push(PortProfile::default());

        save_config(&cfg).expect("save");
        let loaded = load_config().expect("load");

        assert_eq!(loaded.default_log_dir, cfg.default_log_dir);
        assert_eq!(loaded.profiles.len(), 1);
    }
}

