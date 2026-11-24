//! Fachada do núcleo: junta base, lojas, catálogo, cofre, backup e restauração.
//! Usada pelo CLI e, no Plano 2, pelo app Tauri.

use std::collections::BTreeMap;
use std::path::PathBuf;

use chrono::{Local, Utc};

use crate::backup::{BackupCtx, BackupOutcome, backup_game};
use crate::catalog::{Catalog, find_catalog_games};
use crate::custom::CustomGame;
use crate::env::SystemPaths;
use crate::error::{Error, Result};
use crate::fingerprint::{HashCache, fingerprint_of_game};
use crate::local_state::LocalState;
use crate::machine::MachineIdentity;
use crate::manifest::Manifest;
use crate::manifest::store::{Fetcher, ManifestStore, UpdateOutcome};
use crate::model::{Game, GameSource, SaveRoot};
use crate::placeholder::InstallCandidate;
use crate::process::ProcessProbe;
use crate::registry::RegistryReader;
use crate::restore::{RestoreOptions, RestoreReport, restore_snapshot};
use crate::scan::{ScanInput, scan};
use crate::status::{GameStatus, status_of};
use crate::stores::{Installs, Store, detect_installs};
use crate::targets::TargetCtx;
use crate::vault::Vault;
use crate::vault::format::SnapshotKind;
use crate::vault::index::{SnapshotEntry, VaultIndex};
use crate::vault::read::read_meta;
use crate::vault::retention::RetentionPolicy;
use crate::vault::write::RealHooks;

pub struct EngineConfig {
    pub data_dir: PathBuf,
    pub vault_dir: PathBuf,
    pub include_config: bool,
}

pub struct Engine {
    cfg: EngineConfig,
    sys: SystemPaths,
    reg: Box<dyn RegistryReader>,
    store: ManifestStore,
    manifest: Option<Manifest>,
    local: LocalState,
    vault: Vault,
    index: VaultIndex,
    hashes: HashCache,
    machine: MachineIdentity,
    retention: RetentionPolicy,
    installs: Installs,
    games: Vec<Game>,
}

impl Engine {
    pub fn open(cfg: EngineConfig, sys: SystemPaths, reg: Box<dyn RegistryReader>) -> Result<Engine> {
        let store = ManifestStore::new(cfg.data_dir.join("base"));
        // Base corrompida não impede o app de abrir: só fica sem jogos de PC.
        let manifest = store.load().ok().flatten();
        let local = LocalState::load(&cfg.data_dir.join("jogos.json"))?;
        let vault = Vault::open_or_create(&cfg.vault_dir)?;
        let machine = MachineIdentity::detect(&*reg);
        vault.cleanup_partials(&machine, Utc::now())?;
        let index_path = cfg.data_dir.join("indice").join(format!("{}.json", vault.id));
        let mut index = VaultIndex::load(&index_path, &vault.id);
        index.refresh(&vault)?;
        index.save(&index_path)?;
        let hashes = HashCache::load(&cfg.data_dir.join("indice").join("hashes.json"));
        Ok(Engine {
            cfg,
            sys,
            reg,
            store,
            manifest,
            local,
            vault,
            index,
            hashes,
            machine,
            retention: RetentionPolicy::default(),
            installs: Installs::default(),
            games: Vec::new(),
        })
    }

    fn index_path(&self) -> PathBuf {
        self.cfg.data_dir.join("indice").join(format!("{}.json", self.vault.id))
    }

    fn local_path(&self) -> PathBuf {
        self.cfg.data_dir.join("jogos.json")
    }

    fn persist(&self) -> Result<()> {
        self.index.save(&self.index_path())?;
        self.hashes.save(&self.cfg.data_dir.join("indice").join("hashes.json"))
    }

