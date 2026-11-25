//! O serviço do app: decide a fase (primeiro uso, cofre sumido…), guarda a
//! `Engine` e faz as ações que a interface pede. Não depende de Tauri.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use chrono::Utc;

use savecontrol_core::auto::{AutoRules, Running};
use savecontrol_core::backup::BackupOutcome;
use savecontrol_core::catalog::Catalog;
use savecontrol_core::engine::{Engine, EngineConfig};
use savecontrol_core::env::SystemPaths;
use savecontrol_core::fsutil::write_atomic;
use savecontrol_core::manifest::store::{Fetcher, UpdateOutcome};
use savecontrol_core::model::Game;
use savecontrol_core::process::{ProcessProbe, RunningProcess, can_tell_running, is_game_running};
use savecontrol_core::registry::RegistryReader;
use savecontrol_core::restore::RestoreOptions;
use savecontrol_core::status::GameStatus;
use savecontrol_core::vault::Vault;
use savecontrol_core::vault::format::SnapshotKind;
use savecontrol_core::vault::index::SnapshotEntry;
use savecontrol_core::vault::retention::RetentionPolicy;

use crate::activity::{ActivityEntry, ActivityKind, ActivityLog};
use crate::config::{AppConfig, AutoTimes, WindowState, load_config, save_config};
use crate::covers::{CoverChoice, CoverStore};
use crate::dto::*;
use crate::paths::{AppPaths, VAULT_DIR_NAME};
use crate::texts;
use crate::view::{ViewCtx, date_str, game_view, path_str, snapshot_view, vault_view};

