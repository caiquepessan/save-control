//! Leitura de backups do cofre.

use std::fs::File;
use std::path::Path;

use crate::error::{Error, Result, io};
use crate::vault::format::{FORMAT, SnapshotMeta};
use crate::vault::write::{META_ENTRY, entry_name};

fn open(zip_path: &Path) -> Result<zip::ZipArchive<File>> {
    let file = File::open(zip_path).map_err(io(zip_path))?;
    zip::ZipArchive::new(file).map_err(|e| Error::Parse(format!("zip {}: {e}", zip_path.display())))
}

/// Metadados do backup. Backups de um formato mais novo são recusados.
pub fn read_meta(zip_path: &Path) -> Result<SnapshotMeta> {
    let mut ar = open(zip_path)?;
    let entry = ar.by_name(META_ENTRY).map_err(|e| Error::Parse(format!("zip: {e}")))?;
    let meta: SnapshotMeta =
        serde_json::from_reader(entry).map_err(|e| Error::Parse(format!("{META_ENTRY}: {e}")))?;
    if meta.format > FORMAT {
        return Err(Error::FormatTooNew { found: meta.format, supported: FORMAT });
    }
    Ok(meta)
}

/// Extrai um arquivo do backup para `dest`, em fluxo (quem chama cuida de temporário/troca).
/// O CRC do zip é conferido ao fim da leitura.
pub fn extract_file(zip_path: &Path, root_alias: &str, rel_path: &str, dest: &Path) -> Result<()> {
    let mut ar = open(zip_path)?;
    let mut entry = ar
        .by_name(&entry_name(root_alias, rel_path))
        .map_err(|_| Error::NotFound(format!("{rel_path} em {}", zip_path.display())))?;
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(io(parent))?;
    }
    let mut out = File::create(dest).map_err(io(dest))?;
    std::io::copy(&mut entry, &mut out).map_err(|_| Error::VerifyFailed(zip_path.to_path_buf()))?;
    out.sync_all().map_err(io(dest))
}
