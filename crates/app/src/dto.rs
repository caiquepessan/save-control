//! Tipos trocados com a interface. O ts-rs gera os `.ts` em `ui/src/api/gerado/`
//! quando os testes deste crate rodam. Caminhos e datas vão como texto (datas em
//! RFC 3339); números grandes vão como `number`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

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