    pub fn update_manifest(&mut self, fetcher: &dyn Fetcher) -> Result<UpdateOutcome> {
        let out = self.store.update(fetcher, Utc::now())?;
        if out == UpdateOutcome::Updated || self.manifest.is_none() {
            self.manifest = self.store.load()?;
        }
        Ok(out)
    }

    /// Definições de jogos à mão que estão no cofre e ainda não existem neste PC.
    fn import_custom_from_vault(&mut self) -> Result<()> {
        let Ok(dirs) = std::fs::read_dir(&self.vault.root) else { return Ok(()) };
        let mut changed = false;
        for dir in dirs.flatten().filter(|d| d.path().is_dir()) {
            if let Some(def) = Vault::read_descriptor(&dir.path()).and_then(|d| d.custom) {
                if !self.local.custom_games.iter().any(|g| g.id == def.id) {
                    self.local.custom_games.push(def);
                    changed = true;
                }
            }
        }
        if changed {
            self.local.save(&self.local_path())?;
        }
        Ok(())
    }

    /// Jogos com backup no cofre que não foram encontrados neste PC.
    fn vault_only_games(&self, found: &[Game]) -> Vec<Game> {
        let mut out: Vec<Game> = Vec::new();
        for e in &self.index.entries {
            if found.iter().chain(out.iter()).any(|g| g.id == e.game_id) {
                continue;
            }
            let Some(newest) = self.index.for_game(&e.game_id).into_iter().next() else { continue };
            let Ok(meta) = read_meta(&newest.zip_path(&self.vault)) else { continue };
            let desc = Vault::read_descriptor(&self.vault.root.join(&newest.dir_name));
            let settings = self.local.settings.get(&e.game_id);
            if settings.is_some_and(|s| s.ignored) {
                continue;
            }
            out.push(Game {
                id: e.game_id.clone(),
                title: meta.title.clone(),
                source: desc.as_ref().map(|d| d.source.clone()).unwrap_or(GameSource::ManifestOnly),
                install_dirs: Vec::new(),
                game_dir: settings.and_then(|s| s.game_dir.clone()),
                save_roots: meta
                    .roots
                    .iter()
                    .map(|r| SaveRoot { template: r.template.clone(), bindings: r.bindings.clone(), path: r.original_path.clone() })
                    .collect(),
                files: Vec::new(),
                last_saved: None,
                total_size: 0,
                steam_app_id: desc.as_ref().and_then(|d| d.steam_app_id),
                process_names: Vec::new(),
                custom: desc.and_then(|d| d.custom),
                warnings: Vec::new(),
                vault_only: true,
            });
        }
        out
    }

    /// Escaneia o PC e devolve cada jogo com o status em relação ao cofre.
    pub fn scan(&mut self) -> Result<Vec<(Game, GameStatus)>> {
        self.index.refresh(&self.vault)?;
        self.import_custom_from_vault()?;
        let epic = self.sys.program_data.join("Epic").join("EpicGamesLauncher").join("Data").join("Manifests");
        self.installs = detect_installs(&self.sys, &*self.reg, &epic, &self.local.user_roots);
        let catalog = find_catalog_games(&Catalog::builtin(), &self.sys, &self.local.user_roots);
        let input = ScanInput {
            sys: &self.sys,
            manifest: self.manifest.as_ref(),
            installs: &self.installs,
            catalog: &catalog,
            local: &self.local,
            include_config: self.cfg.include_config,
        };
        let mut games = scan(&input);
        games.extend(self.vault_only_games(&games));

        let mut out = Vec::with_capacity(games.len());
        for g in &games {
            // Arquivo travado ou sumido no meio do caminho: conta como "save novo".
            let fp = fingerprint_of_game(g, &mut self.hashes).unwrap_or_default();
            out.push((g.clone(), status_of(g, &fp, &self.index)));
        }
        self.persist()?;
        self.games = games;
        Ok(out)
    }

    fn find_game(&self, game_id: &str) -> Result<Game> {
        self.games
            .iter()
            .find(|g| g.id == game_id)
            .cloned()
            .ok_or_else(|| Error::NotFound(format!("jogo {game_id} (escaneie antes)")))
    }

