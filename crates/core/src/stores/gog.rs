//! GOG Galaxy: uma subchave do registro por jogo instalado.

use std::path::PathBuf;

use crate::registry::{Hive, RegistryReader};
use crate::stores::{InstalledGame, Store};

const GAMES_KEY: &str = r"SOFTWARE\WOW6432Node\GOG.com\Games";

pub fn gog_installed(reg: &dyn RegistryReader) -> Vec<InstalledGame> {
    reg.subkeys(Hive::LocalMachine, GAMES_KEY)
        .into_iter()
        .filter_map(|id| {
            let key = format!(r"{GAMES_KEY}\{id}");
            let base = PathBuf::from(reg.string(Hive::LocalMachine, &key, "path")?);
            let folder = base.file_name()?.to_string_lossy().into_owned();
            let root = base.parent()?.to_path_buf();
            Some(InstalledGame {
                store: Store::Gog,
                root,
                folder_name: folder,
                display_name: reg.string(Hive::LocalMachine, &key, "gameName"),
                store_game_id: Some(id),
                base,
            })
        })
        .collect()
}
