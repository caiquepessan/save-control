//! Catálogo próprio: recomps, ports e emuladores que a base de PC não cobre.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::env::SystemPaths;
use crate::error::{Error, Result};
use crate::placeholder::{ExpandCtx, expand, slash};

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CatalogKind {
    Recomp,
    Port,
    Emulator,
}

/// Regras prontas para famílias de programas.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CatalogRule {
    /// N64Recomp: com `portable.txt` ao lado do exe, saves em `<gameDir>/saves`;
    /// senão em `%LOCALAPPDATA%\<appdata_dir>\saves`.
    N64recomp,
}

#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SplitRule {
    /// Um jogo por nome de arquivo sem extensão (saves de emulador).
    ByFileStem,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CatalogEntry {
    pub id: String,
    pub name: String,
    pub kind: CatalogKind,
    pub exe: Vec<String>,
    #[serde(default)]
    pub search: Vec<String>,
    #[serde(default)]
    pub saves: Vec<String>,
    #[serde(default)]
    pub rule: Option<CatalogRule>,
    #[serde(default)]
    pub appdata_dir: Option<String>,
    #[serde(default)]
    pub split: Option<SplitRule>,
    #[serde(default)]
    pub processes: Vec<String>,
}

#[derive(Deserialize)]
struct CatalogFile {
    entries: Vec<CatalogEntry>,
}

pub struct Catalog {
    pub entries: Vec<CatalogEntry>,
}

impl Catalog {
    pub fn builtin() -> Catalog {
        Self::from_yaml(include_str!("../../../catalog/catalog.yaml")).expect("catálogo embutido válido")
    }

    pub fn from_yaml(text: &str) -> Result<Catalog> {
        serde_yaml::from_str::<CatalogFile>(text)
            .map(|f| Catalog { entries: f.entries })
            .map_err(|e| Error::Parse(format!("catálogo: {e}")))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CatalogMatch {
    pub entry_id: String,
    pub name: String,
    pub game_dir: Option<PathBuf>,
    pub save_templates: Vec<String>,
    pub split: Option<SplitRule>,
    pub processes: Vec<String>,
}

/// Executáveis encontrados: nome em minúsculas → pastas onde aparecem.
fn index_exes(root: &Path, max_depth: usize, index: &mut HashMap<String, Vec<PathBuf>>) {
    for entry in walkdir::WalkDir::new(root).max_depth(max_depth).into_iter().flatten() {
        if !entry.file_type().is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_lowercase();
        if !name.ends_with(".exe") {
            continue;
        }
        if let Some(dir) = entry.path().parent() {
            let dirs = index.entry(name).or_default();
            if !dirs.iter().any(|d| slash(d).eq_ignore_ascii_case(&slash(dir))) {
                dirs.push(dir.to_path_buf());
            }
        }
    }
}

fn appdata_template(entry: &CatalogEntry) -> String {
    format!("<winLocalAppData>/{}/saves", entry.appdata_dir.as_deref().unwrap_or(&entry.name))
}

fn templates_for(entry: &CatalogEntry, game_dir: &Path) -> Vec<String> {
    match entry.rule {
        Some(CatalogRule::N64recomp) if game_dir.join("portable.txt").exists() => {
            vec!["<gameDir>/saves".to_string()]
        }
        Some(CatalogRule::N64recomp) => vec![appdata_template(entry)],
        None => entry.saves.clone(),
    }
}

/// Procura os programas do catálogo nas pastas de jogos do usuário
/// (até 3 níveis) e nas pastas típicas de cada um (até 2 níveis).
pub fn find_catalog_games(catalog: &Catalog, sys: &SystemPaths, user_roots: &[PathBuf]) -> Vec<CatalogMatch> {
    let mut in_roots = HashMap::new();
    for root in user_roots {
        index_exes(root, 3, &mut in_roots);
    }

    let mut out: Vec<CatalogMatch> = Vec::new();
    for entry in &catalog.entries {
        let mut in_search = HashMap::new();
        for t in &entry.search {
            let ctx = ExpandCtx { sys, installs: &[], store_user_ids: &[], game_dir: None, game_name: &entry.name };
            for e in expand(t, &ctx).into_iter().filter(|e| e.tail_glob.is_none()) {
                index_exes(&e.root_path, 2, &mut in_search);
            }
        }
        let mut dirs: Vec<&PathBuf> = Vec::new();
        for exe in &entry.exe {
            let key = exe.to_lowercase();
            for d in in_roots.get(&key).into_iter().chain(in_search.get(&key)).flatten() {
                if !dirs.iter().any(|x| slash(x).eq_ignore_ascii_case(&slash(d))) {
                    dirs.push(d);
                }
            }
        }

        let mut push = |game_dir: Option<PathBuf>, save_templates: Vec<String>| {
            let m = CatalogMatch {
                entry_id: entry.id.clone(),
                name: entry.name.clone(),
                game_dir,
                save_templates,
                split: entry.split,
                processes: entry.processes.clone(),
            };
            if !out.iter().any(|x| x.entry_id == m.entry_id && x.game_dir == m.game_dir) {
                out.push(m);
            }
        };
        let mut appdata_covered = false;
        for d in dirs {
            let templates = templates_for(entry, d);
            appdata_covered |= templates.iter().any(|t| *t == appdata_template(entry));
            push(Some(d.clone()), templates);
        }
        // Saves no AppData sem exe achado (ou com o exe agora em modo portátil).
        if entry.rule == Some(CatalogRule::N64recomp) && !appdata_covered {
            let dir = sys
                .local_app_data
                .join(entry.appdata_dir.as_deref().unwrap_or(&entry.name))
                .join("saves");
            if dir.is_dir() {
                push(None, vec![appdata_template(entry)]);
            }
        }
    }
    out
}
