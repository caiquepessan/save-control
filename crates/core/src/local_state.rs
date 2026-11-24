//! O que é específico deste PC: jogos à mão, ajustes por jogo e pastas de jogos.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::custom::CustomGame;
use crate::error::{Error, Result, io};
use crate::fsutil::write_atomic;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LocalGameSettings {
    /// Nome mostrado neste PC (o app usa; a Engine e o cofre mantêm o título original).
    pub title: Option<String>,
    pub ignored: bool,
    pub game_dir: Option<PathBuf>,
    pub cover: Option<PathBuf>,
    pub auto_backup: Option<bool>,
    pub include_config: Option<bool>,
    /// Template da raiz → pasta escolhida neste PC na hora de restaurar.
    pub target_choices: BTreeMap<String, PathBuf>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LocalState {
    pub custom_games: Vec<CustomGame>,
    pub settings: BTreeMap<String, LocalGameSettings>,
    pub user_roots: Vec<PathBuf>,
}

impl LocalState {
    /// Arquivo ausente vale como estado vazio.
    pub fn load(path: &Path) -> Result<LocalState> {
        match std::fs::read_to_string(path) {
            Ok(text) => serde_json::from_str(&text)
                .map_err(|e| Error::Parse(format!("{}: {e}", path.display()))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(LocalState::default()),
            Err(e) => Err(io(path)(e)),
        }
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_vec_pretty(self).map_err(|e| Error::Parse(e.to_string()))?;
        write_atomic(path, &json)
    }
}
