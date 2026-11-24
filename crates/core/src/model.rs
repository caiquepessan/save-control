//! Modelo de um jogo encontrado no escaneamento.

use std::collections::BTreeMap;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::custom::CustomGame;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameSource {
    Steam,
    Epic,
    Gog,
    Other,
    /// Jogo da base com save no PC, mas sem instalação encontrada.
    ManifestOnly,
    Catalog,
    Custom,
}

/// Uma pasta de save: o template portátil, os valores escolhidos e o caminho real.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SaveRoot {
    pub template: String,
    pub bindings: BTreeMap<String, String>,
    pub path: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SaveFile {
    /// Índice em `Game::save_roots`.
    pub root: usize,
    pub rel_path: String,
    pub abs_path: PathBuf,
    pub size: u64,
    pub modified: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Warning {
    /// Save numa pasta que costuma ser limpa (Downloads, Área de Trabalho, Temp).
    RiskyLocation(PathBuf),
    /// Save dentro da pasta do jogo: atualizar apagando a pasta apaga o save.
    InsideGameFolder,
    RegistryNotIncluded,
    StoreCloud(String),
    IgnoredBroadPath(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Game {
    pub id: String,
    pub title: String,
    pub source: GameSource,
    pub install_dirs: Vec<PathBuf>,
    pub game_dir: Option<PathBuf>,
    pub save_roots: Vec<SaveRoot>,
    pub files: Vec<SaveFile>,
    pub last_saved: Option<DateTime<Utc>>,
    pub total_size: u64,
    pub steam_app_id: Option<u32>,
    pub process_names: Vec<String>,
    pub custom: Option<CustomGame>,
    pub warnings: Vec<Warning>,
    /// Só existe no cofre (não foi encontrado neste PC).
    #[serde(default)]
    pub vault_only: bool,
}
