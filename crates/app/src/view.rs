//! Como os dados do núcleo aparecem para a interface.

use std::path::Path;

use chrono::{DateTime, Utc};

use savecontrol_core::catalog::{Catalog, CatalogKind};
use savecontrol_core::engine::VaultSummary;
use savecontrol_core::env::SystemPaths;
use savecontrol_core::local_state::LocalGameSettings;
use savecontrol_core::model::{Game, GameSource, Warning};
use savecontrol_core::status::GameStatus;
use savecontrol_core::vault::format::SnapshotKind;
use savecontrol_core::vault::index::SnapshotEntry;

use crate::covers::{CoverChoice, CoverStore};
use crate::dto::*;

pub struct ViewCtx<'a> {
    pub sys: &'a SystemPaths,
    pub catalog: &'a Catalog,
    /// Id deste PC (para marcar backups vindos de outro PC).
    pub machine_id: &'a str,
    pub covers: &'a CoverStore,
}

/// Caminho para mostrar no Windows (barras invertidas).
pub fn path_str(p: &Path) -> String {
    p.to_string_lossy().replace('/', "\\")
}

pub fn date_str(d: &DateTime<Utc>) -> String {
    d.to_rfc3339()
}

fn lower_slash(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/").trim_end_matches('/').to_lowercase()
}

fn is_under(path: &Path, dir: &Path) -> bool {
    let (p, d) = (lower_slash(path), lower_slash(dir));
    !d.is_empty() && (p == d || p.starts_with(&format!("{d}/")))
}

/// Nome amigável da pasta arriscada (§15).
fn risky_label(path: &Path, sys: &SystemPaths) -> String {
    if is_under(path, &sys.downloads) || lower_slash(path).split('/').any(|c| c == "downloads") {
        "Downloads".into()
    } else if is_under(path, &sys.desktop) {
        "Área de Trabalho".into()
    } else if is_under(path, &sys.temp) {
        "Temp".into()
    } else {
        path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| path_str(path))
    }
}

fn store_label(store: &str) -> String {
    match store {
        "steam" => "Steam",
        "epic" => "Epic",
        "gog" => "GOG",
        "origin" => "EA",
        "uplay" => "Ubisoft",
        other => other,
    }
    .to_string()
}

fn warning_view(w: &Warning, sys: &SystemPaths) -> WarningView {
    let (kind, detail) = match w {
        Warning::RiskyLocation(p) => (WarningKind::PastaArriscada, Some(risky_label(p, sys))),
        Warning::InsideGameFolder => (WarningKind::DentroDaPastaDoJogo, None),
        Warning::RegistryNotIncluded => (WarningKind::Registro, None),
        Warning::StoreCloud(s) => (WarningKind::Nuvem, Some(store_label(s))),
        Warning::IgnoredBroadPath(t) => (WarningKind::PastaAmpla, Some(t.clone())),
    };
    WarningView { kind, detail }
}

fn status_view(s: GameStatus) -> StatusView {
    match s {
        GameStatus::Protected => StatusView::Protegido,
        GameStatus::NewSave => StatusView::SaveNovo,
        GameStatus::NoBackup => StatusView::SemBackup,
        GameStatus::NoSave => StatusView::SemSave,
    }
}

pub fn snapshot_kind_view(k: SnapshotKind) -> SnapshotKindView {
    match k {
        SnapshotKind::Manual => SnapshotKindView::Manual,
        SnapshotKind::Auto => SnapshotKindView::Auto,
        SnapshotKind::PreRestore => SnapshotKindView::AntesDeRestaurar,
        SnapshotKind::Pinned => SnapshotKindView::Fixado,
    }
}

pub fn snapshot_view(ctx: &ViewCtx, e: &SnapshotEntry) -> SnapshotView {
    SnapshotView {
        file_name: e.file_name.clone(),
        kind: snapshot_kind_view(e.kind),
        label: e.label.clone(),
        created_at: date_str(&e.created_at),
        pc_name: e.pc_name.clone(),
        from_this_pc: e.machine_id == ctx.machine_id,
        total_size: e.total_size,
        file_count: e.file_count,
    }
}

