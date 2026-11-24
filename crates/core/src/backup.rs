//! Backup de um jogo para o cofre (manual, automático ou antes de restaurar).

use chrono::{DateTime, Local, Utc};

use crate::error::Result;
use crate::fingerprint::{HashCache, fingerprint_of_game};
use crate::machine::MachineIdentity;
use crate::model::Game;
use crate::vault::Vault;
use crate::vault::format::{FORMAT, GameDescriptor, SnapshotKind, SnapshotMeta, SnapshotRoot, snapshot_file_name};
use crate::vault::index::{SnapshotEntry, VaultIndex, entry_from_meta};
use crate::vault::retention::{RetentionPolicy, plan_prune};
use crate::vault::write::{CopyHooks, SourceFile, write_snapshot};

pub struct BackupCtx<'a> {
    pub vault: &'a Vault,
    pub index: &'a mut VaultIndex,
    pub hashes: &'a mut HashCache,
    pub machine: &'a MachineIdentity,
    pub hooks: &'a dyn CopyHooks,
    pub now: DateTime<Local>,
    pub app_version: &'a str,
    pub retention: &'a RetentionPolicy,
}

#[derive(Debug)]
pub enum BackupOutcome {
    Created(SnapshotEntry),
    /// Já existia um backup idêntico (só para automático e antes-de-restaurar).
    Unchanged(SnapshotEntry),
    NothingToBackup,
}

/// Faz o backup e aplica a limpeza automática deste jogo neste PC.
/// Quem chama salva o índice e o cache de hashes depois.
pub fn backup_game(
    ctx: &mut BackupCtx,
    game: &Game,
    kind: SnapshotKind,
    label: Option<&str>,
) -> Result<BackupOutcome> {
    ctx.vault.ensure_available()?;
    if game.files.is_empty() {
        return Ok(BackupOutcome::NothingToBackup);
    }
    let fp = fingerprint_of_game(game, ctx.hashes)?;
    if kind != SnapshotKind::Manual {
        if let Some(e) = ctx.index.find_identical(&game.id, &fp) {
            return Ok(BackupOutcome::Unchanged(e.clone()));
        }
    }

    let dir = ctx.vault.game_dir(&game.id, &game.title)?;
    let dir_name = dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    ctx.vault.write_descriptor(
        &dir,
        &GameDescriptor {
            format: FORMAT,
            game_id: game.id.clone(),
            title: game.title.clone(),
            source: game.source.clone(),
            custom: game.custom.clone(),
            steam_app_id: game.steam_app_id,
        },
    )?;

    let roots = game
        .save_roots
        .iter()
        .enumerate()
        .map(|(i, r)| SnapshotRoot {
            alias: format!("r{i}"),
            template: r.template.clone(),
            bindings: r.bindings.clone(),
            original_path: r.path.clone(),
        })
        .collect();
    let sources: Vec<SourceFile> = game
        .files
        .iter()
        .map(|f| SourceFile { abs_path: f.abs_path.clone(), root_alias: format!("r{}", f.root), rel_path: f.rel_path.clone() })
        .collect();
    let meta = SnapshotMeta {
        format: FORMAT,
        game_id: game.id.clone(),
        title: game.title.clone(),
        kind,
        label: label.map(str::trim).filter(|l| !l.is_empty()).map(str::to_string),
        created_at: ctx.now.with_timezone(&Utc),
        app_version: ctx.app_version.to_string(),
        machine: ctx.machine.clone(),
        roots,
        files: Vec::new(),
    };
    let name = snapshot_file_name(ctx.now, ctx.machine, kind);
    let (path, meta) = write_snapshot(&dir, &name, meta, &sources, ctx.hooks)?;
    let file_name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let entry = entry_from_meta(&meta, &dir_name, &file_name, kind);
    ctx.index.insert(entry.clone());

    let prune: Vec<(String, String)> = plan_prune(&ctx.index.for_game(&game.id), &ctx.machine.id, ctx.now, ctx.retention)
        .into_iter()
        .map(|e| (e.dir_name.clone(), e.file_name.clone()))
        .collect();
    for (d, f) in prune {
        // Se um backup velho não puder ser apagado agora, fica para a próxima limpeza.
        if ctx.vault.delete_snapshot(&d, &f).is_ok() {
            ctx.index.remove(&d, &f);
        }
    }
    Ok(BackupOutcome::Created(entry))
}
