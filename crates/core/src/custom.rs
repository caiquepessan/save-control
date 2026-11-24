//! Jogos adicionados à mão pelo usuário.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::env::SystemPaths;
use crate::templatize::templatize;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CustomGame {
    pub id: String,
    pub name: String,
    pub save_templates: Vec<String>,
    pub process_names: Vec<String>,
}

impl CustomGame {
    /// Com `game_dir`, pastas de save dentro dela viram `<gameDir>/…`, para o
    /// jogo poder ser restaurado em outro PC depois de apontar a pasta lá.
    pub fn new(
        name: &str,
        save_dirs: &[PathBuf],
        process_names: &[String],
        game_dir: Option<&Path>,
        sys: &SystemPaths,
    ) -> CustomGame {
        CustomGame {
            id: format!("man:{}", uuid::Uuid::new_v4().simple()),
            name: name.trim().to_string(),
            save_templates: save_dirs.iter().map(|d| templatize(d, sys, game_dir)).collect(),
            process_names: process_names.to_vec(),
        }
    }
}