    pub fn backup(&mut self, game_id: &str, kind: SnapshotKind, label: Option<&str>) -> Result<BackupOutcome> {
        let game = self.find_game(game_id)?;
        let result = {
            let mut ctx = BackupCtx {
                vault: &self.vault,
                index: &mut self.index,
                hashes: &mut self.hashes,
                machine: &self.machine,
                hooks: &RealHooks,
                now: Local::now(),
                app_version: env!("CARGO_PKG_VERSION"),
                retention: &self.retention,
            };
            backup_game(&mut ctx, &game, kind, label)
        };
        self.persist()?;
        result
    }

    pub fn history(&self, game_id: &str) -> Vec<SnapshotEntry> {
        self.index.for_game(game_id).into_iter().cloned().collect()
    }

    /// Instalações deste PC que servem para resolver `<base>`/`<storeUserId>` do jogo.
    fn target_installs(&self, game: &Game) -> (Vec<InstallCandidate>, Vec<String>) {
        let Some(title) = game.id.strip_prefix("pc:") else { return (Vec::new(), Vec::new()) };
        let Some((_, mg)) = self.manifest.as_ref().and_then(|m| m.entry(title)) else {
            return (Vec::new(), Vec::new());
        };
        let mut ids: Vec<u32> = mg.steam_id.into_iter().collect();
        ids.extend(&mg.steam_extra);
        let candidates = self.installs.candidates_for(title, &mg.install_dirs, &ids);
        let steam = candidates
            .iter()
            .any(|c| self.installs.games.iter().any(|g| g.base == c.base && g.store == Store::Steam));
        let users = if steam { self.installs.steam_user_ids.clone() } else { Vec::new() };
        (candidates, users)
    }

    pub fn restore(
        &mut self,
        game_id: &str,
        file_name: &str,
        opts: &RestoreOptions,
        probe: &dyn ProcessProbe,
    ) -> Result<RestoreReport> {
        let game = self.find_game(game_id)?;
        let entry = self
            .history(game_id)
            .into_iter()
            .find(|e| e.file_name == file_name)
            .ok_or_else(|| Error::NotFound(format!("backup {file_name}")))?;
        let others: Vec<Game> = self.games.iter().filter(|g| g.id != game_id).cloned().collect();
        let settings = self.local.settings.get(game_id).cloned().unwrap_or_default();
        let remembered: BTreeMap<String, PathBuf> = settings.target_choices.clone();
        let (installs, user_ids) = self.target_installs(&game);
        let targets = TargetCtx {
            sys: &self.sys,
            installs,
            store_user_ids: user_ids,
            game_dir: game.game_dir.clone().or(settings.game_dir),
            remembered: &remembered,
        };
        let result = {
            let mut ctx = BackupCtx {
                vault: &self.vault,
                index: &mut self.index,
                hashes: &mut self.hashes,
                machine: &self.machine,
                hooks: &RealHooks,
                now: Local::now(),
                app_version: env!("CARGO_PKG_VERSION"),
                retention: &self.retention,
            };
            restore_snapshot(&mut ctx, &targets, probe, &game, &others, &entry, opts)
        };
        self.persist()?;
        result
    }

    pub fn add_custom_game(&mut self, name: &str, save_dirs: &[PathBuf], processes: &[String]) -> Result<CustomGame> {
        let g = CustomGame::new(name, save_dirs, processes, &self.sys);
        self.local.custom_games.push(g.clone());
        self.local.save(&self.local_path())?;
        Ok(g)
    }

    pub fn set_user_roots(&mut self, roots: Vec<PathBuf>) -> Result<()> {
        self.local.user_roots = roots;
        self.local.save(&self.local_path())
    }

    pub fn machine(&self) -> &MachineIdentity {
        &self.machine
    }
}
