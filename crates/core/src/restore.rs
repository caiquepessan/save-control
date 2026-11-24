//! Restauração exata de um backup, com backup de segurança antes e desfazer se falhar.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::{Path, PathBuf};

use crate::backup::{BackupCtx, BackupOutcome, backup_game};
use crate::error::{Error, Result, io};
use crate::model::Game;
use crate::placeholder::slash;
use crate::process::{ProcessProbe, is_game_running};
use crate::targets::{TargetCtx, resolve_targets};
use crate::vault::Vault;
use crate::vault::format::{SnapshotKind, SnapshotMeta};
use crate::vault::index::SnapshotEntry;
use crate::vault::read::{extract_file, read_meta};

#[derive(Clone, Debug, Default)]
pub struct RestoreOptions {
    /// Alias da raiz → pasta onde restaurar (em vez do lugar original).
    pub overrides: BTreeMap<String, PathBuf>,
    /// Só estes arquivos (alias, rel_path); `None` = restauração completa.
    pub only: Option<BTreeSet<(String, String)>>,
}

#[derive(Debug)]
pub struct RestoreReport {
    pub written: usize,
    pub removed: usize,
    /// O backup "antes de restaurar" (ou um idêntico que já existia).
    pub safety: Option<SnapshotEntry>,
}

fn key(p: &Path) -> String {
    slash(p).to_lowercase()
}

fn template_of<'m>(meta: &'m SnapshotMeta, alias: &str) -> &'m str {
    meta.roots.iter().find(|r| r.alias == alias).map(|r| r.template.as_str()).unwrap_or_default()
}

/// Extrai para um temporário na mesma pasta e troca pelo destino.
fn write_from_zip(zip: &Path, alias: &str, rel_path: &str, dest: &Path) -> Result<()> {
    let dir = dest.parent().ok_or_else(|| Error::NotFound(dest.display().to_string()))?;
    std::fs::create_dir_all(dir).map_err(io(dir))?;
    let name = dest.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let tmp = dir.join(format!(".{name}.savecontrol-tmp"));
    extract_file(zip, alias, rel_path, &tmp)?;
    if let Ok(meta) = std::fs::metadata(dest) {
        let mut perms = meta.permissions();
        if meta.is_file() && perms.readonly() {
            #[allow(clippy::permissions_set_readonly_false)]
            perms.set_readonly(false);
            let _ = std::fs::set_permissions(dest, perms);
        }
    }
    if let Err(e) = std::fs::rename(&tmp, dest) {
        let _ = std::fs::remove_file(&tmp);
        return Err(io(dest)(e));
    }
    Ok(())
}

fn hash_file(path: &Path) -> Result<String> {
    let mut f = std::fs::File::open(path).map_err(io(path))?;
    let mut h = blake3::Hasher::new();
    std::io::copy(&mut f, &mut h).map_err(io(path))?;
    Ok(h.finalize().to_hex().to_string())
}

/// Volta os arquivos do jogo ao backup de segurança e apaga o que foi gravado a mais.
fn rollback(vault: &Vault, game: &Game, safety: Option<&SnapshotEntry>, written: &[PathBuf]) {
    let mut keep = HashSet::new();
    if let Some(s) = safety {
        let zip = s.zip_path(vault);
        if let Ok(meta) = read_meta(&zip) {
            for f in &meta.files {
                let template = template_of(&meta, &f.root);
                let root = game
                    .save_roots
                    .iter()
                    .find(|r| r.template == template)
                    .map(|r| r.path.clone())
                    .or_else(|| meta.roots.iter().find(|r| r.alias == f.root).map(|r| r.original_path.clone()));
                if let Some(root) = root {
                    let dest = root.join(&f.rel_path);
                    keep.insert(key(&dest));
                    let _ = write_from_zip(&zip, &f.root, &f.rel_path, &dest);
                }
            }
        }
    }
    for w in written {
        if !keep.contains(&key(w)) {
            let _ = std::fs::remove_file(w);
        }
    }
}

/// Restaura `entry` por cima do save atual do jogo.
pub fn restore_snapshot(
    ctx: &mut BackupCtx,
    targets: &TargetCtx,
    probe: &dyn ProcessProbe,
    game: &Game,
    others: &[Game],
    entry: &SnapshotEntry,
    opts: &RestoreOptions,
) -> Result<RestoreReport> {
    if is_game_running(game, probe) {
        return Err(Error::GameRunning(game.title.clone()));
    }
    ctx.vault.ensure_available()?;
    let zip = entry.zip_path(ctx.vault);
    let meta = read_meta(&zip)?;
    let dests = resolve_targets(&meta, targets, &opts.overrides)?;
    let dest_of = |alias: &str| {
        dests.iter().find(|(a, _)| a == alias).map(|(_, p)| p.clone()).ok_or_else(|| Error::NotFound(alias.to_string()))
    };

    let safety = match backup_game(ctx, game, SnapshotKind::PreRestore, None)? {
        BackupOutcome::Created(e) | BackupOutcome::Unchanged(e) => Some(e),
        BackupOutcome::NothingToBackup => None,
    };

    let selected: Vec<_> = meta
        .files
        .iter()
        .filter(|f| opts.only.as_ref().is_none_or(|o| o.contains(&(f.root.clone(), f.rel_path.clone()))))
        .collect();
    let mut written: Vec<PathBuf> = Vec::new();
    let mut removed = 0;

    let result = (|| -> Result<()> {
        for f in &selected {
            let dest = dest_of(&f.root)?.join(&f.rel_path);
            write_from_zip(&zip, &f.root, &f.rel_path, &dest)?;
            written.push(dest);
        }
        if opts.only.is_none() {
            let in_snapshot: HashSet<(String, String)> = meta
                .files
                .iter()
                .map(|f| (template_of(&meta, &f.root).to_string(), f.rel_path.to_lowercase()))
                .collect();
            let claimed: HashSet<String> = others
                .iter()
                .filter(|o| o.id != game.id)
                .flat_map(|o| o.files.iter().map(|f| key(&f.abs_path)))
                .collect();
            for gf in &game.files {
                let root = &game.save_roots[gf.root];
                // Só raízes que este backup tem e que foram restauradas no lugar de sempre.
                let Some(mr) = meta.roots.iter().find(|r| r.template == root.template) else { continue };
                if key(&dest_of(&mr.alias)?) != key(&root.path) {
                    continue;
                }
                if in_snapshot.contains(&(root.template.clone(), gf.rel_path.to_lowercase()))
                    || claimed.contains(&key(&gf.abs_path))
                {
                    continue;
                }
                std::fs::remove_file(&gf.abs_path).map_err(io(&gf.abs_path))?;
                removed += 1;
            }
        }
        for (f, dest) in selected.iter().zip(&written) {
            if hash_file(dest)? != f.blake3 {
                return Err(Error::VerifyFailed(dest.clone()));
            }
        }
        Ok(())
    })();

    if let Err(e) = result {
        rollback(ctx.vault, game, safety.as_ref(), &written);
        return Err(e);
    }
    Ok(RestoreReport { written: written.len(), removed, safety })
}
