//! O cofre num HD que foi desligado: abrir nunca cria nada no lugar dele, e o
//! índice guardado não é esvaziado enquanto o cofre está fora.
mod common;

use common::write;
use savecontrol_core::{
    engine::{Engine, EngineConfig},
    env::SystemPaths,
    error::Error,
    machine::MachineIdentity,
    registry::FakeRegistry,
    vault::{Vault, format::*, index::VaultIndex, write::*},
};

fn meta() -> SnapshotMeta {
    SnapshotMeta {
        format: FORMAT,
        game_id: "pc:Jogo A".into(),
        title: "Jogo A".into(),
        kind: SnapshotKind::Auto,
        label: None,
        created_at: chrono::Utc::now(),
        app_version: "0.1.0".into(),
        machine: MachineIdentity { id: "aaaa111122223333".into(), pc_name: "PC".into(), user_name: "teste".into() },
        roots: vec![SnapshotRoot {
            alias: "r0".into(),
            template: "<winAppData>/JogoA".into(),
            bindings: Default::default(),
            original_path: "C:/x".into(),
        }],
        files: vec![],
    }
}

#[test]
fn abrir_cofre_que_sumiu_nao_cria_nada() {
    let t = tempfile::tempdir().unwrap();
    let hd = t.path().join("HD");
    let r = Vault::open(&hd.join("cofre"));
    assert!(matches!(r, Err(Error::VaultUnavailable(_))), "{:?}", r.err());
    assert!(!hd.exists(), "abrir criou pasta no lugar do cofre sumido");
}

#[test]
fn criar_e_abrir_o_mesmo_cofre() {
    let t = tempfile::tempdir().unwrap();
    let d = t.path().join("cofre");
    let a = Vault::create(&d).unwrap();
    let b = Vault::create(&d).unwrap();
    assert_eq!(a.id, b.id, "criar de novo trocou o id do cofre");
    assert_eq!(Vault::open(&d).unwrap().id, a.id);
}

#[test]
fn indice_nao_esvazia_com_o_cofre_fora() {
    let t = tempfile::tempdir().unwrap();
    let src = t.path().join("src");
    write(&src.join("slot1.sav"), b"um");
    let cofre = t.path().join("cofre");
    let v = Vault::create(&cofre).unwrap();
    let dir = v.game_dir("pc:Jogo A", "Jogo A").unwrap();
    let sources = [SourceFile { abs_path: src.join("slot1.sav"), root_alias: "r0".into(), rel_path: "slot1.sav".into() }];
    write_snapshot(&dir, "2026-11-24_10-00-00_PC-AAAA_auto.zip", meta(), &sources, &NoWait).unwrap();
    let mut idx = VaultIndex::default();
    idx.refresh(&v).unwrap();
    assert_eq!(idx.entries.len(), 1);

    std::fs::rename(&cofre, t.path().join("cofre-no-hd-desligado")).unwrap();
    let r = idx.refresh(&v);
    assert!(matches!(r, Err(Error::VaultUnavailable(_))), "{r:?}");
    assert_eq!(idx.entries.len(), 1, "o índice foi esvaziado com o cofre fora");
}

#[test]
fn engine_nao_abre_nem_cria_cofre_sumido() {
    let t = tempfile::tempdir().unwrap();
    let s = SystemPaths::under_home(&t.path().join("home"), "teste");
    let sumido = t.path().join("HD").join("cofre");
    let cfg = EngineConfig { data_dir: t.path().join("dados"), vault_dir: sumido.clone(), include_config: false };
    let r = Engine::open(cfg, s, Box::new(FakeRegistry::default()));
    assert!(matches!(r, Err(Error::VaultUnavailable(_))), "{:?}", r.err());
    assert!(!t.path().join("HD").exists(), "a Engine criou o cofre sumido");
}
