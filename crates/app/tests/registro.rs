//! Registro do Windows (só HKCU, com um registro falso): iniciar com o Windows,
//! registro das notificações e "Remover registros do Windows".

mod common;

use std::path::{Path, PathBuf};

use common::RegistroFalso;
use savecontrol_app::registro::{AUMID_KEY, Integracao, RUN_KEY, RUN_NAME};

struct Cena {
    _t: tempfile::TempDir,
    reg: RegistroFalso,
    exe: PathBuf,
    dados: PathBuf,
}

fn cena() -> Cena {
    let t = tempfile::tempdir().unwrap();
    let exe = t.path().join("App").join("SaveControl.exe");
    let dados = t.path().join("App").join("SaveControl-dados");
    std::fs::create_dir_all(&dados).unwrap();
    Cena { reg: RegistroFalso::default(), exe, dados, _t: t }
}

impl Cena {
    /// `removivel` diz que caminhos começando com "E:" ficam num HD removível.
    fn integracao(&self) -> Integracao {
        Integracao::new(
            Box::new(self.reg.clone()),
            self.exe.clone(),
            self.dados.clone(),
            Box::new(|p: &Path| p.to_string_lossy().starts_with("E:")),
        )
    }

    fn run(&self) -> Option<String> {
        self.reg.valor(RUN_KEY, RUN_NAME)
    }
}

#[test]
fn ligar_iniciar_com_windows_grava_o_run_com_bandeja() {
    let c = cena();
    let mut i = c.integracao();
    assert!(!i.iniciar_com_windows());
    i.definir_iniciar_com_windows(true).unwrap();
    assert_eq!(c.run(), Some(format!("\"{}\" --bandeja", c.exe.display())));
    assert!(i.iniciar_com_windows());
}

#[test]
fn desligar_apaga_a_entrada() {
    let c = cena();
    let mut i = c.integracao();
    i.definir_iniciar_com_windows(true).unwrap();
    i.definir_iniciar_com_windows(false).unwrap();
    assert_eq!(c.run(), None);
    assert!(c.reg.vazio(), "sobrou algo no registro");
}

#[test]
fn exe_movido_corrige_a_entrada() {
    let c = cena();
    c.reg.preencher(RUN_KEY, RUN_NAME, r#""D:\Velho\SaveControl.exe" --bandeja"#);
    let mut i = c.integracao();
    assert_eq!(i.corrigir_inicio(), None, "não é HD removível: corrige sem avisar");
    assert_eq!(c.run(), Some(format!("\"{}\" --bandeja", c.exe.display())));
}

#[test]
fn exe_vindo_de_hd_removivel_avisa() {
    let c = cena();
    c.reg.preencher(RUN_KEY, RUN_NAME, r#""E:\SaveControl\SaveControl.exe" --bandeja"#);
    let mut i = c.integracao();
    let aviso = i.corrigir_inicio().expect("deveria avisar do HD removível");
    assert!(aviso.contains(r"E:\SaveControl\SaveControl.exe"), "{aviso}");
    assert_eq!(c.run(), Some(format!("\"{}\" --bandeja", c.exe.display())));
}

#[test]
fn sem_entrada_nao_cria_nada() {
    let c = cena();
    let mut i = c.integracao();
    assert_eq!(i.corrigir_inicio(), None);
    assert!(c.reg.vazio(), "criou entrada sem o usuário ligar");
}

#[test]
fn entrada_certa_fica_como_esta() {
    let c = cena();
    let mut i = c.integracao();
    i.definir_iniciar_com_windows(true).unwrap();
    assert_eq!(i.corrigir_inicio(), None);
    assert_eq!(c.run(), Some(format!("\"{}\" --bandeja", c.exe.display())));
}

#[test]
fn notificacoes_registram_e_removem() {
    let c = cena();
    let mut i = c.integracao();
    assert!(!i.notificacoes_registradas());
    i.registrar_notificacoes(b"\x89PNG icone").unwrap();
    let icone = c.dados.join("icone-notificacao.png");
    assert_eq!(c.reg.valor(AUMID_KEY, "DisplayName").as_deref(), Some("Save Control"));
    assert_eq!(c.reg.valor(AUMID_KEY, "IconUri"), Some(icone.display().to_string()));
    assert_eq!(std::fs::read(&icone).unwrap(), b"\x89PNG icone");
    assert!(i.notificacoes_registradas());

    i.remover_notificacoes().unwrap();
    assert!(c.reg.vazio());
    assert!(!icone.exists(), "o ícone ficou para trás");
    assert!(!i.notificacoes_registradas());
}

#[test]
fn remover_registros_apaga_tudo() {
    let c = cena();
    let mut i = c.integracao();
    i.definir_iniciar_com_windows(true).unwrap();
    i.registrar_notificacoes(b"png").unwrap();
    i.remover_tudo().unwrap();
    assert!(c.reg.vazio(), "sobrou registro depois de \"Remover registros do Windows\"");
    assert!(!i.iniciar_com_windows() && !i.notificacoes_registradas());
}