pub fn vault_view(s: &VaultSummary) -> VaultView {
    VaultView {
        path: path_str(&s.path),
        available: s.available,
        versions: s.versions,
        zip_bytes: s.zip_bytes,
        disk_total: s.disk_total,
        disk_free: s.disk_free,
    }
}

pub fn game_view(
    ctx: &ViewCtx,
    game: &Game,
    status: GameStatus,
    history: &[SnapshotEntry],
    settings: &LocalGameSettings,
) -> GameView {
    let entry = game
        .id
        .strip_prefix("cat:")
        .map(|rest| rest.split(':').next().unwrap_or(rest))
        .and_then(|id| ctx.catalog.entries.iter().find(|e| e.id == id));
    let is_custom = game.custom.is_some() || game.source == GameSource::Custom || game.id.starts_with("man:");
    let kind = match (entry, is_custom) {
        (Some(e), _) => match e.kind {
            CatalogKind::Recomp | CatalogKind::Port => GameKind::Recomp,
            CatalogKind::Emulator => GameKind::Emulador,
        },
        (None, true) => GameKind::Manual,
        (None, false) if game.source == GameSource::Catalog => GameKind::Recomp,
        _ => GameKind::Pc,
    };
    let source_key = match game.source {
        GameSource::Steam => SourceKey::Steam,
        GameSource::Epic => SourceKey::Epic,
        GameSource::Gog => SourceKey::Gog,
        GameSource::Other | GameSource::ManifestOnly => SourceKey::Pc,
        GameSource::Catalog => SourceKey::Catalogo,
        GameSource::Custom => SourceKey::Manual,
    };
    let source_label = if game.vault_only {
        "Só no cofre".to_string()
    } else {
        match game.source {
            GameSource::Steam => "Steam".into(),
            GameSource::Epic => "Epic Games".into(),
            GameSource::Gog => "GOG".into(),
            GameSource::Other | GameSource::ManifestOnly => "PC".into(),
            GameSource::Catalog => entry.map(|e| e.name.clone()).unwrap_or_else(|| "Catálogo".into()),
            GameSource::Custom => "Adicionado à mão".into(),
        }
    };

    let roots = game
        .save_roots
        .iter()
        .enumerate()
        .map(|(i, r)| RootView {
            path: path_str(&r.path),
            template: r.template.clone(),
            files: game
                .files
                .iter()
                .filter(|f| f.root == i)
                .map(|f| FileView { rel_path: f.rel_path.clone(), size: f.size, modified: date_str(&f.modified) })
                .collect(),
        })
        .collect();

    let game_dir = settings.game_dir.clone().or_else(|| game.game_dir.clone()).or_else(|| game.install_dirs.first().cloned());
    let uses_game_dir = game.save_roots.iter().any(|r| r.template.contains("<gameDir>"))
        || game.custom.as_ref().is_some_and(|c| c.save_templates.iter().any(|t| t.contains("<gameDir>")));

    let cover = match ctx.covers.choice(game.steam_app_id, settings) {
        CoverChoice::Custom(p) => CoverView { kind: CoverKind::Propria, path: Some(path_str(&p)) },
        CoverChoice::Steam(p) => CoverView { kind: CoverKind::Steam, path: Some(path_str(&p)) },
        CoverChoice::Generated => CoverView { kind: CoverKind::Gerada, path: None },
    };

    GameView {
        id: game.id.clone(),
        title: settings.title.clone().filter(|t| !t.trim().is_empty()).unwrap_or_else(|| game.title.clone()),
        original_title: game.title.clone(),
        kind,
        source_key,
        source_label,
        status: status_view(status),
        vault_only: game.vault_only,
        last_saved: game.last_saved.as_ref().map(date_str),
        last_backup: history.iter().map(|e| e.created_at).max().as_ref().map(date_str),
        backup_count: history.len(),
        total_size: game.total_size,
        file_count: game.files.len(),
        roots,
        warnings: game.warnings.iter().map(|w| warning_view(w, ctx.sys)).collect(),
        cover,
        process_names: game.process_names.clone(),
        needs_game_dir: uses_game_dir && game_dir.is_none(),
        game_dir: game_dir.as_deref().map(path_str),
        is_custom,
        remembered_targets: settings
            .target_choices
            .iter()
            .map(|(template, path)| RememberedTarget { template: template.clone(), path: path_str(path) })
            .collect(),
    }
}