/// Liga enquanto há backup ou restauração rodando (só eles mexem em save). O
/// app não sai enquanto estiver ligado. Conta guardas aninhadas.
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

    /// Espera o backup ou a restauração em andamento terminar ("Sair"). Devolve
    /// falso se ainda estiver ocupado depois do tempo limite.
    pub fn wait_idle(&self, timeout: Duration) -> bool {
        let limite = Instant::now() + timeout;
        while self.is_busy() {
            if Instant::now() >= limite {
                return false;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        true
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

/// Partes do caminho, sem diferença de maiúsculas (como o Windows compara).
fn path_key(p: &Path) -> Vec<String> {
    p.components().map(|c| c.as_os_str().to_string_lossy().to_lowercase()).collect()
}

fn same_path(a: &Path, b: &Path) -> bool {
    path_key(a) == path_key(b)
}

/// `dir` relativo a `base`, se ficar dentro dela.
fn relative_to(dir: &Path, base: &Path) -> Option<PathBuf> {
    let n = base.components().count();
    if path_key(dir).get(..n)? != path_key(base).as_slice() {
        return None;
    }
    let rest: PathBuf = dir.components().skip(n).collect();
    (!rest.as_os_str().is_empty()).then_some(rest)
}

fn is_empty_dir(d: &Path) -> bool {
    std::fs::read_dir(d).map(|mut it| it.next().is_none()).unwrap_or(false)
}

/// Onde fica (ou vai ficar) o cofre escolhido: a própria pasta se ela já é um
/// cofre, está vazia ou ainda não existe; senão a subpasta "cofre". Um cofre
/// novo nunca é misturado com outros arquivos: numa pasta de jogos, os backups
/// cairiam dentro da pasta de um jogo e sumiriam junto com ela.
fn vault_target(dir: &Path) -> Res<PathBuf> {
    let livre = |d: &Path| d.join("cofre.json").is_file() || !d.exists() || is_empty_dir(d);
    if livre(dir) {
        return Ok(dir.to_path_buf());
    }
    let sub = dir.join(VAULT_DIR_NAME);
    if livre(&sub) {
        return Ok(sub);
    }
    Err(erro("Essa pasta já tem outros arquivos. Escolha uma pasta vazia ou um cofre que já existe."))
}

/// A capa copiada para a pasta de capas fica guardada só pelo nome do arquivo
/// (a pasta de capas acompanha o app quando ele muda de lugar).
fn cover_name(dest: PathBuf) -> PathBuf {
    dest.file_name().map(PathBuf::from).unwrap_or(dest)
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
    /// O config.json existe mas não deu para ler: nada é gravado por cima dele.
    read_only: bool,
    activity: ActivityLog,
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
        let activity = ActivityLog::new(paths.activity_file());
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
            read_only: loaded.read_only,
            activity,
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

    /// A pasta do cofre. Na configuração, `None` é o cofre padrão e um caminho
    /// relativo é dentro da pasta do app (os dois acompanham o HD que muda de letra).
    fn vault_dir(&self) -> PathBuf {
        match &self.config.vault_dir {
            None => self.paths.default_vault.clone(),
            Some(p) if p.is_relative() => self.paths.exe_dir.join(p),
            Some(p) => p.clone(),
        }
    }

    /// Como guardar `dir` na configuração (o contrário de `vault_dir`).
    fn stored_vault(&self, dir: &Path) -> Option<PathBuf> {
        if same_path(dir, &self.paths.default_vault) {
            return None;
        }
        if !self.paths.fallback {
            if let Some(rel) = relative_to(dir, &self.paths.exe_dir) {
                return Some(rel);
            }
        }
        Some(dir.to_path_buf())
    }

    fn save_config(&self) -> Res<()> {
        if self.read_only {
            return Ok(());
        }
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
            Ok(mut e) => {
                e.set_retention(self.config.retention.clone());
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

    /// O cofre está lá agora (o HD pode ter sido desligado com o app aberto).
    fn vault_available(&self) -> bool {
        self.vault_dir().join("cofre.json").is_file()
    }

    pub fn scan(&mut self) -> Res<LibraryView> {
        self.engine()?;
        // Com o cofre fora, escanear esvaziaria o índice e todo jogo viraria
        // "Sem backup": mostra o último resultado, com o cofre indisponível.
        if !self.vault_available() {
            return self.library();
        }
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

    /// Faz o backup, atualiza o status e registra na Atividade (backup feito ou erro).
    fn backup_kind(
        &mut self,
        id: &str,
        kind: SnapshotKind,
        label: Option<&str>,
    ) -> Res<(BackupOutcomeView, Option<SnapshotEntry>)> {
        let outcome = match self.engine()?.backup(id, kind, label) {
            Ok(o) => o,
            Err(e) => {
                let e: ErrorView = e.into();
                self.record_error(id, &e);
                return Err(e);
            }
        };
        let (outcome, entry) = match outcome {
            BackupOutcome::Created(e) => (BackupOutcomeView::Feito, Some(e)),
            BackupOutcome::Unchanged(e) => (BackupOutcomeView::SemMudanca, Some(e)),
            BackupOutcome::NothingToBackup => (BackupOutcomeView::SemArquivos, None),
        };
        self.refresh_status(id);
        if let (BackupOutcomeView::Feito, Some(e)) = (outcome, &entry) {
            let activity = if kind == SnapshotKind::Auto { ActivityKind::BackupAuto } else { ActivityKind::BackupManual };
            let arquivos = texts::arquivos(e.file_count, e.total_size);
            let detail = match &e.label {
                Some(nome) => format!("{nome} · {arquivos}"),
                None => arquivos,
            };
            self.record(activity, Some(id), detail);
        }
        Ok((outcome, entry))
    }

    fn backup_inner(&mut self, id: &str, label: Option<&str>) -> Res<BackupResultView> {
        let (outcome, entry) = self.backup_kind(id, SnapshotKind::Manual, label)?;
        Ok(BackupResultView { outcome, snapshot: entry.map(|e| self.snapshot(&e)) })
    }

    /// Uma linha na Atividade (falhar ao gravar nunca atrapalha a ação).
    fn record(&self, kind: ActivityKind, game_id: Option<&str>, detail: String) {
        let title = game_id.map(|id| self.title_for(id));
        let entry = ActivityEntry { at: Utc::now(), kind, game_id: game_id.map(str::to_string), title, detail };
        let _ = self.activity.append(&entry);
    }

    /// Erros de verdade vão para a Atividade; "feche o jogo" e "escolha o
    /// destino" são só passos do fluxo.
    fn record_error(&self, id: &str, e: &ErrorView) {
        if !matches!(e.kind, ErrorKind::JogoAberto | ErrorKind::PrecisaDestino) {
            self.record(ActivityKind::Error, Some(id), e.message.clone());
        }
    }

    /// Backup automático (o trabalhador chama quando vence o prazo do §9).
    pub fn auto_backup(&mut self, id: &str) -> Res<AutoBackupView> {
        let _busy = self.busy.guard();
        let (outcome, entry) = self.backup_kind(id, SnapshotKind::Auto, None)?;
        let (file_count, total_size) = entry.as_ref().map_or((0, 0), |e| (e.file_count, e.total_size));
        Ok(AutoBackupView {
            id: id.to_string(),
            title: self.title_for(id),
            outcome,
            file_count,
            total_size,
            cover: self.cover_file(id),
        })
    }

    /// Arquivo da capa (própria ou da Steam), para a notificação.
    fn cover_file(&self, id: &str) -> Option<String> {
        let engine = self.engine.as_ref()?;
        let game = engine.games().iter().find(|g| g.id == id)?;
        match self.covers.choice(game.steam_app_id, &engine.game_settings(id)) {
            CoverChoice::Custom(p) | CoverChoice::Steam(p) => Some(path_str(&p)),
            CoverChoice::Generated => None,
        }
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

    /// Backup manual de vários jogos; uma falha não para os outros. Com o cofre
    /// fora, é um erro só (e para no meio se o HD for desligado durante o lote).
    pub fn backup_many(&mut self, ids: &[String]) -> Res<BackupManyView> {
        self.engine()?;
        if !self.vault_available() {
            return Err(ErrorView::new(
                ErrorKind::CofreIndisponivel,
                format!("O cofre em {} não está acessível", path_str(&self.vault_dir())),
            ));
        }
        let _busy = self.busy.guard();
        let mut out = BackupManyView::default();
        for id in ids {
            match self.backup_inner(id, None) {
                Ok(r) if r.outcome == BackupOutcomeView::Feito => out.done.push(id.clone()),
                Ok(_) => out.unchanged.push(id.clone()),
                Err(e) => {
                    let parar = e.kind == ErrorKind::CofreIndisponivel;
                    out.failed.push(BackupFailure { id: id.clone(), title: self.title_for(id), message: e.message });
                    if parar {
                        break;
                    }
                }
            }
        }
        Ok(out)
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

    /// Em "precisa de destino", completa o erro com o alias da raiz (a primeira
    /// com aquele template que ainda não tem destino escolhido).
    fn with_alias(&self, id: &str, file: &str, targets: &BTreeMap<String, String>, mut e: ErrorView) -> ErrorView {
        if e.kind != ErrorKind::PrecisaDestino {
            return e;
        }
        if let (Some(engine), Some(root)) = (&self.engine, e.root.clone()) {
            if let Ok(meta) = engine.snapshot_meta(id, file) {
                e.alias = meta
                    .roots
                    .iter()
                    .find(|r| r.template == root && !targets.contains_key(&r.alias))
                    .map(|r| r.alias.clone());
            }
        }
        e
    }

    pub fn restore_preview(
        &mut self,
        id: &str,
        file: &str,
        targets: &BTreeMap<String, String>,
        probe: &dyn ProcessProbe,
    ) -> Res<RestorePreviewView> {
        let p = match self.engine()?.restore_preview(id, file, &to_paths(targets), probe) {
            Ok(p) => p,
            Err(e) => return Err(self.with_alias(id, file, targets, e.into())),
        };
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
        let report = match self.engine()?.restore(id, file, &opts, probe) {
            Ok(r) => r,
            Err(e) => {
                let e = self.with_alias(id, file, &req.targets, e.into());
                self.record_error(id, &e);
                return Err(e);
            }
        };
        self.refresh_status(id);
        self.record(ActivityKind::Restore, Some(id), texts::restaurados(report.written, report.safety.is_some()));
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
        let g = self.engine()?.add_custom_game(&name, &dirs, &processes, game_dir.as_deref())?;
        if let Some(src) = req.cover_file.as_deref().filter(|c| !c.trim().is_empty()) {
            let dest = cover_name(self.covers.import_custom(Path::new(src)).map_err(erro)?);
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
            Some(src) => Some(cover_name(self.covers.import_custom(&src).map_err(erro)?)),
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
            auto_backup: self.config.auto_backup,
            retention: RetentionView::from(&self.config.retention),
            auto_times: AutoTimesView::from(self.config.auto_times),
            close_to_tray: self.config.close_to_tray,
            notifications: self.config.notifications,
            // Quem sabe é o registro do Windows: o src-tauri preenche.
            start_with_windows: false,
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

    pub fn auto_backup_enabled(&self) -> bool {
        self.config.auto_backup
    }

    /// Chave geral do backup automático (cartão do cofre).
    pub fn set_auto_backup(&mut self, on: bool) -> Res<()> {
        self.config.auto_backup = on;
        self.save_config()
    }

    /// Automático deste jogo. Ligado não guarda nada: vale o padrão.
    pub fn set_game_auto(&mut self, id: &str, on: bool) -> Res<()> {
        self.engine()?.update_game_settings(id, |s| s.auto_backup = if on { None } else { Some(false) })?;
        Ok(())
    }

    /// O automático pode fazer backup deste jogo: chave geral e do jogo ligadas,
    /// e o jogo não está ignorado.
    pub fn can_back_up(&self, id: &str) -> bool {
        self.config.auto_backup
            && self.engine.as_ref().is_some_and(|e| {
                let s = e.game_settings(id);
                s.auto_backup != Some(false) && !s.ignored
            })
    }

    /// A interface mexe nos 3 números dos automáticos; "antes de restaurar" fica como está.
    fn policy(&self, r: &RetentionView) -> RetentionPolicy {
        RetentionPolicy {
            auto_recent: r.auto_recent,
            auto_daily_days: r.auto_daily_days,
            auto_weekly_weeks: r.auto_weekly_weeks,
            pre_restore_keep: self.config.retention.pre_restore_keep,
        }
    }

    pub fn set_retention(&mut self, r: RetentionView) -> Res<()> {
        let p = self.policy(&r);
        self.config.retention = p.clone();
        self.save_config()?;
        if let Some(e) = self.engine.as_mut() {
            e.set_retention(p);
        }
        Ok(())
    }

    pub fn set_game_retention(&mut self, id: &str, r: Option<RetentionView>) -> Res<()> {
        let p = r.map(|r| self.policy(&r));
        self.engine()?.update_game_settings(id, |s| s.retention = p)?;
        Ok(())
    }

    /// Tempos do automático (Configurações), dentro das faixas da Decisão 15 do plano.
    pub fn set_auto_times(&mut self, t: AutoTimesView) -> Res<()> {
        let fora = |v: u32, min: u32, max: u32| v < min || v > max;
        if fora(t.quiet_secs, 5, 600) || fora(t.after_close_secs, 5, 600) {
            return Err(erro(texts::TEMPO_SEGUNDOS));
        }
        if fora(t.while_running_mins, 5, 120) {
            return Err(erro(texts::TEMPO_MINUTOS));
        }
        self.config.auto_times =
            AutoTimes { quiet_secs: t.quiet_secs, after_close_secs: t.after_close_secs, while_running_mins: t.while_running_mins };
        self.save_config()
    }

    pub fn auto_rules(&self) -> AutoRules {
        self.config.auto_times.rules()
    }

    /// Um save mudou (vigia ou varredura): "Protegido" vira "Save novo" na hora,
    /// sem calcular hashes. O backup confirma depois.
    pub fn mark_changed(&mut self, id: &str) {
        if self.statuses.get(id) == Some(&GameStatus::Protected) {
            self.statuses.insert(id.to_string(), GameStatus::NewSave);
        }
    }

    pub fn set_game_include_config(&mut self, id: &str, on: Option<bool>) -> Res<()> {
        self.engine()?.update_game_settings(id, |s| s.include_config = on)?;
        self.refresh_status(id);
        Ok(())
    }

    pub fn pin_backup(&mut self, id: &str, file: &str, pinned: bool) -> Res<SnapshotView> {
        let e = self.engine()?.pin_snapshot(id, file, pinned)?;
        Ok(self.snapshot(&e))
    }

    /// Apaga um backup (a interface já pediu confirmação).
    pub fn delete_backup(&mut self, id: &str, file: &str) -> Res<()> {
        self.engine()?.delete_snapshot(id, file)?;
        self.refresh_status(id);
        self.record(ActivityKind::Notice, Some(id), texts::BACKUP_APAGADO.to_string());
        Ok(())
    }

    /// Os eventos mais novos da Atividade.
    pub fn activity(&self, limit: usize) -> Vec<ActivityEntry> {
        self.activity.recent(limit)
    }

    /// Escaneia e já diz quais capas da Steam faltam, com o serviço travado uma vez só.
    pub fn scan_with_covers(&mut self) -> Res<(LibraryView, Vec<(String, u32)>)> {
        let lib = self.scan()?;
        Ok((lib, self.steam_covers_to_fetch()))
    }

    pub fn close_to_tray(&self) -> bool {
        self.config.close_to_tray
    }

    pub fn set_close_to_tray(&mut self, on: bool) -> Res<()> {
        self.config.close_to_tray = on;
        self.save_config()
    }

    pub fn notifications_enabled(&self) -> bool {
        self.config.notifications
    }

    pub fn set_notifications(&mut self, on: bool) -> Res<()> {
        self.config.notifications = on;
        self.save_config()
    }

    pub fn game_ids_for_path(&self, path: &Path) -> Vec<String> {
        self.engine.as_ref().map(|e| e.games_for_path(path)).unwrap_or_default()
    }

    pub fn changed_game_ids(&self) -> Vec<String> {
        self.engine.as_ref().map(|e| e.changed_games()).unwrap_or_default()
    }

    /// Para cada jogo encontrado: aberto, fechado ou "não dá para saber".
    pub fn running_states(&self, probe: &dyn ProcessProbe) -> Vec<(String, Running)> {
        let Some(engine) = &self.engine else { return Vec::new() };
        let snapshot = ProcessSnapshot(probe.running());
        engine
            .games()
            .iter()
            .filter(|g| !g.vault_only)
            .map(|g| {
                let r = if !can_tell_running(g) {
                    Running::Unknown
                } else if is_game_running(g, &snapshot) {
                    Running::Yes
                } else {
                    Running::No
                };
                (g.id.clone(), r)
            })
            .collect()
    }

    /// Passa a usar o cofre de `dir`. Sem cofre lá, só cria quando `create`.
    pub fn change_vault(&mut self, dir: &Path, create: bool) -> Res<AppStateView> {
        let dir = vault_target(dir)?;
        if !dir.join("cofre.json").is_file() {
            if !create {
                return Err(ErrorView::new(ErrorKind::NaoEncontrado, "Não há um cofre nessa pasta."));
            }
            Vault::create(&dir)?;
        }
        self.config.vault_dir = self.stored_vault(&dir);
        self.save_config()?;
        self.reset_engine();
        Ok(self.state())
    }

    /// Só lê: onde o cofre de `dir` fica, se ele já existe e quantos backups tem.
    pub fn inspect_vault(&self, dir: &Path) -> Res<VaultProbeView> {
        let dir = vault_target(dir)?;
        let exists = dir.join("cofre.json").is_file();
        let mut versions = 0;
        if exists {
            for sub in std::fs::read_dir(&dir).into_iter().flatten().flatten().filter(|s| s.path().is_dir()) {
                versions += std::fs::read_dir(sub.path())
                    .into_iter()
                    .flatten()
                    .flatten()
                    .filter(|f| SnapshotKind::from_file_name(&f.file_name().to_string_lossy()).is_some())
                    .count();
            }
        }
        Ok(VaultProbeView { exists, versions, path: path_str(&dir) })
    }

    pub fn retry_vault(&mut self) -> AppStateView {
        self.reset_engine();
        self.state()
    }

    /// Sem internet não é erro: a interface mostra o aviso e segue. Não liga o
    /// "ocupado": baixar a base não mexe em save nenhum (e os arquivos dela são
    /// gravados de forma atômica), então a janela pode fechar no meio.
    pub fn update_manifest(&mut self, fetcher: &dyn Fetcher) -> Res<ManifestUpdateView> {
        Ok(match self.engine()?.update_manifest(fetcher) {
            Ok(UpdateOutcome::Updated) => ManifestUpdateView { outcome: ManifestOutcome::Atualizada, message: None },
            Ok(_) => ManifestUpdateView { outcome: ManifestOutcome::SemNovidade, message: None },
            Err(e) => ManifestUpdateView { outcome: ManifestOutcome::Falhou, message: Some(e.to_string()) },
        })
    }

    pub fn finish_first_run(&mut self, vault_dir: &Path) -> Res<AppStateView> {
        let vault_dir = vault_target(vault_dir)?;
        Vault::create(&vault_dir)?;
        self.config.vault_dir = self.stored_vault(&vault_dir);
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
