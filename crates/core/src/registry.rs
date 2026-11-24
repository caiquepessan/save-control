//! Leitura do registro do Windows atrás de uma trait, para os testes.

use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Hive {
    CurrentUser,
    LocalMachine,
}

pub trait RegistryReader {
    fn string(&self, hive: Hive, key: &str, value: &str) -> Option<String>;
    fn subkeys(&self, hive: Hive, key: &str) -> Vec<String>;
}

/// O registro de verdade.
pub struct WinRegistry;

#[cfg(windows)]
impl WinRegistry {
    fn open(hive: Hive, key: &str) -> Option<winreg::RegKey> {
        use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
        let root = winreg::RegKey::predef(match hive {
            Hive::CurrentUser => HKEY_CURRENT_USER,
            Hive::LocalMachine => HKEY_LOCAL_MACHINE,
        });
        root.open_subkey(key).ok()
    }
}

#[cfg(windows)]
impl RegistryReader for WinRegistry {
    fn string(&self, hive: Hive, key: &str, value: &str) -> Option<String> {
        Self::open(hive, key)?.get_value::<String, _>(value).ok()
    }

    fn subkeys(&self, hive: Hive, key: &str) -> Vec<String> {
        Self::open(hive, key).map(|k| k.enum_keys().flatten().collect()).unwrap_or_default()
    }
}

#[cfg(not(windows))]
impl RegistryReader for WinRegistry {
    fn string(&self, _: Hive, _: &str, _: &str) -> Option<String> {
        None
    }

    fn subkeys(&self, _: Hive, _: &str) -> Vec<String> {
        Vec::new()
    }
}

/// Registro em memória, para testes e para simular outro PC.
#[derive(Default)]
pub struct FakeRegistry {
    values: BTreeMap<(Hive, String, String), String>,
    keys: BTreeMap<(Hive, String), Vec<String>>,
}

impl FakeRegistry {
    pub fn set(&mut self, hive: Hive, key: &str, value: &str, data: &str) {
        self.values.insert((hive, key.to_lowercase(), value.to_lowercase()), data.to_string());
    }

    pub fn add_subkey(&mut self, hive: Hive, key: &str, sub: &str) {
        self.keys.entry((hive, key.to_lowercase())).or_default().push(sub.to_string());
    }
}

impl RegistryReader for FakeRegistry {
    fn string(&self, hive: Hive, key: &str, value: &str) -> Option<String> {
        self.values.get(&(hive, key.to_lowercase(), value.to_lowercase())).cloned()
    }

    fn subkeys(&self, hive: Hive, key: &str) -> Vec<String> {
        self.keys.get(&(hive, key.to_lowercase())).cloned().unwrap_or_default()
    }
}
