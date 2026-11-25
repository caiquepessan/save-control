//! As duas únicas entradas que o app cria no registro do Windows (spec §2, §13),
//! sempre em HKCU e só quando o usuário liga:
//!
//! - "Iniciar com o Windows": valor `Save Control` em `...\CurrentVersion\Run`;
//! - notificações: a chave `AppUserModelId\SaveControl.Portatil` (nome e ícone).
//!
//! "Remover registros do Windows" apaga as duas.

use std::path::{Path, PathBuf};

use savecontrol_core::fsutil::write_atomic;

use crate::texts;

pub const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
pub const RUN_NAME: &str = "Save Control";
pub const AUMID: &str = "SaveControl.Portatil";
pub const AUMID_KEY: &str = r"Software\Classes\AppUserModelId\SaveControl.Portatil";

const ICONE: &str = "icone-notificacao.png";

/// Registro do usuário (HKCU). A versão de verdade usa o `winreg`; os testes, uma falsa.
pub trait Registro: Send {
    fn ler(&self, chave: &str, nome: &str) -> Option<String>;
    fn gravar(&mut self, chave: &str, nome: &str, valor: &str) -> std::io::Result<()>;
    /// Apagar o que não existe não é erro.
    fn apagar_valor(&mut self, chave: &str, nome: &str) -> std::io::Result<()>;
    /// Apaga a chave e o que houver dentro dela (nunca acima dela).
    fn apagar_chave(&mut self, chave: &str) -> std::io::Result<()>;
}

pub struct WinRegistro;

#[cfg(windows)]
impl Registro for WinRegistro {
    fn ler(&self, chave: &str, nome: &str) -> Option<String> {
        winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER).open_subkey(chave).ok()?.get_value(nome).ok()
    }

    fn gravar(&mut self, chave: &str, nome: &str, valor: &str) -> std::io::Result<()> {
        let (k, _) = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER).create_subkey(chave)?;
        k.set_value(nome, &valor)
    }

    fn apagar_valor(&mut self, chave: &str, nome: &str) -> std::io::Result<()> {
        let hkcu = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
        let k = match hkcu.open_subkey_with_flags(chave, winreg::enums::KEY_SET_VALUE) {
            Ok(k) => k,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e),
        };
        match k.delete_value(nome) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        }
    }

    fn apagar_chave(&mut self, chave: &str) -> std::io::Result<()> {
        match winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER).delete_subkey_all(chave) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        }
    }
}

#[cfg(not(windows))]
impl Registro for WinRegistro {
    fn ler(&self, _chave: &str, _nome: &str) -> Option<String> {
        None
    }

    fn gravar(&mut self, _chave: &str, _nome: &str, _valor: &str) -> std::io::Result<()> {
        Err(std::io::Error::other("registro do Windows indisponível"))
    }

    fn apagar_valor(&mut self, _chave: &str, _nome: &str) -> std::io::Result<()> {
        Ok(())
    }

    fn apagar_chave(&mut self, _chave: &str) -> std::io::Result<()> {
        Ok(())
    }
}

/// O caminho do `.exe` dentro de um valor do Run (`"C:\...\SaveControl.exe" --bandeja`).
fn exe_do_valor(valor: &str) -> PathBuf {
    let v = valor.trim();
    match v.strip_prefix('"').and_then(|r| r.split_once('"')) {
        Some((caminho, _)) => PathBuf::from(caminho),
        None => PathBuf::from(v.split_whitespace().next().unwrap_or(v)),
    }
}

/// A unidade do caminho é um HD removível, ou não existe mais (HD desconectado).
pub fn unidade_removivel(path: &Path) -> bool {
    #[cfg(windows)]
    {
        use std::path::Component;
        // Valores documentados de GetDriveTypeW.
        const DRIVE_NO_ROOT_DIR: u32 = 1;
        const DRIVE_REMOVABLE: u32 = 2;
        let Some(Component::Prefix(p)) = path.components().next() else { return false };
        let raiz: Vec<u16> = format!("{}\\", p.as_os_str().to_string_lossy()).encode_utf16().chain([0]).collect();
        let tipo = unsafe { windows_sys::Win32::Storage::FileSystem::GetDriveTypeW(raiz.as_ptr()) };
        tipo == DRIVE_REMOVABLE || tipo == DRIVE_NO_ROOT_DIR
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        false
    }
}

pub struct Integracao {
    reg: Box<dyn Registro>,
    exe: PathBuf,
    data_dir: PathBuf,
    removivel: Box<dyn Fn(&Path) -> bool + Send>,
}

impl Integracao {
    pub fn new(
        reg: Box<dyn Registro>,
        exe: PathBuf,
        data_dir: PathBuf,
        removivel: Box<dyn Fn(&Path) -> bool + Send>,
    ) -> Integracao {
        Integracao { reg, exe, data_dir, removivel }
    }

    fn valor_run(&self) -> String {
        format!("\"{}\" --bandeja", self.exe.display())
    }

    pub fn iniciar_com_windows(&self) -> bool {
        self.reg.ler(RUN_KEY, RUN_NAME).is_some()
    }

    pub fn definir_iniciar_com_windows(&mut self, ligado: bool) -> std::io::Result<()> {
        if ligado {
            let valor = self.valor_run();
            self.reg.gravar(RUN_KEY, RUN_NAME, &valor)
        } else {
            self.reg.apagar_valor(RUN_KEY, RUN_NAME)
        }
    }

    /// Ao abrir: se o "Iniciar com o Windows" aponta para outro `.exe` (o app
    /// mudou de lugar), passa a apontar para este. Devolve um aviso quando o
    /// caminho antigo era de um HD removível. Sem a entrada, não cria nada.
    pub fn corrigir_inicio(&mut self) -> Option<String> {
        let atual = self.reg.ler(RUN_KEY, RUN_NAME)?;
        let esperado = self.valor_run();
        if atual == esperado {
            return None;
        }
        let antigo = exe_do_valor(&atual);
        let _ = self.reg.gravar(RUN_KEY, RUN_NAME, &esperado);
        (self.removivel)(&antigo).then(|| texts::inicio_removivel(&antigo.display().to_string()))
    }

    pub fn notificacoes_registradas(&self) -> bool {
        self.reg.ler(AUMID_KEY, "DisplayName").is_some()
    }

    /// Registra o nome e o ícone do app para as notificações do Windows (só
    /// quando o usuário liga as notificações).
    pub fn registrar_notificacoes(&mut self, icone_png: &[u8]) -> std::io::Result<()> {
        let icone = self.data_dir.join(ICONE);
        write_atomic(&icone, icone_png).map_err(std::io::Error::other)?;
        self.reg.gravar(AUMID_KEY, "DisplayName", "Save Control")?;
        self.reg.gravar(AUMID_KEY, "IconUri", &icone.display().to_string())
    }

    pub fn remover_notificacoes(&mut self) -> std::io::Result<()> {
        self.reg.apagar_chave(AUMID_KEY)?;
        match std::fs::remove_file(self.data_dir.join(ICONE)) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        }
    }

    /// "Remover registros do Windows" (Configurações).
    pub fn remover_tudo(&mut self) -> std::io::Result<()> {
        self.definir_iniciar_com_windows(false)?;
        self.remover_notificacoes()
    }
}
