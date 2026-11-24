//! Lojas e pastas onde os jogos estão instalados.

pub mod epic;
pub mod gog;
pub mod other;
pub mod steam;
pub mod vdf;

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::env::SystemPaths;
use crate::placeholder::InstallCandidate;
use crate::registry::RegistryReader;

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

/// Tudo que está instalado neste PC.
#[derive(Clone, Debug, Default)]
pub struct Installs {
    pub games: Vec<InstalledGame>,
    pub steam_user_ids: Vec<String>,
}

fn eq_ci(a: &str, b: &str) -> bool {
    a.to_lowercase() == b.to_lowercase()
}

impl Installs {
    /// Instalações que correspondem a um jogo da base: pelo id da Steam, pelo
    /// nome da pasta (`installDir` ou título) ou pelo nome exibido na loja.
    pub fn candidates_for(&self, title: &str, install_dirs: &[String], steam_ids: &[u32]) -> Vec<InstallCandidate> {
        let mut out: Vec<InstallCandidate> = Vec::new();
        for g in &self.games {
            let by_id = g.store == Store::Steam
                && g.store_game_id
                    .as_deref()
                    .and_then(|s| s.parse::<u32>().ok())
                    .is_some_and(|id| steam_ids.contains(&id));
            let by_dir = install_dirs.iter().any(|d| eq_ci(d, &g.folder_name)) || eq_ci(&g.folder_name, title);
            let by_name = g.display_name.as_deref().is_some_and(|n| eq_ci(n, title));
            if by_id || by_dir || by_name {
                let c = InstallCandidate {
                    root: g.root.clone(),
                    game: g.folder_name.clone(),
                    base: g.base.clone(),
                    store_game_id: g.store_game_id.clone(),
                };
                if !out.contains(&c) {
                    out.push(c);
                }
            }
        }
        out
    }
}

/// Junta Steam, Epic, GOG e as pastas de jogos do usuário. Loja ausente não é erro.
pub fn detect_installs(
    sys: &SystemPaths,
    reg: &dyn RegistryReader,
    epic_manifests_dir: &Path,
    user_roots: &[PathBuf],
) -> Installs {
    let mut games = Vec::new();
    let mut steam_user_ids = Vec::new();
    let steam_root = steam::find_steam_root(reg).or_else(|| {
        let p = sys.program_files_x86.join("Steam");
        p.exists().then_some(p)
    });
    if let Some(root) = steam_root {
        if let Ok(info) = steam::read_steam(&root) {
            games.extend(info.games);
            steam_user_ids = info.user_ids;
        }
    }
    games.extend(epic::epic_installed(epic_manifests_dir));
    games.extend(gog::gog_installed(reg));
    for root in user_roots {
        games.extend(other::other_installed(root));
    }
    Installs { games, steam_user_ids }
}
