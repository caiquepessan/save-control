//! O serviço do app: decide a fase (primeiro uso, cofre sumido…), guarda a
//! `Engine` e faz as ações que a interface pede. Não depende de Tauri.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use chrono::Utc;

use savecontrol_core::backup::BackupOutcome;
use savecontrol_core::catalog::Catalog;
use savecontrol_core::engine::{Engine, EngineConfig};
use savecontrol_core::env::SystemPaths;
use savecontrol_core::fsutil::write_atomic;
use savecontrol_core::manifest::store::{Fetcher, UpdateOutcome};
use savecontrol_core::model::Game;
use savecontrol_core::process::{ProcessProbe, RunningProcess, is_game_running};
use savecontrol_core::registry::RegistryReader;
use savecontrol_core::restore::RestoreOptions;
use savecontrol_core::status::GameStatus;
use savecontrol_core::vault::Vault;
use savecontrol_core::vault::format::SnapshotKind;
use savecontrol_core::vault::index::SnapshotEntry;

use crate::config::{AppConfig, WindowState, load_config, save_config};
use crate::covers::CoverStore;
use crate::dto::*;
use crate::paths::AppPaths;
use crate::texts;
use crate::view::{ViewCtx, date_str, game_view, path_str, snapshot_view, vault_view};

/// Liga enquanto há escaneamento, backup ou restauração rodando. A janela não
/// fecha enquanto estiver ligado. Conta guardas aninhadas.
#[derive(Clone, Default)]
pub struct BusyFlag(Arc<AtomicUsize>);

impl BusyFlag {
    pub fn is_busy(&self) -> bool {
        self.0.load(Ordering::SeqCst) > 0
    }

    pub fn guard(&self) -> BusyGuard {
        self.0.fetch_add(1, Ordering::SeqCst);
        BusyGuard(self.0.clone())
    }
}

/// Desliga o `BusyFlag` ao sair de escopo, inclusive em erro ou pânico.
pub struct BusyGuard(Arc<AtomicUsize>);

impl Drop for BusyGuard {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

pub type RegistryFactory = Box<dyn Fn() -> Box<dyn RegistryReader + Send> + Send>;

type Res<T> = std::result::Result<T, ErrorView>;

/// Uma foto dos processos abertos, para não listar tudo de novo a cada jogo.
struct ProcessSnapshot(Vec<RunningProcess>);

impl ProcessProbe for ProcessSnapshot {
    fn running(&self) -> Vec<RunningProcess> {
        self.0.clone()
    }
}

fn to_paths(m: &BTreeMap<String, String>) -> BTreeMap<String, PathBuf> {
    m.iter().map(|(k, v)| (k.clone(), PathBuf::from(v))).collect()
}

fn erro(message: impl Into<String>) -> ErrorView {
    ErrorView::new(ErrorKind::Erro, message)
}

pub struct AppService {
    paths: AppPaths,
    sys: SystemPaths,
    make_reg: RegistryFactory,
    busy: BusyFlag,
    config: AppConfig,
    warnings: Vec<String>,
    engine: Option<Engine>,
    open_error: Option<String>,
    statuses: BTreeMap<String, GameStatus>,
    /// Já escaneou nesta sessão (senão a biblioteca vem do cache).
    scanned: bool,
    catalog: Catalog,
    covers: CoverStore,
}

impl AppService {
    /// Só lê o `config.json`; a `Engine` abre na primeira ação que precisa dela.
    pub fn new(paths: AppPaths, sys: SystemPaths, make_reg: RegistryFactory, busy: BusyFlag) -> AppService {
        let loaded = load_config(&paths.config_file());
        let mut warnings = Vec::new();
        if paths.fallback {
            warnings.push(texts::DATA_FALLBACK.to_string());
        }
        warnings.extend(loaded.warning);
        let covers = CoverStore { dir: paths.covers_dir() };
        AppService {
            paths,
            sys,
            make_reg,
            busy,
            config: loaded.config,
            warnings,
            engine: None,
            open_error: None,
            statuses: BTreeMap::new(),
            scanned: false,
            catalog: Catalog::builtin(),
            covers,
        }
    }

    pub fn paths(&self) -> &AppPaths {
        &self.paths
    }

    pub fn busy(&self) -> BusyFlag {
        self.busy.clone()
    }

    pub fn cover_store(&self) -> CoverStore {
        self.covers.clone()
    }

    pub fn window(&self) -> Option<WindowState> {
        self.config.window
    }

    pub fn steam_covers_enabled(&self) -> bool {
        self.config.steam_covers
    }

