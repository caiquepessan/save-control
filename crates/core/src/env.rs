use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

/// Pastas do Windows usadas para resolver os placeholders e para os avisos.
#[derive(Clone, Debug)]
pub struct SystemPaths {
    pub home: PathBuf,
    pub app_data: PathBuf,
    pub local_app_data: PathBuf,
    pub local_app_data_low: PathBuf,
    pub documents: PathBuf,
    pub saved_games: PathBuf,
    pub public: PathBuf,
    pub program_data: PathBuf,
    pub win_dir: PathBuf,
    pub program_files: PathBuf,
    pub program_files_x86: PathBuf,
    pub downloads: PathBuf,
    pub desktop: PathBuf,
    pub temp: PathBuf,
    pub os_user_name: String,
}

impl SystemPaths {
    /// Lê as pastas do Windows atual. Documentos, Downloads e Área de Trabalho
    /// vêm das "known folders", para respeitar redirecionamentos (ex.: OneDrive).
    pub fn detect() -> Result<SystemPaths> {
        let var = |name: &str| std::env::var_os(name).map(PathBuf::from);
        let home = dirs::home_dir()
            .or_else(|| var("USERPROFILE"))
            .ok_or_else(|| Error::NotFound("pasta do usuário".into()))?;
        let app_data = var("APPDATA").unwrap_or_else(|| home.join("AppData").join("Roaming"));
        let local_app_data = var("LOCALAPPDATA").unwrap_or_else(|| home.join("AppData").join("Local"));
        let drive = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into());
        let root = PathBuf::from(format!("{drive}\\"));
        Ok(SystemPaths {
            local_app_data_low: home.join("AppData").join("LocalLow"),
            documents: dirs::document_dir().unwrap_or_else(|| home.join("Documents")),
            saved_games: home.join("Saved Games"),
            public: var("PUBLIC").unwrap_or_else(|| root.join("Users").join("Public")),
            program_data: var("PROGRAMDATA").unwrap_or_else(|| root.join("ProgramData")),
            win_dir: var("WINDIR").unwrap_or_else(|| root.join("Windows")),
            program_files: var("ProgramFiles").unwrap_or_else(|| root.join("Program Files")),
            program_files_x86: var("ProgramFiles(x86)")
                .unwrap_or_else(|| root.join("Program Files (x86)")),
            downloads: dirs::download_dir().unwrap_or_else(|| home.join("Downloads")),
            desktop: dirs::desktop_dir().unwrap_or_else(|| home.join("Desktop")),
            temp: var("TEMP").unwrap_or_else(|| local_app_data.join("Temp")),
            os_user_name: std::env::var("USERNAME").unwrap_or_default(),
            app_data,
            local_app_data,
            home,
        })
    }

    /// Layout padrão do Windows debaixo de `home`. Usado nos testes e para
    /// simular outro PC; pastas fora do perfil ficam em subpastas `_*`.
    pub fn under_home(home: &Path, user: &str) -> SystemPaths {
        let home = home.to_path_buf();
        SystemPaths {
            app_data: home.join("AppData").join("Roaming"),
            local_app_data: home.join("AppData").join("Local"),
            local_app_data_low: home.join("AppData").join("LocalLow"),
            documents: home.join("Documents"),
            saved_games: home.join("Saved Games"),
            public: home.join("_public"),
            program_data: home.join("_programdata"),
            win_dir: home.join("_windows"),
            program_files: home.join("_pf"),
            program_files_x86: home.join("_pf86"),
            downloads: home.join("Downloads"),
            desktop: home.join("Desktop"),
            temp: home.join("AppData").join("Local").join("Temp"),
            os_user_name: user.to_string(),
            home,
        }
    }
}
