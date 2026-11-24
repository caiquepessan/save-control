//! Restauração exata de um backup, com backup de segurança antes e desfazer se falhar.
//!
//! Regra de ouro: o backup de segurança e o desfazer saem do que está NO DISCO,
//! nos destinos resolvidos agora — nunca de um escaneamento guardado.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::{Path, PathBuf};

use chrono::Utc;

use crate::backup::BackupCtx;
use crate::error::{Error, Result, io};
use crate::model::Game;
use crate::placeholder::slash;
use crate::process::{ProcessProbe, is_game_running};
use crate::targets::{TargetCtx, resolve_targets};
use crate::vault::Vault;
use crate::vault::format::{
    FORMAT, GameDescriptor, SnapshotFile, SnapshotKind, SnapshotMeta, SnapshotRoot, snapshot_file_name,
};
use crate::vault::index::{SnapshotEntry, entry_from_meta};
use crate::vault::read::{extract_file, read_meta};
use crate::vault::write::{SourceFile, write_snapshot};

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
    /// O backup "antes de restaurar" com o que existia nos destinos (se existia algo).
    pub safety: Option<SnapshotEntry>,
}

/// Uma restauração completa nunca apaga mais que isto de arquivos fora do backup
/// (a menos que o próprio backup tenha mais arquivos que isso).
const MAX_EXTRAS: usize = 20;

fn key(p: &Path) -> String {
    slash(p).to_lowercase()
}

/// Caminho relativo seguro: sem raiz, sem letra de unidade, sem `.`/`..`.
fn safe_rel(rel: &str) -> bool {
    !rel.is_empty()
        && !rel.contains(['\\', ':'])
        && rel.split('/').all(|c| !c.is_empty() && c != "." && c != "..")
}