    fn vault_dir(&self) -> PathBuf {
        self.config.vault_dir.clone().unwrap_or_else(|| self.paths.default_vault.clone())
    }

    fn save_config(&self) -> Res<()> {
        save_config(&self.paths.config_file(), &self.config).map_err(ErrorView::from)
    }

    fn reset_engine(&mut self) {
        self.engine = None;
        self.open_error = None;
        self.statuses.clear();
        self.scanned = false;
    }

    /// Decide a fase. Abre a `Engine` só quando o cofre existe: um cofre
    /// sumido (HD desligado) nunca é recriado aqui.
    fn phase(&mut self) -> AppPhase {
        if !self.config.first_run_done {
            return AppPhase::PrimeiroUso;
        }
        if self.engine.is_some() {
            return AppPhase::Pronto;
        }
        let vault = self.vault_dir();
        if !vault.join("cofre.json").is_file() {
            return AppPhase::CofreSumiu;
        }
        let cfg = EngineConfig {
            data_dir: self.paths.data_dir.clone(),
            vault_dir: vault,
            include_config: self.config.include_config,
        };
        match Engine::open(cfg, self.sys.clone(), (self.make_reg)()) {
            Ok(e) => {
                self.engine = Some(e);
                self.open_error = None;
                AppPhase::Pronto
            }
            Err(e) => {
                self.open_error = Some(e.to_string());
                AppPhase::Falhou
            }
        }
    }

