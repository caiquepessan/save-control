//! `config.json`: ajustes do app neste PC (cofre, janela, capas…).

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use savecontrol_core::error::{Error, Result};
use savecontrol_core::fsutil::write_atomic;

use crate::texts;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowState {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub maximized: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub vault_dir: Option<PathBuf>,
    pub first_run_done: bool,
    pub steam_covers: bool,
    pub include_config: bool,
    pub window: Option<WindowState>,
    pub last_scan: Option<DateTime<Utc>>,
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            vault_dir: None,
            first_run_done: false,
            steam_covers: true,
            include_config: false,
            window: None,
            last_scan: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct LoadedConfig {
    pub config: AppConfig,
    /// Aviso para mostrar ao usuário (ex.: config danificada, voltou ao padrão).
    pub warning: Option<String>,
}

/// Arquivo ausente = padrão. Arquivo danificado = padrão + aviso, e o original
/// fica guardado como `config.json.corrompido`. Nunca falha.
pub fn load_config(path: &Path) -> LoadedConfig {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return LoadedConfig { config: AppConfig::default(), warning: None };
        }
        Err(_) => {
            return LoadedConfig { config: AppConfig::default(), warning: Some(texts::CONFIG_RESET.to_string()) };
        }
    };
    match serde_json::from_str(&text) {
        Ok(config) => LoadedConfig { config, warning: None },
        Err(_) => {
            let mut bad = path.as_os_str().to_owned();
            bad.push(".corrompido");
            let _ = std::fs::rename(path, PathBuf::from(bad));
            LoadedConfig { config: AppConfig::default(), warning: Some(texts::CONFIG_RESET.to_string()) }
        }
    }
}

pub fn save_config(path: &Path, c: &AppConfig) -> Result<()> {
    let json = serde_json::to_vec_pretty(c).map_err(|e| Error::Parse(e.to_string()))?;
    write_atomic(path, &json)
}
