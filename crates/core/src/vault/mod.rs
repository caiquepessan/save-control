//! O cofre: uma pasta com uma subpasta por jogo e um `.zip` por backup.

pub mod format;
pub mod read;
pub mod write;

use std::path::{Path, PathBuf};

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result, io};
use crate::fsutil::write_atomic;
use crate::machine::MachineIdentity;
use crate::vault::format::{FORMAT, GameDescriptor, sanitize_folder_name};

const VAULT_FILE: &str = "cofre.json";
const GAME_FILE: &str = "jogo.json";
pub(crate) const PARTIAL_SUFFIX: &str = ".parcial";

#[derive(Serialize, Deserialize)]
struct VaultFile {
    format: u32,
    id: String,
}

pub struct Vault {
    pub root: PathBuf,
    pub id: String,
}

impl Vault {
    pub fn open_or_create(root: &Path) -> Result<Vault> {
        let file = root.join(VAULT_FILE);
        if file.is_file() {
            let text = std::fs::read_to_string(&file).map_err(io(&file))?;
            let v: VaultFile = serde_json::from_str(&text).map_err(|e| Error::Parse(format!("cofre.json: {e}")))?;
            return Ok(Vault { root: root.to_path_buf(), id: v.id });
        }
        std::fs::create_dir_all(root).map_err(io(root))?;
        let v = VaultFile { format: FORMAT, id: uuid::Uuid::new_v4().simple().to_string() };
        let json = serde_json::to_vec_pretty(&v).map_err(|e| Error::Parse(e.to_string()))?;
        write_atomic(&file, &json)?;
        Ok(Vault { root: root.to_path_buf(), id: v.id })
    }

    /// O cofre continua no lugar? Nunca cria nada.
    pub fn ensure_available(&self) -> Result<()> {
        if self.root.is_dir() && self.root.join(VAULT_FILE).is_file() {
            Ok(())
        } else {
            Err(Error::VaultUnavailable(self.root.clone()))
        }
    }

    pub fn read_descriptor(dir: &Path) -> Option<GameDescriptor> {
        let text = std::fs::read_to_string(dir.join(GAME_FILE)).ok()?;
        serde_json::from_str(&text).ok()
    }

    /// Pasta do jogo no cofre (criada se faltar). Se o nome já pertence a outro
    /// jogo, usa `"<nome> (<6 hex do id>)"`.
    pub fn game_dir(&self, game_id: &str, title: &str) -> Result<PathBuf> {
        let base = sanitize_folder_name(title);
        let hash = blake3::hash(game_id.as_bytes()).to_hex().to_string();
        let names = [base.clone(), format!("{base} ({})", &hash[..6]), format!("{base} ({hash})")];
        for name in names {
            let dir = self.root.join(&name);
            match Self::read_descriptor(&dir) {
                Some(d) if d.game_id != game_id => continue,
                _ => {
                    std::fs::create_dir_all(&dir).map_err(io(&dir))?;
                    return Ok(dir);
                }
            }
        }
        Err(Error::Parse(format!("não foi possível criar a pasta de {title} no cofre")))
    }

    pub fn write_descriptor(&self, dir: &Path, d: &GameDescriptor) -> Result<()> {
        let json = serde_json::to_vec_pretty(d).map_err(|e| Error::Parse(e.to_string()))?;
        write_atomic(&dir.join(GAME_FILE), &json)
    }

    /// Apaga `.zip.parcial` deste PC, ou de qualquer PC com mais de 24 h
    /// (outro PC pode estar gravando agora).
    pub fn cleanup_partials(&self, machine: &MachineIdentity, now: DateTime<Utc>) -> Result<usize> {
        let tag = format!("_{}_", machine.label());
        let mut removed = 0;
        let Ok(dirs) = std::fs::read_dir(&self.root) else { return Ok(0) };
        for dir in dirs.flatten().filter(|d| d.path().is_dir()) {
            let Ok(files) = std::fs::read_dir(dir.path()) else { continue };
            for f in files.flatten() {
                let name = f.file_name().to_string_lossy().into_owned();
                if !name.ends_with(&format!(".zip{PARTIAL_SUFFIX}")) {
                    continue;
                }
                let old = f
                    .metadata()
                    .and_then(|m| m.modified())
                    .map(|t| now - DateTime::<Utc>::from(t) > Duration::hours(24))
                    .unwrap_or(false);
                if (name.contains(&tag) || old) && std::fs::remove_file(f.path()).is_ok() {
                    removed += 1;
                }
            }
        }
        Ok(removed)
    }
}
