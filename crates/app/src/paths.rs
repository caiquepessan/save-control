//! Onde o app guarda os dados: ao lado do `.exe` (modo portátil) ou, se a
//! pasta do `.exe` não aceitar gravação, em `%LOCALAPPDATA%\SaveControl`.

use std::path::{Path, PathBuf};

pub const DATA_DIR_NAME: &str = "SaveControl-dados";
pub const VAULT_DIR_NAME: &str = "cofre";

#[derive(Clone, Debug, PartialEq)]
pub struct AppPaths {
    pub exe_dir: PathBuf,
    pub data_dir: PathBuf,
    /// Cofre sugerido no primeiro uso.
    pub default_vault: PathBuf,
    /// `true` quando a pasta do `.exe` não aceitou gravação.
    pub fallback: bool,
}

impl AppPaths {
    pub fn config_file(&self) -> PathBuf {
        self.data_dir.join("config.json")
    }

    pub fn covers_dir(&self) -> PathBuf {
        self.data_dir.join("capas")
    }

    pub fn webview_dir(&self) -> PathBuf {
        self.data_dir.join("webview")
    }

    pub fn activity_file(&self) -> PathBuf {
        self.data_dir.join("atividade.jsonl")
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.data_dir.join("logs")
    }

    pub fn library_cache(&self) -> PathBuf {
        self.data_dir.join("indice").join("biblioteca.json")
    }
}

/// Cria a pasta e confere se dá para gravar nela (gravando e apagando um arquivo).
fn writable(dir: &Path) -> bool {
    if std::fs::create_dir_all(dir).is_err() {
        return false;
    }
    let probe = dir.join(".teste-escrita");
    let ok = std::fs::write(&probe, b"ok").is_ok();
    let _ = std::fs::remove_file(&probe);
    ok
}

pub fn resolve_paths(exe_dir: &Path, local_app_data: &Path) -> AppPaths {
    let data_dir = exe_dir.join(DATA_DIR_NAME);
    if writable(&data_dir) {
        return AppPaths {
            exe_dir: exe_dir.to_path_buf(),
            data_dir,
            default_vault: exe_dir.join(VAULT_DIR_NAME),
            fallback: false,
        };
    }
    let base = local_app_data.join("SaveControl");
    AppPaths { exe_dir: exe_dir.to_path_buf(), default_vault: base.join(VAULT_DIR_NAME), data_dir: base, fallback: true }
}