/// Extrai para um temporário na mesma pasta e troca pelo destino.
fn write_from_zip(zip: &Path, alias: &str, rel_path: &str, dest: &Path) -> Result<()> {
    let dir = dest.parent().ok_or_else(|| Error::NotFound(dest.display().to_string()))?;
    std::fs::create_dir_all(dir).map_err(io(dir))?;
    let name = dest.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let tmp = dir.join(format!(".{name}.savecontrol-tmp"));
    if let Err(e) = extract_file(zip, alias, rel_path, &tmp) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
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

/// Um arquivo que existe agora e será sobrescrito ou apagado: (alias, rel, caminho).
type Touched = (String, String, PathBuf);

/// Guarda no cofre o que existe agora nos destinos. Não faz limpeza (a limpeza
/// poderia apagar justamente o backup que está sendo restaurado).
fn write_safety(
    ctx: &mut BackupCtx,
    game: &Game,
    meta: &SnapshotMeta,
    dests: &[(String, PathBuf)],
    existing: &[Touched],
) -> Result<Option<SnapshotEntry>> {
    if existing.is_empty() {
        return Ok(None);
    }
    let dir = ctx.vault.game_dir(&game.id, &game.title)?;
    if Vault::read_descriptor(&dir).is_none() {
        ctx.vault.write_descriptor(
            &dir,
            &GameDescriptor {
                format: FORMAT,
                game_id: game.id.clone(),
                title: game.title.clone(),
                source: game.source.clone(),
                custom: game.custom.clone(),
                steam_app_id: game.steam_app_id,
                process_names: game.process_names.clone(),
            },
        )?;
    }
    let roots = meta
        .roots
        .iter()
        .filter_map(|r| {
            let (_, dest) = dests.iter().find(|(a, _)| *a == r.alias)?;
            Some(SnapshotRoot {
                alias: r.alias.clone(),
                template: r.template.clone(),
                bindings: r.bindings.clone(),
                original_path: dest.clone(),
            })
        })
        .collect();
    let sources: Vec<SourceFile> = existing
        .iter()
        .map(|(alias, rel, abs)| SourceFile { abs_path: abs.clone(), root_alias: alias.clone(), rel_path: rel.clone() })
        .collect();
    let safety_meta = SnapshotMeta {
        format: FORMAT,
        game_id: game.id.clone(),
        title: game.title.clone(),
        kind: SnapshotKind::PreRestore,
        label: None,
        created_at: ctx.now.with_timezone(&Utc),
        app_version: ctx.app_version.to_string(),
        machine: ctx.machine.clone(),
        roots,
        files: Vec::new(),
    };
    let name = snapshot_file_name(ctx.now, ctx.machine, SnapshotKind::PreRestore);
    let (path, safety_meta) = write_snapshot(&dir, &name, safety_meta, &sources, ctx.hooks)?;
    let dir_name = dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let file_name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let entry = entry_from_meta(&safety_meta, &dir_name, &file_name, SnapshotKind::PreRestore);
    ctx.index.insert(entry.clone());
    Ok(Some(entry))
}

/// Devolve o que o backup de segurança guardou (pelo mesmo mapa alias → destino)
/// e apaga o que foi gravado onde antes não havia nada. Devolve quantas falhas houve.
fn rollback(
    vault: &Vault,
    safety: Option<&SnapshotEntry>,
    dests: &[(String, PathBuf)],
    written: &[PathBuf],
    existed: &HashSet<String>,
) -> usize {
    let mut failures = 0;
    if let Some(s) = safety {
        let zip = s.zip_path(vault);
        match read_meta(&zip) {
            Ok(meta) => {
                for f in &meta.files {
                    let ok = dests
                        .iter()
                        .find(|(a, _)| *a == f.root)
                        .is_some_and(|(_, dir)| write_from_zip(&zip, &f.root, &f.rel_path, &dir.join(&f.rel_path)).is_ok());
                    if !ok {
                        failures += 1;
                    }
                }
            }
            Err(_) => failures += 1,
        }
    }
    for w in written {
        if !existed.contains(&key(w)) && w.is_file() && std::fs::remove_file(w).is_err() {
            failures += 1;
        }
    }
    failures
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
    if let Some(bad) = meta.files.iter().find(|f| !safe_rel(&f.rel_path)) {
        return Err(Error::UnsafeRestore(format!("o backup tem um caminho inválido: {}", bad.rel_path)));
    }
    let dests = resolve_targets(&meta, targets, &opts.overrides)?;
    let dest_of = |alias: &str| {
        dests.iter().find(|(a, _)| a == alias).map(|(_, p)| p.clone()).ok_or_else(|| Error::NotFound(alias.to_string()))
    };

    // O que vai ser gravado.
    let mut plan: Vec<(&SnapshotFile, PathBuf)> = Vec::new();
    for f in &meta.files {
        if opts.only.as_ref().is_none_or(|o| o.contains(&(f.root.clone(), f.rel_path.clone()))) {
            plan.push((f, dest_of(&f.root)?.join(&f.rel_path)));
        }
    }

    // O que vai ser apagado: arquivos do jogo que não estão no backup, só nas raízes
    // restauradas no lugar de sempre, e nunca os que pertencem a outro jogo.
    let mut extras: Vec<Touched> = Vec::new();
    if opts.only.is_none() {
        let template_of = |alias: &str| meta.roots.iter().find(|r| r.alias == alias).map(|r| r.template.as_str());
        let in_snapshot: HashSet<(String, String)> = meta
            .files
            .iter()
            .map(|f| (template_of(&f.root).unwrap_or_default().to_string(), f.rel_path.to_lowercase()))
            .collect();
        let claimed: HashSet<String> = others
            .iter()
            .filter(|o| o.id != game.id)
            .flat_map(|o| o.files.iter().map(|f| key(&f.abs_path)))
            .collect();
        for gf in &game.files {
            let root = &game.save_roots[gf.root];
            let Some(mr) = meta.roots.iter().find(|r| r.template == root.template) else { continue };
            if key(&dest_of(&mr.alias)?) != key(&root.path)
                || in_snapshot.contains(&(root.template.clone(), gf.rel_path.to_lowercase()))
                || claimed.contains(&key(&gf.abs_path))
            {
                continue;
            }
            extras.push((mr.alias.clone(), gf.rel_path.clone(), gf.abs_path.clone()));
        }
    }
    if extras.len() > MAX_EXTRAS && extras.len() > meta.files.len() {
        return Err(Error::UnsafeRestore(format!(
            "ela apagaria {} arquivos que não estão no backup; restaure só alguns arquivos ou em outra pasta",
            extras.len()
        )));
    }

    // Backup de segurança do que existe agora e vai mudar.
    let mut existing: Vec<Touched> = Vec::new();
    for (f, dest) in &plan {
        if dest.is_file() {
            existing.push((f.root.clone(), f.rel_path.clone(), dest.clone()));
        }
    }
    for x in &extras {
        if x.2.is_file() && !existing.iter().any(|e| key(&e.2) == key(&x.2)) {
            existing.push(x.clone());
        }
    }
    let safety = write_safety(ctx, game, &meta, &dests, &existing)?;
    let existed: HashSet<String> = existing.iter().map(|e| key(&e.2)).collect();

    let mut written: Vec<PathBuf> = Vec::new();
    let mut removed = 0;
    let result = (|| -> Result<()> {
        for (f, dest) in &plan {
            write_from_zip(&zip, &f.root, &f.rel_path, dest)?;
            written.push(dest.clone());
        }
        for (_, _, abs) in &extras {
            match std::fs::remove_file(abs) {
                Ok(()) => removed += 1,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(io(abs)(e)),
            }
        }
        for (f, dest) in &plan {
            if hash_file(dest)? != f.blake3 {
                return Err(Error::VerifyFailed(dest.clone()));
            }
        }
        Ok(())
    })();

    if let Err(e) = result {
        ctx.hooks.before_rollback();
        if rollback(ctx.vault, safety.as_ref(), &dests, &written, &existed) > 0 {
            return Err(Error::RollbackFailed {
                cause: e.to_string(),
                safety: safety.as_ref().map(|s| s.file_name.clone()).unwrap_or_else(|| "nenhum".into()),
            });
        }
        return Err(e);
    }
    Ok(RestoreReport { written: written.len(), removed, safety })
}
