#![allow(dead_code)]

use std::path::Path;

/// Grava um arquivo criando as pastas que faltarem.
pub fn write(path: &Path, bytes: &[u8]) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
}
