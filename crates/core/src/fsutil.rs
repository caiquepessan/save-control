//! Utilidades de arquivo compartilhadas.

use std::path::{Path, PathBuf};

use crate::error::{Result, io};

/// Grava num temporário ao lado e renomeia por cima (troca atômica no mesmo disco).
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(io(parent))?;
    }
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    std::fs::write(&tmp, bytes).map_err(io(&tmp))?;
    std::fs::rename(&tmp, path).map_err(io(path))
}
