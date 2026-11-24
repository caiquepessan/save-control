//! Pastas de jogos do usuário (ex.: `D:\Jogos`): cada subpasta é um jogo.

use std::path::Path;

use crate::stores::{InstalledGame, Store};

pub fn other_installed(user_root: &Path) -> Vec<InstalledGame> {
    let Ok(dir) = std::fs::read_dir(user_root) else { return Vec::new() };
    let mut out: Vec<InstalledGame> = dir
        .flatten()
        .filter(|e| e.path().is_dir())
        .map(|e| InstalledGame {
            store: Store::Other,
            root: user_root.to_path_buf(),
            folder_name: e.file_name().to_string_lossy().into_owned(),
            base: e.path(),
            store_game_id: None,
            display_name: None,
        })
        .collect();
    out.sort_by(|a, b| a.folder_name.cmp(&b.folder_name));
    out
}
