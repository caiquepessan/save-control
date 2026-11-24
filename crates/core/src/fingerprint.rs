//! "Impressão digital" do conteúdo de um save, para comparar com os backups.

use std::collections::HashMap;
use std::path::Path;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result, io};
use crate::fsutil::write_atomic;
use crate::model::Game;
use crate::placeholder::slash;
use crate::vault::format::SnapshotMeta;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FpItem {
    pub root_template: String,
    /// `rel_path` em minúsculas.
    pub rel_key: String,
    pub size: u64,
    pub blake3: String,
}

/// Itens ordenados: dois saves com o mesmo conteúdo têm a mesma impressão.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fingerprint(pub Vec<FpItem>);

impl Fingerprint {
    fn sorted(mut items: Vec<FpItem>) -> Fingerprint {
        items.sort_by(|a, b| (&a.root_template, &a.rel_key).cmp(&(&b.root_template, &b.rel_key)));
        Fingerprint(items)
    }
}

/// Hashes já calculados, por caminho + tamanho + data: não relê arquivo igual.
#[derive(Default, Serialize, Deserialize)]
pub struct HashCache {
    entries: HashMap<String, String>,
}

impl HashCache {
    pub fn load(path: &Path) -> HashCache {
        std::fs::read_to_string(path).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_vec(self).map_err(|e| Error::Parse(e.to_string()))?;
        write_atomic(path, &json)
    }

    pub fn hash(&mut self, abs: &Path, size: u64, modified: DateTime<Utc>) -> Result<String> {
        let nanos = modified.timestamp_nanos_opt().unwrap_or_else(|| modified.timestamp());
        let key = format!("{}|{size}|{nanos}", slash(abs).to_lowercase());
        if let Some(h) = self.entries.get(&key) {
            return Ok(h.clone());
        }
        let mut file = std::fs::File::open(abs).map_err(io(abs))?;
        let mut hasher = blake3::Hasher::new();
        std::io::copy(&mut file, &mut hasher).map_err(io(abs))?;
        let h = hasher.finalize().to_hex().to_string();
        self.entries.insert(key, h.clone());
        Ok(h)
    }
}

pub fn fingerprint_of_game(game: &Game, cache: &mut HashCache) -> Result<Fingerprint> {
    let mut items = Vec::with_capacity(game.files.len());
    for f in &game.files {
        items.push(FpItem {
            root_template: game.save_roots[f.root].template.clone(),
            rel_key: f.rel_path.to_lowercase(),
            size: f.size,
            blake3: cache.hash(&f.abs_path, f.size, f.modified)?,
        });
    }
    Ok(Fingerprint::sorted(items))
}

pub fn fingerprint_of_meta(meta: &SnapshotMeta) -> Fingerprint {
    let items = meta
        .files
        .iter()
        .map(|f| FpItem {
            root_template: meta
                .roots
                .iter()
                .find(|r| r.alias == f.root)
                .map(|r| r.template.clone())
                .unwrap_or_default(),
            rel_key: f.rel_path.to_lowercase(),
            size: f.size,
            blake3: f.blake3.clone(),
        })
        .collect();
    Fingerprint::sorted(items)
}
