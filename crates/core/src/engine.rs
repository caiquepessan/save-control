//! Fachada do núcleo: junta base, lojas, catálogo, cofre, backup e restauração.
//! Usada pelo CLI e, no Plano 2, pelo app Tauri.

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Local, Utc};

use crate::backup::{BackupCtx, BackupOutcome, backup_game};
use crate::catalog::{Catalog, find_catalog_games};
use crate::custom::CustomGame;
use crate::env::SystemPaths;
use crate::error::{Error, Result};
use crate::fingerprint::{HashCache, fingerprint_of_game};
use crate::local_state::{LocalGameSettings, LocalState};
use crate::machine::MachineIdentity;
use crate::manifest::Manifest;
use crate::manifest::store::{Fetcher, ManifestStore, UpdateOutcome};
use crate::model::{Game, GameSource, SaveRoot};
use crate::placeholder::{InstallCandidate, slash};
use crate::process::{ProcessProbe, is_game_running};
use crate::registry::RegistryReader;
use crate::restore::{RestoreOptions, RestoreReport, restore_snapshot};
use crate::scan::{ScanInput, scan, scan_game};
use crate::status::{GameStatus, status_of};
use crate::stores::{Installs, Store, detect_installs};
use crate::targets::{TargetCtx, resolve_targets};
use crate::vault::Vault;
use crate::vault::format::{SnapshotKind, SnapshotMeta};
use crate::vault::index::{SnapshotEntry, VaultIndex};
use crate::vault::read::read_meta;
use crate::vault::retention::RetentionPolicy;
use crate::vault::write::RealHooks;

/// A base às vezes renomeia um jogo e deixa o nome antigo como alias: os
/// backups feitos com o nome antigo passam a contar para o nome novo.
fn remap_aliases(index: &mut VaultIndex, manifest: Option<&Manifest>) {
    let Some(m) = manifest else { return };
    for e in &mut index.entries {
        let Some(title) = e.game_id.strip_prefix("pc:") else { continue };
        if let Some((canonical, g)) = m.entry(title) {
            if canonical != title && g.alias.is_none() {
                e.game_id = format!("pc:{canonical}");
            }
        }
    }
}

pub struct EngineConfig {
    pub data_dir: PathBuf,
    pub vault_dir: PathBuf,
    pub include_config: bool,
}

/// Para onde vai cada raiz de um backup, se ele for restaurado agora.
#[derive(Clone, Debug, PartialEq)]
pub struct RestoreTarget {
    pub alias: String,
    pub template: String,
    pub path: PathBuf,
}

/// O que a confirmação de restaurar mostra, sem mexer em nada.
#[derive(Clone, Debug)]
pub struct RestorePreview {
    pub entry: SnapshotEntry,
    pub targets: Vec<RestoreTarget>,
    pub file_count: usize,
    pub total_size: u64,
    pub running: bool,
}

#[derive(Clone, Debug)]
pub struct VaultSummary {
    pub path: PathBuf,
    pub available: bool,
    pub versions: usize,
    /// Tamanho dos `.zip` no disco.
    pub zip_bytes: u64,
    pub disk_total: Option<u64>,
    pub disk_free: Option<u64>,
}

