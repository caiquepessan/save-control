#![allow(dead_code)]

use std::path::Path;

use chrono::{DateTime, Local, TimeZone};
use savecontrol_core::env::SystemPaths;

/// Uma "home" falsa dentro de uma pasta temporária, com o layout padrão do Windows.
pub fn fake_home() -> (tempfile::TempDir, SystemPaths) {
    let tmp = tempfile::tempdir().unwrap();
    let sys = SystemPaths::under_home(&tmp.path().join("home"), "teste");
    (tmp, sys)
}

/// Grava um arquivo criando as pastas que faltarem.
pub fn write(path: &Path, bytes: &[u8]) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
}

/// Data e hora locais fixas para testes.
pub fn local(y: i32, m: u32, d: u32, h: u32, mi: u32, s: u32) -> DateTime<Local> {
    Local.with_ymd_and_hms(y, m, d, h, mi, s).unwrap()
}