    pub fn state(&mut self) -> AppStateView {
        let phase = self.phase();
        AppStateView {
            phase,
            vault_dir: path_str(&self.vault_dir()),
            data_dir: path_str(&self.paths.data_dir),
            default_vault: path_str(&self.paths.default_vault),
            warnings: self.warnings.clone(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            error: if phase == AppPhase::Falhou { self.open_error.clone() } else { None },
        }
    }

    fn engine(&mut self) -> Res<&mut Engine> {
        match self.phase() {
            AppPhase::Pronto => Ok(self.engine.as_mut().expect("Engine aberta na fase Pronto")),
            AppPhase::PrimeiroUso => Err(erro("Conclua o primeiro uso antes.")),
            AppPhase::CofreSumiu => Err(ErrorView::new(
                ErrorKind::CofreIndisponivel,
                format!("O cofre em {} não está acessível", path_str(&self.vault_dir())),
            )),
            AppPhase::Falhou => Err(erro(self.open_error.clone().unwrap_or_default())),
        }
    }

    fn ctx<'a>(&'a self, engine: &'a Engine) -> ViewCtx<'a> {
        ViewCtx { sys: &self.sys, catalog: &self.catalog, machine_id: &engine.machine().id, covers: &self.covers }
    }

    fn view_of(&self, engine: &Engine, game: &Game) -> GameView {
        let status = self.statuses.get(&game.id).copied().unwrap_or(GameStatus::NoSave);
        game_view(&self.ctx(engine), game, status, &engine.history(&game.id), &engine.game_settings(&game.id))
    }

    fn snapshot(&self, e: &SnapshotEntry) -> SnapshotView {
        match &self.engine {
            Some(engine) => snapshot_view(&self.ctx(engine), e),
            None => unreachable!("snapshot sem Engine aberta"),
        }
    }

    fn library_view(&self) -> LibraryView {
        let engine = self.engine.as_ref().expect("Engine aberta");
        let games = engine
            .games()
            .iter()
            .filter(|g| !engine.game_settings(&g.id).ignored)
            .map(|g| self.view_of(engine, g))
            .collect();
        LibraryView {
            games,
            vault: vault_view(&engine.vault_summary()),
            scanned_at: self.config.last_scan.as_ref().map(date_str),
            manifest_loaded: engine.has_manifest(),
        }
    }

    /// O último escaneamento; antes do primeiro escaneamento da sessão, o
    /// resultado guardado em disco (para a biblioteca abrir na hora).
    pub fn library(&mut self) -> Res<LibraryView> {
        self.engine()?;
        if self.scanned {
            return Ok(self.library_view());
        }
        let engine = self.engine.as_ref().expect("Engine aberta");
        let vault = vault_view(&engine.vault_summary());
        let cached = std::fs::read_to_string(self.paths.library_cache())
            .ok()
            .and_then(|t| serde_json::from_str::<LibraryView>(&t).ok());
        Ok(match cached {
            Some(mut lib) => {
                lib.vault = vault;
                lib
            }
            None => LibraryView { games: Vec::new(), vault, scanned_at: None, manifest_loaded: engine.has_manifest() },
        })
    }

    pub fn scan(&mut self) -> Res<LibraryView> {
        let _busy = self.busy.guard();
        let found = self.engine()?.scan()?;
        self.statuses = found.into_iter().map(|(g, s)| (g.id, s)).collect();
        self.scanned = true;
        self.config.last_scan = Some(Utc::now());
        let _ = self.save_config();
        let lib = self.library_view();
        if let Ok(json) = serde_json::to_vec(&lib) {
            let _ = write_atomic(&self.paths.library_cache(), &json);
        }
        Ok(lib)
    }

    /// Reescaneia só este jogo e guarda o status novo (sem escanear tudo).
    fn refresh_status(&mut self, id: &str) {
        if let Some(engine) = self.engine.as_mut() {
            if let Ok((_, status)) = engine.rescan_game(id) {
                self.statuses.insert(id.to_string(), status);
            }
        }
    }

    fn backup_inner(&mut self, id: &str, label: Option<&str>) -> Res<BackupResultView> {
        let outcome = self.engine()?.backup(id, SnapshotKind::Manual, label)?;
        let (outcome, entry) = match outcome {
            BackupOutcome::Created(e) => (BackupOutcomeView::Feito, Some(e)),
            BackupOutcome::Unchanged(e) => (BackupOutcomeView::SemMudanca, Some(e)),
            BackupOutcome::NothingToBackup => (BackupOutcomeView::SemArquivos, None),
        };
        self.refresh_status(id);
        Ok(BackupResultView { outcome, snapshot: entry.map(|e| self.snapshot(&e)) })
    }

    /// Backup manual, com nome opcional.
    pub fn backup_game(&mut self, id: &str, label: Option<String>) -> Res<BackupResultView> {
        let _busy = self.busy.guard();
        let label = label.map(|l| l.trim().to_string()).filter(|l| !l.is_empty());
        self.backup_inner(id, label.as_deref())
    }

    /// Jogos que precisam de backup: save novo ou sem backup nenhum.
    pub fn needs_backup_ids(&self) -> Vec<String> {
        let Some(engine) = &self.engine else { return Vec::new() };
        engine
            .games()
            .iter()
            .filter(|g| !g.vault_only && !engine.game_settings(&g.id).ignored)
            .filter(|g| matches!(self.statuses.get(&g.id), Some(GameStatus::NewSave | GameStatus::NoBackup)))
            .map(|g| g.id.clone())
            .collect()
    }

    /// Backup manual de vários jogos; uma falha não para os outros.
    pub fn backup_many(&mut self, ids: &[String]) -> BackupManyView {
        let _busy = self.busy.guard();
        let mut out = BackupManyView::default();
        for id in ids {
            match self.backup_inner(id, None) {
                Ok(r) if r.outcome == BackupOutcomeView::Feito => out.done.push(id.clone()),
                Ok(_) => out.unchanged.push(id.clone()),
                Err(e) => out.failed.push(BackupFailure { id: id.clone(), title: self.title_for(id), message: e.message }),
            }
        }
        out
    }

    pub fn history(&mut self, id: &str) -> Res<Vec<SnapshotView>> {
        let entries = self.engine()?.history(id);
        Ok(entries.iter().map(|e| self.snapshot(e)).collect())
    }

    pub fn snapshot_files(&mut self, id: &str, file: &str) -> Res<Vec<SnapshotFileView>> {
        let meta = self.engine()?.snapshot_meta(id, file)?;
        Ok(meta
            .files
            .iter()
            .map(|f| SnapshotFileView { root: f.root.clone(), rel_path: f.rel_path.clone(), size: f.size })
            .collect())
    }

    pub fn restore_preview(
        &mut self,
        id: &str,
        file: &str,
        targets: &BTreeMap<String, String>,
        probe: &dyn ProcessProbe,
    ) -> Res<RestorePreviewView> {
        let p = self.engine()?.restore_preview(id, file, &to_paths(targets), probe)?;
        Ok(RestorePreviewView {
            snapshot: self.snapshot(&p.entry),
            single_root: p.targets.len() == 1,
            targets: p
                .targets
                .iter()
                .map(|t| RestoreTargetView { alias: t.alias.clone(), template: t.template.clone(), path: path_str(&t.path) })
                .collect(),
            file_count: p.file_count,
            total_size: p.total_size,
            running: p.running,
        })
    }

    pub fn restore(&mut self, id: &str, file: &str, req: &RestoreRequest, probe: &dyn ProcessProbe) -> Res<RestoreResultView> {
        let _busy = self.busy.guard();
        let opts = RestoreOptions {
            overrides: to_paths(&req.targets),
            only: req.only_files.as_ref().map(|fs| fs.iter().map(|f| (f.root.clone(), f.rel_path.clone())).collect()),
            remember: req.remember,
        };
        let report = self.engine()?.restore(id, file, &opts, probe)?;
        self.refresh_status(id);
        Ok(RestoreResultView {
            written: report.written,
            removed: report.removed,
            safety: report.safety.as_ref().map(|s| self.snapshot(s)),
        })
    }

    /// Adiciona o jogo e escaneia, para devolver ele já com arquivos e status.
    pub fn add_game(&mut self, req: AddGameRequest) -> Res<GameView> {
        let name = req.name.trim().to_string();
        if name.is_empty() {
            return Err(erro("Dê um nome ao jogo."));
        }
        let dirs: Vec<PathBuf> = req.save_dirs.iter().filter(|d| !d.trim().is_empty()).map(PathBuf::from).collect();
        if dirs.is_empty() {
            return Err(erro("Escolha pelo menos uma pasta do save."));
        }
        let game_dir = req.game_dir.as_deref().filter(|d| !d.trim().is_empty()).map(PathBuf::from);
        let processes: Vec<String> =
            req.processes.iter().map(|p| p.trim().to_string()).filter(|p| !p.is_empty()).collect();
        let _busy = self.busy.guard();
        let g = self.engine()?.add_custom_game(&name, &dirs, &processes, game_dir.as_deref())?;
        if let Some(src) = req.cover_file.as_deref().filter(|c| !c.trim().is_empty()) {
            let dest = self.covers.import_custom(Path::new(src)).map_err(erro)?;
            self.engine()?.update_game_settings(&g.id, |s| s.cover = Some(dest))?;
        }
        let lib = self.scan()?;
        lib.games
            .into_iter()
            .find(|v| v.id == g.id)
            .ok_or_else(|| ErrorView::new(ErrorKind::NaoEncontrado, "O jogo novo não apareceu no escaneamento."))
    }

    /// Nome mostrado neste PC; `None` volta ao original.
    pub fn rename_game(&mut self, id: &str, title: Option<String>) -> Res<()> {
        let title = title.map(|t| t.trim().to_string()).filter(|t| !t.is_empty());
        self.engine()?.update_game_settings(id, |s| s.title = title)?;
        Ok(())
    }

    pub fn set_ignored(&mut self, id: &str, ignored: bool) -> Res<()> {
        self.engine()?.update_game_settings(id, |s| s.ignored = ignored)?;
        Ok(())
    }

    pub fn set_game_dir(&mut self, id: &str, dir: Option<PathBuf>) -> Res<()> {
        self.engine()?.update_game_settings(id, |s| s.game_dir = dir)?;
        self.refresh_status(id);
        Ok(())
    }

    /// Copia a imagem para a pasta de capas e usa neste jogo; `None` volta à capa automática.
    pub fn set_cover(&mut self, id: &str, file: Option<PathBuf>) -> Res<()> {
        let dest = match file {
            Some(src) => Some(self.covers.import_custom(&src).map_err(erro)?),
            None => None,
        };
        self.engine()?.update_game_settings(id, |s| s.cover = dest)?;
        Ok(())
    }

    pub fn forget_target(&mut self, id: &str, template: &str) -> Res<()> {
        self.engine()?.update_game_settings(id, |s| {
            s.target_choices.remove(template);
        })?;
        Ok(())
    }

    pub fn remove_custom_game(&mut self, id: &str) -> Res<()> {
        self.engine()?.remove_custom_game(id)?;
        self.statuses.remove(id);
        Ok(())
    }

    /// Título para mostrar mesmo de jogos que não estão na lista (ignorados, removidos).
    fn title_for(&self, id: &str) -> String {
        if let Some(engine) = &self.engine {
            if let Some(g) = engine.games().iter().find(|g| g.id == id) {
                return g.title.clone();
            }
            if let Some(e) = engine.history(id).first() {
                if let Ok(meta) = engine.snapshot_meta(id, &e.file_name) {
                    return meta.title;
                }
            }
        }
        if let Some(rest) = id.strip_prefix("cat:") {
            let entry = rest.split(':').next().unwrap_or(rest);
            if let Some(e) = self.catalog.entries.iter().find(|e| e.id == entry) {
                return e.name.clone();
            }
        }
        id.strip_prefix("pc:").unwrap_or(id).to_string()
    }

    pub fn settings(&mut self) -> Res<SettingsView> {
        let engine = self.engine()?;
        let user_roots = engine.user_roots().iter().map(|p| path_str(p)).collect();
        let manifest_loaded = engine.has_manifest();
        let ignored_ids = engine.ignored_ids();
        let ignored = ignored_ids.iter().map(|id| IgnoredGame { id: id.clone(), title: self.title_for(id) }).collect();
        Ok(SettingsView {
            vault_dir: path_str(&self.vault_dir()),
            user_roots,
            include_config: self.config.include_config,
            steam_covers: self.config.steam_covers,
            manifest_loaded,
            ignored,
            version: env!("CARGO_PKG_VERSION").to_string(),
        })
    }

    pub fn set_user_roots(&mut self, roots: Vec<PathBuf>) -> Res<()> {
        self.engine()?.set_user_roots(roots)?;
        Ok(())
    }

    pub fn set_include_config(&mut self, on: bool) -> Res<()> {
        self.config.include_config = on;
        self.save_config()?;
        if let Some(e) = self.engine.as_mut() {
            e.set_include_config(on);
        }
        Ok(())
    }

    pub fn set_steam_covers(&mut self, on: bool) -> Res<()> {
        self.config.steam_covers = on;
        self.save_config()
    }

    /// Passa a usar o cofre de `dir`. Sem cofre lá, só cria quando `create`.
    pub fn change_vault(&mut self, dir: &Path, create: bool) -> Res<AppStateView> {
        if !dir.join("cofre.json").is_file() {
            if !create {
                return Err(ErrorView::new(ErrorKind::NaoEncontrado, "Não há um cofre nessa pasta."));
            }
            Vault::open_or_create(dir)?;
        }
        self.config.vault_dir = Some(dir.to_path_buf());
        self.save_config()?;
        self.reset_engine();
        Ok(self.state())
    }

    /// Só lê: existe um cofre em `dir`? Quantos backups ele tem?
    pub fn inspect_vault(&self, dir: &Path) -> VaultProbeView {
        let exists = dir.join("cofre.json").is_file();
        let mut versions = 0;
        if exists {
            for sub in std::fs::read_dir(dir).into_iter().flatten().flatten().filter(|s| s.path().is_dir()) {
                versions += std::fs::read_dir(sub.path())
                    .into_iter()
                    .flatten()
                    .flatten()
                    .filter(|f| SnapshotKind::from_file_name(&f.file_name().to_string_lossy()).is_some())
                    .count();
            }
        }
        VaultProbeView { exists, versions }
    }

    pub fn retry_vault(&mut self) -> AppStateView {
        self.reset_engine();
        self.state()
    }

    /// Sem internet não é erro: a interface mostra o aviso e segue.
    pub fn update_manifest(&mut self, fetcher: &dyn Fetcher) -> Res<ManifestUpdateView> {
        let _busy = self.busy.guard();
        Ok(match self.engine()?.update_manifest(fetcher) {
            Ok(UpdateOutcome::Updated) => ManifestUpdateView { outcome: ManifestOutcome::Atualizada, message: None },
            Ok(_) => ManifestUpdateView { outcome: ManifestOutcome::SemNovidade, message: None },
            Err(e) => ManifestUpdateView { outcome: ManifestOutcome::Falhou, message: Some(e.to_string()) },
        })
    }

    pub fn finish_first_run(&mut self, vault_dir: &Path) -> Res<AppStateView> {
        Vault::open_or_create(vault_dir)?;
        self.config.vault_dir = Some(vault_dir.to_path_buf());
        self.config.first_run_done = true;
        self.save_config()?;
        self.reset_engine();
        Ok(self.state())
    }

    pub fn running_ids(&self, probe: &dyn ProcessProbe) -> Vec<String> {
        let Some(engine) = &self.engine else { return Vec::new() };
        let snapshot = ProcessSnapshot(probe.running());
        engine.games().iter().filter(|g| is_game_running(g, &snapshot)).map(|g| g.id.clone()).collect()
    }

    /// Jogos da Steam cuja capa ainda não foi baixada (vazio com as capas desligadas).
    pub fn steam_covers_to_fetch(&self) -> Vec<(String, u32)> {
        if !self.config.steam_covers {
            return Vec::new();
        }
        let Some(engine) = &self.engine else { return Vec::new() };
        engine
            .games()
            .iter()
            .filter_map(|g| g.steam_app_id.map(|a| (g.id.clone(), a)))
            .filter(|(_, a)| !self.covers.steam_path(*a).is_file())
            .collect()
    }

    pub fn save_window(&mut self, w: WindowState) -> Res<()> {
        self.config.window = Some(w);
        self.save_config()
    }
}