pub struct Engine {
    cfg: EngineConfig,
    sys: SystemPaths,
    reg: Box<dyn RegistryReader + Send>,
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
    pub fn open(cfg: EngineConfig, sys: SystemPaths, reg: Box<dyn RegistryReader + Send>) -> Result<Engine> {
        let store = ManifestStore::new(cfg.data_dir.join("base"));
        // Base corrompida não impede o app de abrir: só fica sem jogos de PC.
        let manifest = store.load().ok().flatten();
        let local = LocalState::load(&cfg.data_dir.join("jogos.json"))?;
        let vault = Vault::open(&cfg.vault_dir)?;
        let machine = MachineIdentity::detect(&*reg);
        vault.cleanup_partials(&machine, Utc::now())?;
        let index_path = cfg.data_dir.join("indice").join(format!("{}.json", vault.id));
        let mut index = VaultIndex::load(&index_path, &vault.id);
        index.refresh(&vault)?;
        remap_aliases(&mut index, manifest.as_ref());
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
            remap_aliases(&mut self.index, self.manifest.as_ref());
            self.index.save(&self.index_path())?;
        }
        Ok(out)
    }

    /// Definições de jogos à mão que estão no cofre e ainda não existem neste PC.
    fn import_custom_from_vault(&mut self) -> Result<()> {
        let Ok(dirs) = std::fs::read_dir(&self.vault.root) else { return Ok(()) };
        let mut changed = false;
        for dir in dirs.flatten().filter(|d| d.path().is_dir()) {
            if let Some(def) = Vault::read_descriptor(&dir.path()).and_then(|d| d.custom) {
                // Removido ou ignorado neste PC: não volta do cofre.
                let ignored = self.local.settings.get(&def.id).is_some_and(|s| s.ignored);
                if !ignored && !self.local.custom_games.iter().any(|g| g.id == def.id) {
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

    /// Um jogo que só existe no cofre, montado a partir do backup mais novo dele.
    fn vault_only_game(&self, game_id: &str) -> Option<Game> {
        let settings = self.local.settings.get(game_id);
        if settings.is_some_and(|s| s.ignored) {
            return None;
        }
        let newest = self.index.for_game(game_id).into_iter().next()?;
        let meta = read_meta(&newest.zip_path(&self.vault)).ok()?;
        let desc = Vault::read_descriptor(&self.vault.root.join(&newest.dir_name));
        Some(Game {
            id: game_id.to_string(),
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
            // Sem isto, um jogo só-no-cofre aberto não seria percebido na restauração.
            process_names: desc.as_ref().map(|d| d.process_names.clone()).unwrap_or_default(),
            custom: desc.and_then(|d| d.custom),
            warnings: Vec::new(),
            vault_only: true,
        })
    }

    /// Jogos com backup no cofre que não foram encontrados neste PC.
    fn vault_only_games(&self, found: &[Game]) -> Vec<Game> {
        let mut seen: HashSet<&str> = found.iter().map(|g| g.id.as_str()).collect();
        let mut out = Vec::new();
        for e in &self.index.entries {
            if seen.insert(e.game_id.as_str()) {
                out.extend(self.vault_only_game(&e.game_id));
            }
        }
        out
    }

    /// Escaneia o PC e devolve cada jogo com o status em relação ao cofre.
    pub fn scan(&mut self) -> Result<Vec<(Game, GameStatus)>> {
        self.index.refresh(&self.vault)?;
        remap_aliases(&mut self.index, self.manifest.as_ref());
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
        for g in &mut games {
            self.sem_o_app(g);
        }
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

    /// O jogo como está no disco agora. O escaneamento guardado só diz quais
    /// jogos existem: os arquivos podem ter mudado depois dele (horas de jogo).
    fn refresh_game(&mut self, game_id: &str) -> Result<Game> {
        if !self.games.iter().any(|g| g.id == game_id) {
            return Err(Error::NotFound(format!("jogo {game_id} (escaneie antes)")));
        }
        let catalog = if game_id.starts_with("cat:") {
            find_catalog_games(&Catalog::builtin(), &self.sys, &self.local.user_roots)
        } else {
            Vec::new()
        };
        let input = ScanInput {
            sys: &self.sys,
            manifest: self.manifest.as_ref(),
            installs: &self.installs,
            catalog: &catalog,
            local: &self.local,
            include_config: self.cfg.include_config,
        };
        let mut game = scan_game(&input, game_id)
            .or_else(|| self.vault_only_game(game_id))
            .ok_or_else(|| Error::NotFound(format!("jogo {game_id}")))?;
        self.sem_o_app(&mut game);
        if let Some(slot) = self.games.iter_mut().find(|g| g.id == game_id) {
            *slot = game.clone();
        }
        Ok(game)
    }

    pub fn backup(&mut self, game_id: &str, kind: SnapshotKind, label: Option<&str>) -> Result<BackupOutcome> {
        let game = self.refresh_game(game_id)?;
        // A retenção do jogo, se ele tiver uma; senão a geral.
        let retention = self.game_settings(game_id).retention.unwrap_or_else(|| self.retention.clone());
        let result = {
            let mut ctx = BackupCtx {
                vault: &self.vault,
                index: &mut self.index,
                hashes: &mut self.hashes,
                machine: &self.machine,
                hooks: &RealHooks,
                now: Local::now(),
                app_version: env!("CARGO_PKG_VERSION"),
                retention: &retention,
            };
            backup_game(&mut ctx, &game, kind, label)
        };
        self.persist()?;
        result
    }

    /// Retenção geral dos automáticos (a de cada jogo, se houver, vale mais).
    pub fn set_retention(&mut self, policy: RetentionPolicy) {
        self.retention = policy;
    }

    pub fn retention(&self) -> &RetentionPolicy {
        &self.retention
    }

    /// Fixa (ou desafixa) um backup automático, de qualquer PC: fixado nunca
    /// sai na limpeza automática. Só automáticos e fixados mudam.
    pub fn pin_snapshot(&mut self, game_id: &str, file_name: &str, pinned: bool) -> Result<SnapshotEntry> {
        self.vault.ensure_available()?;
        let e = self.find_entry(game_id, file_name)?;
        let (new_name, kind) = match (pinned, e.kind) {
            (true, SnapshotKind::Auto) => (self.vault.pin(&e.dir_name, &e.file_name)?, SnapshotKind::Pinned),
            (false, SnapshotKind::Pinned) => (self.vault.unpin(&e.dir_name, &e.file_name)?, SnapshotKind::Auto),
            (true, SnapshotKind::Pinned) | (false, SnapshotKind::Auto) => return Ok(e),
            _ => return Err(Error::NotFound(format!("backup automático {file_name}"))),
        };
        self.index.remove(&e.dir_name, &e.file_name);
        let entry = SnapshotEntry { file_name: new_name, kind, ..e };
        self.index.insert(entry.clone());
        self.persist()?;
        Ok(entry)
    }

    /// Apaga um backup do cofre (de qualquer PC), a pedido do usuário.
    pub fn delete_snapshot(&mut self, game_id: &str, file_name: &str) -> Result<()> {
        self.vault.ensure_available()?;
        let e = self.find_entry(game_id, file_name)?;
        self.vault.delete_snapshot(&e.dir_name, &e.file_name)?;
        self.index.remove(&e.dir_name, &e.file_name);
        self.persist()
    }

    pub fn history(&self, game_id: &str) -> Vec<SnapshotEntry> {
        self.index.for_game(game_id).into_iter().cloned().collect()
    }

    fn find_entry(&self, game_id: &str, file_name: &str) -> Result<SnapshotEntry> {
        self.history(game_id)
            .into_iter()
            .find(|e| e.file_name == file_name)
            .ok_or_else(|| Error::NotFound(format!("backup {file_name}")))
    }

    /// Metadados de um backup do jogo (lista de arquivos, raízes…).
    pub fn snapshot_meta(&self, game_id: &str, file_name: &str) -> Result<SnapshotMeta> {
        read_meta(&self.find_entry(game_id, file_name)?.zip_path(&self.vault))
    }

    /// Jogos cujos arquivos de save mudaram desde o último escaneamento. Só
    /// lista as pastas (tamanho e data), sem calcular hashes: é a varredura leve
    /// que cobre eventos de arquivo perdidos.
    pub fn changed_games(&self) -> Vec<String> {
        let catalog = find_catalog_games(&Catalog::builtin(), &self.sys, &self.local.user_roots);
        let input = ScanInput {
            sys: &self.sys,
            manifest: self.manifest.as_ref(),
            installs: &self.installs,
            catalog: &catalog,
            local: &self.local,
            include_config: self.cfg.include_config,
        };
        let key = |g: &Game| {
            let mut v: Vec<(usize, String, u64, DateTime<Utc>)> =
                g.files.iter().map(|f| (f.root, f.rel_path.to_lowercase(), f.size, f.modified)).collect();
            v.sort();
            v
        };
        let agora = |id: &str| {
            scan_game(&input, id).map(|mut g| {
                self.sem_o_app(&mut g);
                key(&g)
            })
        };
        self.games
            .iter()
            .filter(|g| !g.vault_only)
            .filter(|g| agora(&g.id).unwrap_or_default() != key(g))
            .map(|g| g.id.clone())
            .collect()
    }

    /// Jogos com alguma pasta de save que contém `path` (sem diferença de maiúsculas).
    pub fn games_for_path(&self, path: &Path) -> Vec<String> {
        if self.do_app(path) {
            return Vec::new();
        }
        let p = slash(path).to_lowercase();
        self.games
            .iter()
            .filter(|g| {
                g.save_roots.iter().any(|r| {
                    let d = slash(&r.path).to_lowercase();
                    !d.is_empty() && (p == d || p.starts_with(&format!("{d}/")))
                })
            })
            .map(|g| g.id.clone())
            .collect()
    }

    /// O cofre e a pasta de dados do app nunca fazem parte de um jogo (§7.4: o
    /// próprio cofre é ruído). Dentro de uma pasta de save, cada backup entraria
    /// no próximo e o vigia faria um backup atrás do outro.
    fn do_app(&self, path: &Path) -> bool {
        let p = slash(path).to_lowercase();
        [&self.cfg.vault_dir, &self.cfg.data_dir].iter().any(|d| {
            let d = slash(d).to_lowercase();
            !d.is_empty() && (p == d || p.starts_with(&format!("{d}/")))
        })
    }

    fn sem_o_app(&self, g: &mut Game) {
        g.files.retain(|f| !self.do_app(&f.abs_path));
    }

    /// Jogos do último escaneamento.
    pub fn games(&self) -> &[Game] {
        &self.games
    }

    /// Reescaneia só este jogo no disco e calcula o status dele.
    pub fn rescan_game(&mut self, game_id: &str) -> Result<(Game, GameStatus)> {
        let game = self.refresh_game(game_id)?;
        let fp = fingerprint_of_game(&game, &mut self.hashes).unwrap_or_default();
        let status = status_of(&game, &fp, &self.index);
        self.persist()?;
        Ok((game, status))
    }

    pub fn game_settings(&self, game_id: &str) -> LocalGameSettings {
        self.local.settings.get(game_id).cloned().unwrap_or_default()
    }

    /// Muda os ajustes deste PC para um jogo e grava `jogos.json`.
    pub fn update_game_settings(&mut self, game_id: &str, f: impl FnOnce(&mut LocalGameSettings)) -> Result<()> {
        let s = self.local.settings.entry(game_id.to_string()).or_default();
        f(s);
        if *s == LocalGameSettings::default() {
            self.local.settings.remove(game_id);
        }
        self.local.save(&self.local_path())
    }

    /// Tira a definição deste PC e marca o jogo como ignorado, para ele não
    /// voltar do cofre. Os backups continuam no cofre.
    pub fn remove_custom_game(&mut self, game_id: &str) -> Result<()> {
        let before = self.local.custom_games.len();
        self.local.custom_games.retain(|g| g.id != game_id);
        if self.local.custom_games.len() == before {
            return Err(Error::NotFound(format!("jogo adicionado à mão {game_id}")));
        }
        self.local.settings.entry(game_id.to_string()).or_default().ignored = true;
        self.games.retain(|g| g.id != game_id);
        self.local.save(&self.local_path())
    }

    pub fn ignored_ids(&self) -> Vec<String> {
        self.local.settings.iter().filter(|(_, s)| s.ignored).map(|(id, _)| id.clone()).collect()
    }

    pub fn has_manifest(&self) -> bool {
        self.manifest.is_some()
    }

    pub fn include_config(&self) -> bool {
        self.cfg.include_config
    }

    /// Vale a partir do próximo escaneamento.
    pub fn set_include_config(&mut self, on: bool) {
        self.cfg.include_config = on;
    }

    pub fn vault_dir(&self) -> &Path {
        &self.vault.root
    }

    pub fn user_roots(&self) -> &[PathBuf] {
        &self.local.user_roots
    }

    pub fn vault_summary(&self) -> VaultSummary {
        let zip_bytes = self
            .index
            .entries
            .iter()
            .filter_map(|e| std::fs::metadata(e.zip_path(&self.vault)).ok())
            .map(|m| m.len())
            .sum();
        VaultSummary {
            path: self.vault.root.clone(),
            available: self.vault.ensure_available().is_ok(),
            versions: self.index.entries.len(),
            zip_bytes,
            disk_total: fs4::total_space(&self.vault.root).ok(),
            disk_free: fs4::available_space(&self.vault.root).ok(),
        }
    }

    /// Instalações, contas e pastas do jogo neste PC, para resolver os destinos.
    fn target_parts(&self, game: &Game, settings: &LocalGameSettings) -> (Vec<InstallCandidate>, Vec<String>, Vec<PathBuf>) {
        let (installs, user_ids) = self.target_installs(game);
        let mut game_dirs: Vec<PathBuf> = Vec::new();
        for d in game.install_dirs.iter().chain(&game.game_dir).chain(&settings.game_dir) {
            if !game_dirs.iter().any(|x| slash(x).eq_ignore_ascii_case(&slash(d))) {
                game_dirs.push(d.clone());
            }
        }
        (installs, user_ids, game_dirs)
    }

    /// Para onde cada raiz iria e o que seria restaurado, sem mexer em nada.
    /// `NeedsTarget` sobe como erro, igual à restauração.
    pub fn restore_preview(
        &mut self,
        game_id: &str,
        file_name: &str,
        overrides: &BTreeMap<String, PathBuf>,
        probe: &dyn ProcessProbe,
    ) -> Result<RestorePreview> {
        let game = self.refresh_game(game_id)?;
        self.vault.ensure_available()?;
        let entry = self.find_entry(game_id, file_name)?;
        let meta = read_meta(&entry.zip_path(&self.vault))?;
        let settings = self.game_settings(game_id);
        let (installs, store_user_ids, game_dirs) = self.target_parts(&game, &settings);
        let ctx = TargetCtx { sys: &self.sys, installs, store_user_ids, game_dirs, remembered: &settings.target_choices };
        let targets = resolve_targets(&meta, &ctx, overrides)?
            .into_iter()
            .map(|(alias, path)| {
                let template = meta.roots.iter().find(|r| r.alias == alias).map(|r| r.template.clone()).unwrap_or_default();
                RestoreTarget { alias, template, path }
            })
            .collect();
        Ok(RestorePreview {
            entry,
            targets,
            file_count: meta.files.len(),
            total_size: meta.files.iter().map(|f| f.size).sum(),
            running: is_game_running(&game, probe),
        })
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
        let game = self.refresh_game(game_id)?;
        let entry = self.find_entry(game_id, file_name)?;
        let others: Vec<Game> = self.games.iter().filter(|g| g.id != game_id).cloned().collect();
        let settings = self.game_settings(game_id);
        let (installs, store_user_ids, game_dirs) = self.target_parts(&game, &settings);
        let targets =
            TargetCtx { sys: &self.sys, installs, store_user_ids, game_dirs, remembered: &settings.target_choices };
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
        let report = result?;
        if opts.remember && !opts.overrides.is_empty() {
            let meta = read_meta(&entry.zip_path(&self.vault))?;
            let s = self.local.settings.entry(game_id.to_string()).or_default();
            for (alias, path) in &opts.overrides {
                if let Some(r) = meta.roots.iter().find(|r| &r.alias == alias) {
                    s.target_choices.insert(r.template.clone(), path.clone());
                }
            }
            self.local.save(&self.local_path())?;
        }
        Ok(report)
    }

    /// Com `game_dir`, as pastas de save dentro dela viram `<gameDir>/…` e a
    /// pasta do jogo fica guardada nos ajustes deste PC.
    pub fn add_custom_game(
        &mut self,
        name: &str,
        save_dirs: &[PathBuf],
        processes: &[String],
        game_dir: Option<&Path>,
    ) -> Result<CustomGame> {
        let g = CustomGame::new(name, save_dirs, processes, game_dir, &self.sys);
        self.local.custom_games.push(g.clone());
        if let Some(gd) = game_dir {
            self.local.settings.entry(g.id.clone()).or_default().game_dir = Some(gd.to_path_buf());
        }
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
