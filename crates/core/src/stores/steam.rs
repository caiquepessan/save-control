//! Steam: raiz pelo registro, bibliotecas, jogos instalados e contas.

use std::path::{Path, PathBuf};

use crate::error::Result;
use crate::placeholder::slash;
use crate::registry::{Hive, RegistryReader};
use crate::stores::vdf::{self, Vdf};
use crate::stores::{InstalledGame, Store};

pub struct SteamInfo {
    pub root: PathBuf,
    pub libraries: Vec<PathBuf>,
    pub games: Vec<InstalledGame>,
    pub user_ids: Vec<String>,
}

pub fn find_steam_root(reg: &dyn RegistryReader) -> Option<PathBuf> {
    reg.string(Hive::CurrentUser, r"Software\Valve\Steam", "SteamPath")
        .filter(|s| !s.trim().is_empty())
        .map(PathBuf::from)
}

fn same_path(a: &Path, b: &Path) -> bool {
    slash(a).eq_ignore_ascii_case(&slash(b))
}

fn libraries(root: &Path) -> Vec<PathBuf> {
    let mut libs = vec![root.to_path_buf()];
    let file = root.join("steamapps").join("libraryfolders.vdf");
    let Ok(text) = std::fs::read_to_string(file) else { return libs };
    let Ok(v) = vdf::parse(&text) else { return libs };
    let Some(folders) = v.get("libraryfolders") else { return libs };
    for (_, lib) in folders.entries() {
        // Formato novo: objeto com "path"; formato antigo: o valor já é o caminho.
        let path = lib.get("path").and_then(Vdf::as_str).or_else(|| lib.as_str());
        if let Some(p) = path.map(PathBuf::from) {
            if p.is_absolute() && !libs.iter().any(|l| same_path(l, &p)) {
                libs.push(p);
            }
        }
    }
    libs
}

/// `root` é a instalação da Steam (onde fica `userdata`), não a biblioteca.
fn apps_in(root: &Path, lib: &Path) -> Vec<InstalledGame> {
    let Ok(dir) = std::fs::read_dir(lib.join("steamapps")) else { return Vec::new() };
    let mut games = Vec::new();
    for entry in dir.flatten() {
        let name = entry.file_name().to_string_lossy().to_lowercase();
        if !(name.starts_with("appmanifest_") && name.ends_with(".acf")) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(entry.path()) else { continue };
        let Ok(v) = vdf::parse(&text) else { continue };
        let Some(app) = v.get("AppState") else { continue };
        let (Some(id), Some(dir_name)) =
            (app.get("appid").and_then(Vdf::as_str), app.get("installdir").and_then(Vdf::as_str))
        else {
            continue;
        };
        games.push(InstalledGame {
            store: Store::Steam,
            root: root.to_path_buf(),
            folder_name: dir_name.to_string(),
            base: lib.join("steamapps").join("common").join(dir_name),
            store_game_id: Some(id.to_string()),
            display_name: app.get("name").and_then(Vdf::as_str).map(str::to_string),
        });
    }
    games
}

fn user_ids(root: &Path) -> Vec<String> {
    let Ok(dir) = std::fs::read_dir(root.join("userdata")) else { return Vec::new() };
    let mut ids: Vec<String> = dir
        .flatten()
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()) && n != "0")
        .collect();
    ids.sort();
    ids
}

/// Lê uma instalação da Steam. Arquivos ilegíveis são pulados, não derrubam a leitura.
pub fn read_steam(root: &Path) -> Result<SteamInfo> {
    let libraries = libraries(root);
    let games = libraries.iter().flat_map(|l| apps_in(root, l)).collect();
    Ok(SteamInfo { root: root.to_path_buf(), user_ids: user_ids(root), libraries, games })
}
