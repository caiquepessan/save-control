//! Lojas e pastas onde os jogos estão instalados.

pub mod steam;
pub mod vdf;

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Store {
    Steam,
    Epic,
    Gog,
    Other,
}

/// Um jogo instalado encontrado numa loja ou numa pasta de jogos do usuário.
#[derive(Clone, Debug, PartialEq)]
pub struct InstalledGame {
    pub store: Store,
    pub root: PathBuf,
    pub folder_name: String,
    pub base: PathBuf,
    pub store_game_id: Option<String>,
    pub display_name: Option<String>,
}
