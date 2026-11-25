//! Escaneamento: junta base de PC, catálogo e jogos à mão numa lista de jogos.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use rayon::prelude::*;

use crate::catalog::{CatalogMatch, SplitRule};
use crate::custom::CustomGame;
use crate::env::SystemPaths;
use crate::fsglob::{FindResult, find_files};
use crate::local_state::LocalState;
use crate::manifest::{Manifest, ManifestGame};
use crate::model::{Game, GameSource, SaveFile, SaveRoot, Warning};
use crate::placeholder::{ExpandCtx, Expansion, InstallCandidate, expand, slash};
use crate::stores::{Installs, Store};

pub struct ScanInput<'a> {
    pub sys: &'a SystemPaths,
    pub manifest: Option<&'a Manifest>,
    pub installs: &'a Installs,
    pub catalog: &'a [CatalogMatch],
    pub local: &'a LocalState,
    pub include_config: bool,
}

fn key(p: &Path) -> String {
    slash(p).to_lowercase()
}

fn is_under(path: &Path, dir: &Path) -> bool {
    let (p, d) = (key(path), key(dir));
    !d.is_empty() && (p == d || p.starts_with(&format!("{d}/")))
}

#[derive(Default)]
struct Collector {
    roots: Vec<SaveRoot>,
    files: Vec<SaveFile>,
    broad: Vec<String>,
}

impl Collector {
    fn add(&mut self, template: &str, exp: &Expansion, sys: &SystemPaths) {
        match find_files(exp, sys) {
            FindResult::Nothing => {}
            FindResult::IgnoredBroad => {
                if !self.broad.iter().any(|b| b == template) {
                    self.broad.push(template.to_string());
                }
            }
            FindResult::Found(found) => {
                let idx = match self.roots.iter().position(|r| key(&r.path) == key(&found.root_dir)) {
                    Some(i) => i,
                    None => {
                        self.roots.push(SaveRoot {
                            template: found.root_template.clone(),
                            bindings: exp.bindings.clone(),
                            path: found.root_dir.clone(),
                        });
                        self.roots.len() - 1
                    }
                };
                for f in found.files {
                    if !self.files.iter().any(|x| key(&x.abs_path) == key(&f.abs_path)) {
                        self.files.push(SaveFile {
                            root: idx,
                            rel_path: f.rel_path,
                            abs_path: f.abs_path,
                            size: f.size,
                            modified: f.modified,
                        });
                    }
                }
            }
        }
    }

    fn add_all(&mut self, templates: &[String], ctx: &ExpandCtx) {
        for t in templates {
            for exp in expand(t, ctx) {
                self.add(t, &exp, ctx.sys);
            }
        }
    }
}

fn empty_game(id: String, title: String, source: GameSource) -> Game {
    Game {
        id,
        title,
        source,
        install_dirs: Vec::new(),
        game_dir: None,
        save_roots: Vec::new(),
        files: Vec::new(),
        last_saved: None,
        total_size: 0,
        steam_app_id: None,
        process_names: Vec::new(),
        custom: None,
        warnings: Vec::new(),
        vault_only: false,
    }
}

fn store_of(installs: &Installs, c: &InstallCandidate) -> Option<Store> {
    installs.games.iter().find(|g| g.base == c.base).map(|g| g.store)
}

