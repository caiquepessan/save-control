//! Para onde restaurar cada raiz de um backup, neste PC.

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::env::SystemPaths;
use crate::error::{Error, Result};
use crate::placeholder::{InstallCandidate, names_in, resolve_root, slash};
use crate::vault::format::{SnapshotMeta, SnapshotRoot};

pub struct TargetCtx<'a> {
    pub sys: &'a SystemPaths,
    pub installs: Vec<InstallCandidate>,
    pub store_user_ids: Vec<String>,
    pub game_dir: Option<PathBuf>,
    /// Escolhas anteriores deste PC: template da raiz → pasta.
    pub remembered: &'a BTreeMap<String, PathBuf>,
}

const INSTALL_KEYS: [&str; 4] = ["base", "root", "game", "storeGameId"];

fn install_bindings(i: &InstallCandidate) -> BTreeMap<String, String> {
    let mut b = BTreeMap::from([
        ("base".to_string(), slash(&i.base)),
        ("root".to_string(), slash(&i.root)),
        ("game".to_string(), i.game.clone()),
    ]);
    if let Some(id) = &i.store_game_id {
        b.insert("storeGameId".to_string(), id.clone());
    }
    b
}

/// Escolhe entre as opções deste PC: a original se existir, senão a única.
fn pick<T: Clone>(options: &[T], is_original: impl Fn(&T) -> bool) -> Option<T> {
    options.iter().find(|o| is_original(o)).cloned().or_else(|| match options {
        [only] => Some(only.clone()),
        _ => None,
    })
}

fn resolve_one(r: &SnapshotRoot, ctx: &TargetCtx) -> Result<PathBuf> {
    let names = names_in(&r.template);
    let uses = |k: &str| names.iter().any(|n| n == k);
    let uses_install = INSTALL_KEYS.iter().any(|k| uses(k));

    let installs: Vec<Option<InstallCandidate>> = if uses_install {
        ctx.installs
            .iter()
            .filter(|i| !uses("storeGameId") || i.store_game_id.is_some())
            .cloned()
            .map(Some)
            .collect()
    } else {
        vec![None]
    };
    let users: Vec<Option<String>> = if uses("storeUserId") {
        ctx.store_user_ids.iter().cloned().map(Some).collect()
    } else {
        vec![None]
    };
    let game_dir = if uses("gameDir") { ctx.game_dir.as_ref().map(|g| slash(g)) } else { None };

    let bindings_for = |i: &Option<InstallCandidate>, u: &Option<String>| {
        let mut b = i.as_ref().map(install_bindings).unwrap_or_default();
        if let Some(u) = u {
            b.insert("storeUserId".to_string(), u.clone());
        }
        if let Some(gd) = &game_dir {
            b.insert("gameDir".to_string(), gd.clone());
        }
        b
    };

    let orig = |k: &str| r.bindings.get(k).map(|v| v.to_lowercase());
    let install = pick(&installs, |i| {
        i.as_ref().is_some_and(|i| Some(slash(&i.base).to_lowercase()) == orig("base"))
    });
    let user = pick(&users, |u| u.as_ref().map(|u| u.to_lowercase()) == orig("storeUserId"));
    let game_dir_ok = !uses("gameDir") || game_dir.is_some();

    if let (Some(i), Some(u), true) = (&install, &user, game_dir_ok) {
        if let Some(p) = resolve_root(&r.template, ctx.sys, &bindings_for(i, u)) {
            return Ok(p);
        }
    }
    let mut candidates = Vec::new();
    if game_dir_ok {
        for i in &installs {
            for u in &users {
                if let Some(p) = resolve_root(&r.template, ctx.sys, &bindings_for(i, u)) {
                    candidates.push(p);
                }
            }
        }
    }
    Err(Error::NeedsTarget { root: r.template.clone(), candidates })
}

/// Destino de cada raiz (alias → pasta): forçado > lembrado > resolvido neste PC.
pub fn resolve_targets(
    meta: &SnapshotMeta,
    ctx: &TargetCtx,
    overrides: &BTreeMap<String, PathBuf>,
) -> Result<Vec<(String, PathBuf)>> {
    meta.roots
        .iter()
        .map(|r| {
            if let Some(p) = overrides.get(&r.alias) {
                return Ok((r.alias.clone(), p.clone()));
            }
            if let Some(p) = ctx.remembered.get(&r.template) {
                return Ok((r.alias.clone(), p.clone()));
            }
            resolve_one(r, ctx).map(|p| (r.alias.clone(), p))
        })
        .collect()
}
