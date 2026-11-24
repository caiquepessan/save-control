//! Epic Games: um `.item` (JSON) por jogo instalado.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::stores::{InstalledGame, Store};

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Item {
    display_name: Option<String>,
    install_location: Option<String>,
    app_name: Option<String>,
}

pub fn epic_installed(manifests_dir: &Path) -> Vec<InstalledGame> {
    let Ok(dir) = std::fs::read_dir(manifests_dir) else { return Vec::new() };
    let mut out = Vec::new();
    for entry in dir.flatten() {
        let path = entry.path();
        if !path.extension().is_some_and(|e| e.eq_ignore_ascii_case("item")) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else { continue };
        let Ok(item) = serde_json::from_str::<Item>(&text) else { continue };
        let Some(loc) = item.install_location.filter(|l| !l.trim().is_empty()) else { continue };
        let base = PathBuf::from(loc);
        let (Some(folder), Some(root)) = (base.file_name(), base.parent()) else { continue };
        out.push(InstalledGame {
            store: Store::Epic,
            root: root.to_path_buf(),
            folder_name: folder.to_string_lossy().into_owned(),
            store_game_id: item.app_name,
            display_name: item.display_name,
            base,
        });
    }
    out.sort_by(|a, b| a.folder_name.cmp(&b.folder_name));
    out
}
