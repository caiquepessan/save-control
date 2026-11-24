#![allow(dead_code)]

use std::path::{Path, PathBuf};

use savecontrol_app::paths::resolve_paths;
use savecontrol_app::service::{AppService, BusyFlag};
use savecontrol_core::env::SystemPaths;
use savecontrol_core::registry::{FakeRegistry, Hive, RegistryReader};

/// Grava um arquivo criando as pastas que faltarem.
pub fn write(path: &Path, bytes: &[u8]) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
}

pub fn ler(path: &Path) -> String {
    String::from_utf8_lossy(&std::fs::read(path).unwrap()).into_owned()
}

/// Uma "home" falsa, a pasta do `.exe` e um `%LOCALAPPDATA%` falso.
pub struct Mundo {
    pub t: tempfile::TempDir,
    pub sys: SystemPaths,
    pub exe: PathBuf,
    pub lad: PathBuf,
}

pub fn mundo() -> Mundo {
    let t = tempfile::tempdir().unwrap();
    let sys = SystemPaths::under_home(&t.path().join("home"), "teste");
    let exe = t.path().join("App");
    std::fs::create_dir_all(&exe).unwrap();
    let lad = t.path().join("lad");
    Mundo { t, sys, exe, lad }
}

fn registro() -> Box<dyn RegistryReader + Send> {
    let mut reg = FakeRegistry::default();
    reg.set(Hive::LocalMachine, r"SOFTWARE\Microsoft\Cryptography", "MachineGuid", "guid-teste");
    Box::new(reg)
}

pub fn servico(w: &Mundo, busy: BusyFlag) -> AppService {
    AppService::new(resolve_paths(&w.exe, &w.lad), w.sys.clone(), Box::new(registro), busy)
}

/// Serviço já passado pelo primeiro uso, com o cofre padrão ao lado do `.exe`.
pub fn pronto(w: &Mundo) -> AppService {
    let mut s = servico(w, BusyFlag::default());
    s.finish_first_run(&w.exe.join("cofre")).unwrap();
    s
}
