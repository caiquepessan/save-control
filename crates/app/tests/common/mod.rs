#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use savecontrol_app::paths::resolve_paths;
use savecontrol_app::registro::Registro;
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
    servico_em(w, &w.exe, busy)
}

/// O serviço com o `.exe` em outra pasta (ex.: o HD em outra letra).
pub fn servico_em(w: &Mundo, exe: &Path, busy: BusyFlag) -> AppService {
    AppService::new(resolve_paths(exe, &w.lad), w.sys.clone(), Box::new(registro), busy)
}

/// Move a pasta do app inteira (o `.exe`, os dados e o cofre ao lado) para outro lugar.
pub fn mover_app(w: &Mundo) -> PathBuf {
    let novo = w.t.path().join("OutraLetra").join("App");
    std::fs::create_dir_all(novo.parent().unwrap()).unwrap();
    std::fs::rename(&w.exe, &novo).unwrap();
    novo
}

/// Serviço já passado pelo primeiro uso, com o cofre padrão ao lado do `.exe`.
pub fn pronto(w: &Mundo) -> AppService {
    let mut s = servico(w, BusyFlag::default());
    s.finish_first_run(&w.exe.join("cofre")).unwrap();
    s
}

/// Registro do Windows de mentira (HKCU), compartilhado entre o teste e o app.
#[derive(Clone, Default)]
pub struct RegistroFalso(Arc<Mutex<BTreeMap<(String, String), String>>>);

impl RegistroFalso {
    pub fn valor(&self, chave: &str, nome: &str) -> Option<String> {
        self.0.lock().unwrap().get(&(chave.to_string(), nome.to_string())).cloned()
    }

    pub fn preencher(&self, chave: &str, nome: &str, valor: &str) {
        self.0.lock().unwrap().insert((chave.to_string(), nome.to_string()), valor.to_string());
    }

    pub fn vazio(&self) -> bool {
        self.0.lock().unwrap().is_empty()
    }
}

impl Registro for RegistroFalso {
    fn ler(&self, chave: &str, nome: &str) -> Option<String> {
        self.valor(chave, nome)
    }

    fn gravar(&mut self, chave: &str, nome: &str, valor: &str) -> std::io::Result<()> {
        self.preencher(chave, nome, valor);
        Ok(())
    }

    fn apagar_valor(&mut self, chave: &str, nome: &str) -> std::io::Result<()> {
        self.0.lock().unwrap().remove(&(chave.to_string(), nome.to_string()));
        Ok(())
    }

    fn apagar_chave(&mut self, chave: &str) -> std::io::Result<()> {
        self.0.lock().unwrap().retain(|(c, _), _| c != chave);
        Ok(())
    }
}
