//! Índice local do cofre: cache do que há em cada `.zip`, reconstruível a qualquer hora.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::fingerprint::{Fingerprint, fingerprint_of_meta};
use crate::fsutil::write_atomic;
use crate::vault::Vault;
use crate::vault::format::{SnapshotKind, SnapshotMeta};
use crate::vault::read::read_meta;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SnapshotEntry {
    pub game_id: String,
    pub dir_name: String,
    pub file_name: String,
    pub kind: SnapshotKind,
    pub label: Option<String>,
    pub created_at: DateTime<Utc>,
    pub machine_id: String,
    pub machine_label: String,
    pub pc_name: String,
    pub fingerprint: Fingerprint,
    pub total_size: u64,
    pub file_count: usize,
}

impl SnapshotEntry {
    pub fn zip_path(&self, vault: &Vault) -> PathBuf {
        vault.root.join(&self.dir_name).join(&self.file_name)
    }
}

pub fn entry_from_meta(meta: &SnapshotMeta, dir_name: &str, file_name: &str, kind: SnapshotKind) -> SnapshotEntry {
    SnapshotEntry {
        game_id: meta.game_id.clone(),
        dir_name: dir_name.to_string(),
        file_name: file_name.to_string(),
        kind,
        label: meta.label.clone(),
        created_at: meta.created_at,
        machine_id: meta.machine.id.clone(),
        machine_label: meta.machine.label(),
        pc_name: meta.machine.pc_name.clone(),
        fingerprint: fingerprint_of_meta(meta),
        total_size: meta.files.iter().map(|f| f.size).sum(),
        file_count: meta.files.len(),
    }
}

#[derive(Default, Serialize, Deserialize)]
pub struct VaultIndex {
    pub vault_id: String,
    pub entries: Vec<SnapshotEntry>,
}

impl VaultIndex {
    /// Índice salvo deste cofre; de outro cofre, ausente ou inválido → vazio.
    pub fn load(path: &Path, vault_id: &str) -> VaultIndex {
        let idx: Option<VaultIndex> =
            std::fs::read_to_string(path).ok().and_then(|t| serde_json::from_str(&t).ok());
        match idx {
            Some(i) if i.vault_id == vault_id => i,
            _ => VaultIndex { vault_id: vault_id.to_string(), entries: Vec::new() },
        }
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_vec(self).map_err(|e| Error::Parse(e.to_string()))?;
        write_atomic(path, &json)
    }

    /// Lê só os zips novos; ignora `.parcial` e nomes fora do padrão; tira os apagados.
    pub fn refresh(&mut self, vault: &Vault) -> Result<()> {
        self.vault_id.clone_from(&vault.id);
        let mut seen: Vec<(String, String)> = Vec::new();
        if let Ok(dirs) = std::fs::read_dir(&vault.root) {
            for dir in dirs.flatten().filter(|d| d.path().is_dir()) {
                let dir_name = dir.file_name().to_string_lossy().into_owned();
                let Ok(files) = std::fs::read_dir(dir.path()) else { continue };
                for f in files.flatten() {
                    let file_name = f.file_name().to_string_lossy().into_owned();
                    let Some(kind) = SnapshotKind::from_file_name(&file_name) else { continue };
                    seen.push((dir_name.clone(), file_name.clone()));
                    if self.entries.iter().any(|e| e.dir_name == dir_name && e.file_name == file_name) {
                        continue;
                    }
                    if let Ok(meta) = read_meta(&f.path()) {
                        self.entries.push(entry_from_meta(&meta, &dir_name, &file_name, kind));
                    }
                }
            }
        }
        self.entries.retain(|e| seen.iter().any(|(d, f)| *d == e.dir_name && *f == e.file_name));
        Ok(())
    }

    /// Backups de um jogo, do mais novo para o mais velho.
    pub fn for_game(&self, game_id: &str) -> Vec<&SnapshotEntry> {
        let mut v: Vec<&SnapshotEntry> = self.entries.iter().filter(|e| e.game_id == game_id).collect();
        v.sort_by(|a, b| b.created_at.cmp(&a.created_at).then_with(|| b.file_name.cmp(&a.file_name)));
        v
    }

    pub fn find_identical(&self, game_id: &str, fp: &Fingerprint) -> Option<&SnapshotEntry> {
        self.for_game(game_id).into_iter().find(|e| e.fingerprint == *fp)
    }

    pub fn insert(&mut self, e: SnapshotEntry) {
        self.remove(&e.dir_name.clone(), &e.file_name.clone());
        self.entries.push(e);
    }

    pub fn remove(&mut self, dir_name: &str, file_name: &str) {
        self.entries.retain(|e| !(e.dir_name == dir_name && e.file_name == file_name));
    }
}

impl Vault {
    /// Fixa um backup automático: vira permanente (renomeia `_auto.zip` → `_fixado.zip`).
    pub fn pin(&self, dir_name: &str, file_name: &str) -> Result<String> {
        let auto = format!("_{}.zip", SnapshotKind::Auto.suffix());
        let stem = file_name
            .strip_suffix(&auto)
            .ok_or_else(|| Error::NotFound(format!("backup automático {file_name}")))?;
        let new_name = format!("{stem}_{}.zip", SnapshotKind::Pinned.suffix());
        let dir = self.root.join(dir_name);
        std::fs::rename(dir.join(file_name), dir.join(&new_name)).map_err(crate::error::io(dir.join(file_name)))?;
        Ok(new_name)
    }

    pub fn delete_snapshot(&self, dir_name: &str, file_name: &str) -> Result<()> {
        let path = self.root.join(dir_name).join(file_name);
        std::fs::remove_file(&path).map_err(crate::error::io(&path))
    }
}
