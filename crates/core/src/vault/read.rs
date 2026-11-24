//! Leitura de backups do cofre.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use crate::error::{Error, Result, io};
use crate::vault::format::SnapshotMeta;
use crate::vault::write::{META_ENTRY, entry_name};

fn open(zip_path: &Path) -> Result<zip::ZipArchive<File>> {
    let file = File::open(zip_path).map_err(io(zip_path))?;
    zip::ZipArchive::new(file).map_err(|e| Error::Parse(format!("zip {}: {e}", zip_path.display())))
}

pub fn read_meta(zip_path: &Path) -> Result<SnapshotMeta> {
    let mut ar = open(zip_path)?;
    let entry = ar.by_name(META_ENTRY).map_err(|e| Error::Parse(format!("zip: {e}")))?;
    serde_json::from_reader(entry).map_err(|e| Error::Parse(format!("{META_ENTRY}: {e}")))
}

/// Extrai um arquivo do backup para `dest` (quem chama cuida de temporário/troca).
pub fn extract_file(zip_path: &Path, root_alias: &str, rel_path: &str, dest: &Path) -> Result<()> {
    let mut ar = open(zip_path)?;
    let mut entry = ar
        .by_name(&entry_name(root_alias, rel_path))
        .map_err(|_| Error::NotFound(format!("{rel_path} em {}", zip_path.display())))?;
    let mut buf = Vec::new();
    entry.read_to_end(&mut buf).map_err(|_| Error::VerifyFailed(zip_path.to_path_buf()))?;
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(io(parent))?;
    }
    std::fs::write(dest, buf).map_err(io(dest))
}