fn manifest_game(input: &ScanInput, title: &str, g: &ManifestGame) -> Option<Game> {
    let mut steam_ids: Vec<u32> = g.steam_id.into_iter().collect();
    steam_ids.extend(&g.steam_extra);
    let candidates = input.installs.candidates_for(title, &g.install_dirs, &steam_ids);
    let any_steam = candidates.iter().any(|c| store_of(input.installs, c) == Some(Store::Steam));
    let user_ids: &[String] = if any_steam { &input.installs.steam_user_ids } else { &[] };
    let ctx = ExpandCtx {
        sys: input.sys,
        installs: &candidates,
        store_user_ids: user_ids,
        game_dir: None,
        game_name: title,
    };
    // "Incluir configurações": o ajuste do jogo vale mais que o geral.
    let include_config = input
        .local
        .settings
        .get(&format!("pc:{title}"))
        .and_then(|s| s.include_config)
        .unwrap_or(input.include_config);
    let templates: Vec<String> =
        g.save_files_for_windows(include_config).into_iter().map(|f| f.path.clone()).collect();
    let mut col = Collector::default();
    col.add_all(&templates, &ctx);
    if col.files.is_empty() && candidates.is_empty() {
        return None;
    }

    let source = match candidates.first().and_then(|c| store_of(input.installs, c)) {
        Some(Store::Steam) => GameSource::Steam,
        Some(Store::Epic) => GameSource::Epic,
        Some(Store::Gog) => GameSource::Gog,
        Some(Store::Other) => GameSource::Other,
        None => GameSource::ManifestOnly,
    };
    let mut game = empty_game(format!("pc:{title}"), title.to_string(), source);
    game.install_dirs = candidates.iter().map(|c| c.base.clone()).collect();
    game.steam_app_id = g.steam_id;
    game.save_roots = col.roots;
    game.files = col.files;
    if g.has_registry {
        game.warnings.push(Warning::RegistryNotIncluded);
    }
    let cloud = [
        (g.cloud.steam, "steam"),
        (g.cloud.epic, "epic"),
        (g.cloud.gog, "gog"),
        (g.cloud.origin, "origin"),
        (g.cloud.uplay, "uplay"),
    ];
    for (on, store) in cloud {
        if on {
            game.warnings.push(Warning::StoreCloud(store.to_string()));
        }
    }
    game.warnings.extend(col.broad.into_iter().map(Warning::IgnoredBroadPath));
    Some(game)
}

fn stem_of(rel_path: &str) -> String {
    let name = rel_path.rsplit('/').next().unwrap_or(rel_path);
    Path::new(name).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
}

/// Um programa do catálogo é um jogo só, achado em quantas pastas for (ex.: o
/// recomp baixado de novo em outra pasta, ou o modo portátil junto com o AppData).
/// O id não depende da pasta, para o histórico não se perder.
fn catalog_games(input: &ScanInput, matches: &[&CatalogMatch]) -> Vec<Game> {
    let Some(cm) = matches.first() else { return Vec::new() };
    let mut col = Collector::default();
    let mut install_dirs: Vec<PathBuf> = Vec::new();
    let mut processes: Vec<String> = Vec::new();
    for m in matches {
        let ctx = ExpandCtx {
            sys: input.sys,
            installs: &[],
            store_user_ids: &[],
            game_dir: m.game_dir.as_deref(),
            game_name: &m.name,
        };
        col.add_all(&m.save_templates, &ctx);
        if let Some(d) = &m.game_dir {
            if !install_dirs.iter().any(|x| key(x) == key(d)) {
                install_dirs.push(d.clone());
            }
        }
        for p in &m.processes {
            if !processes.iter().any(|x| x.eq_ignore_ascii_case(p)) {
                processes.push(p.clone());
            }
        }
    }
    let base_id = format!("cat:{}", cm.entry_id);
    let make = |id: String, title: String| {
        let mut g = empty_game(id, title, GameSource::Catalog);
        g.install_dirs = install_dirs.clone();
        g.process_names = processes.clone();
        g
    };

    match cm.split {
        None => {
            let mut g = make(base_id, cm.name.clone());
            g.save_roots = col.roots;
            g.files = col.files;
            vec![g]
        }
        Some(SplitRule::ByFileStem) => {
            let mut groups: BTreeMap<String, Vec<SaveFile>> = BTreeMap::new();
            let mut first_stem: BTreeMap<String, String> = BTreeMap::new();
            for f in col.files {
                let stem = stem_of(&f.rel_path);
                let k = stem.to_lowercase();
                first_stem.entry(k.clone()).or_insert(stem);
                groups.entry(k).or_default().push(f);
            }
            groups
                .into_iter()
                .map(|(k, files)| {
                    let mut g = make(format!("{base_id}:{k}"), format!("{} · {}", first_stem[&k], cm.name));
                    // Só as raízes que estes arquivos usam, reindexadas.
                    let mut map = BTreeMap::new();
                    for f in &files {
                        if !map.contains_key(&f.root) {
                            map.insert(f.root, g.save_roots.len());
                            g.save_roots.push(col.roots[f.root].clone());
                        }
                    }
                    g.files = files.into_iter().map(|f| SaveFile { root: map[&f.root], ..f }).collect();
                    g
                })
                .collect()
        }
    }
}

