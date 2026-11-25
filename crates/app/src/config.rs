//! `config.json`: ajustes do app neste PC (cofre, janela, capas…).

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use savecontrol_core::auto::AutoRules;
use savecontrol_core::error::{Error, Result};
use savecontrol_core::fsutil::write_atomic;
use savecontrol_core::vault::retention::RetentionPolicy;

use crate::texts;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowState {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub maximized: bool,
}

/// Tempos do backup automático (spec §9), ajustáveis em Configurações.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AutoTimes {
    /// Sem saber se o jogo está aberto: segundos sem mudanças.
    pub quiet_secs: u32,
    /// Segundos depois de o jogo fechar.
    pub after_close_secs: u32,
    /// Com o jogo aberto: no máximo 1 backup a cada tantos minutos.
    pub while_running_mins: u32,
}

impl Default for AutoTimes {
    fn default() -> Self {
        AutoTimes { quiet_secs: 60, after_close_secs: 10, while_running_mins: 15 }
    }
}

impl AutoTimes {
    pub fn rules(&self) -> AutoRules {
        AutoRules {
            quiet: chrono::Duration::seconds(self.quiet_secs.into()),
            after_close: chrono::Duration::seconds(self.after_close_secs.into()),
            while_running: chrono::Duration::minutes(self.while_running_mins.into()),
            ..AutoRules::default()
        }
    }
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
    /// Backup automático ligado (a chave do cartão do cofre).
    pub auto_backup: bool,
    /// Retenção geral dos automáticos (§8.2).
    pub retention: RetentionPolicy,
    pub auto_times: AutoTimes,
    /// Fechar a janela deixa o app na bandeja (§13).
    pub close_to_tray: bool,
    /// Notificações do Windows ligadas (o registro do AUMID existe).
    pub notifications: bool,
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
            auto_backup: true,
            retention: RetentionPolicy::default(),
            auto_times: AutoTimes::default(),
            close_to_tray: true,
            notifications: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct LoadedConfig {
    pub config: AppConfig,
    /// Aviso para mostrar ao usuário (ex.: config danificada, voltou ao padrão).
    pub warning: Option<String>,
    /// O arquivo existe mas não deu para ler agora: o app usa os padrões e não
    /// grava por cima dele nesta sessão.
    pub read_only: bool,
}

/// Arquivo ausente = padrão. Arquivo danificado = padrão + aviso, e o original
/// fica guardado como `config.json.corrompido`. Arquivo que existe mas não dá
/// para ler agora (em uso, sem permissão) = padrão + aviso + só leitura, para
/// não ser sobrescrito. Nunca falha.
pub fn load_config(path: &Path) -> LoadedConfig {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return LoadedConfig { config: AppConfig::default(), warning: None, read_only: false };
        }
        Err(_) => {
            return LoadedConfig {
                config: AppConfig::default(),
                warning: Some(texts::CONFIG_UNREADABLE.to_string()),
                read_only: true,
            };
        }
    };
    match serde_json::from_str(&text) {
        Ok(config) => LoadedConfig { config, warning: None, read_only: false },
        Err(_) => {
            let mut bad = path.as_os_str().to_owned();
            bad.push(".corrompido");
            let _ = std::fs::rename(path, PathBuf::from(bad));
            LoadedConfig { config: AppConfig::default(), warning: Some(texts::CONFIG_RESET.to_string()), read_only: false }
        }
    }
}

pub fn save_config(path: &Path, c: &AppConfig) -> Result<()> {
    let json = serde_json::to_vec_pretty(c).map_err(|e| Error::Parse(e.to_string()))?;
    write_atomic(path, &json)
}
