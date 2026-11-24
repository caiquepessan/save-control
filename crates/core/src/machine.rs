//! Identidade deste PC (e usuário do Windows) dentro do cofre.

use serde::{Deserialize, Serialize};

use crate::registry::{Hive, RegistryReader};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MachineIdentity {
    /// 16 hex estáveis para este PC + usuário.
    pub id: String,
    pub pc_name: String,
    pub user_name: String,
}

impl MachineIdentity {
    pub fn detect(reg: &dyn RegistryReader) -> MachineIdentity {
        let pc_name = std::env::var("COMPUTERNAME").unwrap_or_else(|_| "PC".into());
        let user_name = std::env::var("USERNAME").unwrap_or_default();
        let guid = reg.string(Hive::LocalMachine, r"SOFTWARE\Microsoft\Cryptography", "MachineGuid");
        let seed = match guid {
            Some(g) => format!("{g}|{user_name}"),
            None => format!("{pc_name}|{user_name}"),
        };
        let id = blake3::hash(seed.as_bytes()).to_hex()[..16].to_string();
        MachineIdentity { id, pc_name, user_name }
    }

    /// Rótulo curto usado no nome dos arquivos do cofre: `NOMEDOPC-ABCD`.
    pub fn label(&self) -> String {
        let mut name: String =
            self.pc_name.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').take(15).collect();
        if name.is_empty() {
            name = "PC".into();
        }
        let tag: String = self.id.chars().take(4).collect();
        format!("{name}-{}", tag.to_uppercase())
    }
}