fn custom_game(input: &ScanInput, cg: &CustomGame) -> Game {
    let game_dir = input.local.settings.get(&cg.id).and_then(|s| s.game_dir.clone());
    let ctx = ExpandCtx {
        sys: input.sys,
        installs: &[],
        store_user_ids: &[],
        game_dir: game_dir.as_deref(),
        game_name: &cg.name,
    };
    let mut col = Collector::default();
    col.add_all(&cg.save_templates, &ctx);
    let mut g = empty_game(cg.id.clone(), cg.name.clone(), GameSource::Custom);
    g.game_dir = game_dir;
    g.process_names = cg.process_names.clone();
    g.custom = Some(cg.clone());
    g.save_roots = col.roots;
    g.files = col.files;
    g.warnings.extend(col.broad.into_iter().map(Warning::IgnoredBroadPath));
    g
}

fn is_risky(path: &Path, sys: &SystemPaths) -> bool {
    [&sys.downloads, &sys.desktop, &sys.temp].iter().any(|d| is_under(path, d))
        || key(path).split('/').any(|c| c == "downloads")
}

fn finalize(g: &mut Game, sys: &SystemPaths) {
    let mut extra = Vec::new();
    for r in &g.save_roots {
        if is_risky(&r.path, sys) {
            extra.push(Warning::RiskyLocation(r.path.clone()));
        }
    }
    let inside = g.save_roots.iter().any(|r| {
        g.install_dirs.iter().chain(g.game_dir.iter()).any(|d| is_under(&r.path, d))
    });
    if inside {
        extra.push(Warning::InsideGameFolder);
    }
    for w in extra {
        if !g.warnings.contains(&w) {
            g.warnings.push(w);
        }
    }
    g.last_saved = g.files.iter().map(|f| f.modified).max();
    g.total_size = g.files.iter().map(|f| f.size).sum();
}

/// Escaneia tudo. Resultado: save mais recente primeiro (sem save no fim), depois título.
pub fn scan(input: &ScanInput) -> Vec<Game> {
    let mut games: Vec<Game> = match input.manifest {
        Some(m) => m
            .games
            .par_iter()
            .filter(|(_, g)| g.alias.is_none())
            .filter_map(|(title, g)| manifest_game(input, title, g))
            .collect(),
        None => Vec::new(),
    };
    let mut entries: Vec<&str> = Vec::new();
    for cm in input.catalog {
        if !entries.contains(&cm.entry_id.as_str()) {
            entries.push(&cm.entry_id);
        }
    }
    for id in entries {
        games.extend(catalog_games(input, &catalog_group(input, id)));
    }
    for cg in &input.local.custom_games {
        games.push(custom_game(input, cg));
    }
    games.retain(|g| !is_ignored(input, &g.id));
    for g in &mut games {
        finalize(g, input.sys);
    }
    games.sort_by(|a, b| b.last_saved.cmp(&a.last_saved).then_with(|| a.title.cmp(&b.title)));
    games
}

fn catalog_group<'a>(input: &ScanInput<'a>, entry_id: &str) -> Vec<&'a CatalogMatch> {
    input.catalog.iter().filter(|x| x.entry_id == entry_id).collect()
}

fn is_ignored(input: &ScanInput, id: &str) -> bool {
    input.local.settings.get(id).is_some_and(|s| s.ignored)
}

/// Escaneia um jogo só, com o mesmo resultado que `scan` daria para ele: usado
/// logo antes de um backup ou restauração, para agir sobre o disco de agora.
/// `None` se o jogo não existe (mais) neste PC.
pub fn scan_game(input: &ScanInput, id: &str) -> Option<Game> {
    let mut game = if let Some(title) = id.strip_prefix("pc:") {
        let g = input.manifest?.games.get(title).filter(|g| g.alias.is_none())?;
        manifest_game(input, title, g)?
    } else if let Some(rest) = id.strip_prefix("cat:") {
        let entry_id = rest.split(':').next().unwrap_or(rest);
        catalog_games(input, &catalog_group(input, entry_id)).into_iter().find(|g| g.id == id)?
    } else {
        custom_game(input, input.local.custom_games.iter().find(|c| c.id == id)?)
    };
    if is_ignored(input, &game.id) {
        return None;
    }
    finalize(&mut game, input.sys);
    Some(game)
}
