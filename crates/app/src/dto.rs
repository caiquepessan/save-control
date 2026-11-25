//! Tipos trocados com a interface. O ts-rs gera os `.ts` em `ui/src/api/gerado/`
//! quando os testes deste crate rodam. Caminhos e datas vão como texto (datas em
//! RFC 3339); números grandes vão como `number`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use savecontrol_core::vault::retention::RetentionPolicy;

use crate::config::AutoTimes;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum AppPhase {
    PrimeiroUso,
    CofreSumiu,
    Falhou,
    Pronto,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AppStateView {
    pub phase: AppPhase,
    pub vault_dir: String,
    pub data_dir: String,
    pub default_vault: String,
    pub warnings: Vec<String>,
    pub version: String,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LibraryView {
    pub games: Vec<GameView>,
    pub vault: VaultView,
    pub scanned_at: Option<String>,
    pub manifest_loaded: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum GameKind {
    Pc,
    Recomp,
    Emulador,
    Manual,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum SourceKey {
    Steam,
    Epic,
    Gog,
    Pc,
    Catalogo,
    Manual,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum StatusView {
    Protegido,
    SaveNovo,
    SemBackup,
    SemSave,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GameView {
    pub id: String,
    /// Nome mostrado (o ajuste deste PC, se houver).
    pub title: String,
    pub original_title: String,
    pub kind: GameKind,
    pub source_key: SourceKey,
    pub source_label: String,
    pub status: StatusView,
    pub vault_only: bool,
    pub last_saved: Option<String>,
    pub last_backup: Option<String>,
    #[ts(type = "number")]
    pub backup_count: usize,
    #[ts(type = "number")]
    pub total_size: u64,
    #[ts(type = "number")]
    pub file_count: usize,
    pub roots: Vec<RootView>,
    pub warnings: Vec<WarningView>,
    pub cover: CoverView,
    pub process_names: Vec<String>,
    pub game_dir: Option<String>,
    pub needs_game_dir: bool,
    pub is_custom: bool,
    pub remembered_targets: Vec<RememberedTarget>,
    /// Automático ligado neste jogo (a chave da aba Ajustes).
    #[serde(default = "ligado")]
    pub auto_backup: bool,
    /// Retenção só deste jogo (`null` = a geral).
    #[serde(default)]
    pub retention: Option<RetentionView>,
    /// "Incluir configurações" só deste jogo (`null` = o geral).
    #[serde(default)]
    pub include_config: Option<bool>,
}

fn ligado() -> bool {
    true
}

/// Os 3 números da retenção dos automáticos (§8.2).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RetentionView {
    #[ts(type = "number")]
    pub auto_recent: usize,
    pub auto_daily_days: u32,
    pub auto_weekly_weeks: u32,
}

impl From<&RetentionPolicy> for RetentionView {
    fn from(p: &RetentionPolicy) -> Self {
        RetentionView { auto_recent: p.auto_recent, auto_daily_days: p.auto_daily_days, auto_weekly_weeks: p.auto_weekly_weeks }
    }
}

/// Tempos do automático (§9), ajustáveis em Configurações.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AutoTimesView {
    pub quiet_secs: u32,
    pub after_close_secs: u32,
    pub while_running_mins: u32,
}

impl From<AutoTimes> for AutoTimesView {
    fn from(t: AutoTimes) -> Self {
        AutoTimesView { quiet_secs: t.quiet_secs, after_close_secs: t.after_close_secs, while_running_mins: t.while_running_mins }
    }
}

/// Situação do backup automático (cartão do cofre e ícone da bandeja).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum AutoState {
    Ativo,
    Pausado,
    Desligado,
    CofreFora,
    /// Sem a Engine aberta (primeiro uso, cofre sumido, falhou).
    Parado,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LastBackupView {
    pub title: String,
    pub at: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AutoStatusView {
    pub state: AutoState,
    /// Até quando está pausado (sem pausa, ou "até reabrir": null).
    pub paused_until: Option<String>,
    /// Quantos jogos o automático vigia.
    #[ts(type = "number")]
    pub watching: usize,
    /// Fazendo backup agora.
    pub busy: bool,
    pub last_backup: Option<LastBackupView>,
}

/// Resultado de um backup automático (para a notificação e a Atividade).
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AutoBackupView {
    pub id: String,
    pub title: String,
    pub outcome: BackupOutcomeView,
    #[ts(type = "number")]
    pub file_count: usize,
    #[ts(type = "number")]
    pub total_size: u64,
    /// Arquivo da capa (própria ou da Steam), se houver.
    pub cover: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RootView {
    pub path: String,
    pub template: String,
    pub files: Vec<FileView>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FileView {
    pub rel_path: String,
    #[ts(type = "number")]
    pub size: u64,
    pub modified: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum WarningKind {
    PastaArriscada,
    DentroDaPastaDoJogo,
    Registro,
    Nuvem,
    PastaAmpla,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WarningView {
    pub kind: WarningKind,
    pub detail: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum CoverKind {
    Propria,
    Steam,
    Gerada,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CoverView {
    pub kind: CoverKind,
    pub path: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RememberedTarget {
    pub template: String,
    pub path: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum SnapshotKindView {
    Manual,
    Auto,
    AntesDeRestaurar,
    Fixado,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SnapshotView {
    pub file_name: String,
    pub kind: SnapshotKindView,
    pub label: Option<String>,
    pub created_at: String,
    pub pc_name: String,
    pub from_this_pc: bool,
    #[ts(type = "number")]
    pub total_size: u64,
    #[ts(type = "number")]
    pub file_count: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SnapshotFileView {
    pub root: String,
    pub rel_path: String,
    #[ts(type = "number")]
    pub size: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RestoreTargetView {
    pub alias: String,
    pub template: String,
    pub path: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RestorePreviewView {
    pub snapshot: SnapshotView,
    pub targets: Vec<RestoreTargetView>,
    #[ts(type = "number")]
    pub file_count: usize,
    #[ts(type = "number")]
    pub total_size: u64,
    pub running: bool,
    pub single_root: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FileRef {
    pub root: String,
    pub rel_path: String,
}

/// Pedido de restauração vindo da interface.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct RestoreRequest {
    /// Alias da raiz → pasta onde restaurar.
    pub targets: BTreeMap<String, String>,
    pub only_files: Option<Vec<FileRef>>,
    /// Lembrar `targets` para as próximas vezes (escolha feita quando o app perguntou).
    pub remember: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RestoreResultView {
    #[ts(type = "number")]
    pub written: usize,
    #[ts(type = "number")]
    pub removed: usize,
    pub safety: Option<SnapshotView>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum BackupOutcomeView {
    Feito,
    SemMudanca,
    SemArquivos,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BackupResultView {
    pub outcome: BackupOutcomeView,
    pub snapshot: Option<SnapshotView>,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BackupFailure {
    pub id: String,
    pub title: String,
    pub message: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BackupManyView {
    pub done: Vec<String>,
    pub unchanged: Vec<String>,
    pub failed: Vec<BackupFailure>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct AddGameRequest {
    pub name: String,
    pub save_dirs: Vec<String>,
    pub game_dir: Option<String>,
    pub processes: Vec<String>,
    pub cover_file: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct IgnoredGame {
    pub id: String,
    pub title: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SettingsView {
    pub vault_dir: String,
    pub user_roots: Vec<String>,
    pub include_config: bool,
    pub steam_covers: bool,
    pub manifest_loaded: bool,
    pub ignored: Vec<IgnoredGame>,
    pub version: String,
    pub auto_backup: bool,
    pub retention: RetentionView,
    pub auto_times: AutoTimesView,
    pub close_to_tray: bool,
    pub notifications: bool,
    pub start_with_windows: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ManifestOutcome {
    Atualizada,
    SemNovidade,
    Falhou,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ManifestUpdateView {
    pub outcome: ManifestOutcome,
    pub message: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct VaultView {
    pub path: String,
    pub available: bool,
    #[ts(type = "number")]
    pub versions: usize,
    #[ts(type = "number")]
    pub zip_bytes: u64,
    #[ts(type = "number | null")]
    pub disk_total: Option<u64>,
    #[ts(type = "number | null")]
    pub disk_free: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct VaultProbeView {
    pub exists: bool,
    #[ts(type = "number")]
    pub versions: usize,
    /// Onde o cofre fica (ou vai ficar): a pasta escolhida ou a subpasta "cofre" dela.
    pub path: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ErrorKind {
    JogoAberto,
    PrecisaDestino,
    CofreIndisponivel,
    RestauracaoRecusada,
    DesfazerFalhou,
    VersaoNova,
    NaoEncontrado,
    Erro,
}

/// Erro que a interface recebe (a `message` já vem em pt-BR).
#[derive(Clone, Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ErrorView {
    pub kind: ErrorKind,
    pub message: String,
    /// Em `precisaDestino`: o template da raiz que precisa de destino…
    pub root: Option<String>,
    /// …e o alias dela no backup (para a interface mandar o destino certo).
    pub alias: Option<String>,
    pub candidates: Vec<String>,
}
