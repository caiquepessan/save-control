mod common;

use chrono::{DateTime, Duration, Local};
use common::{local, write};
use savecontrol_core::{
    backup::*,
    env::SystemPaths,
    error::Error,
    fingerprint::{HashCache, fingerprint_of_game},
    local_state::LocalState,
    machine::MachineIdentity,
    manifest::Manifest,
    model::Game,
    scan::{ScanInput, scan},
    status::{GameStatus, status_of},
    stores::Installs,
    vault::{Vault, format::SnapshotKind, index::VaultIndex, retention::RetentionPolicy, write::NoWait},
};

struct Mundo {
    t: tempfile::TempDir,
    s: SystemPaths,
    vault: Vault,
    index: VaultIndex,
    hashes: HashCache,
    pc: MachineIdentity,
}

fn mundo() -> Mundo {
    let t = tempfile::tempdir().unwrap();
    let s = SystemPaths::under_home(&t.path().join("home"), "teste");
    let vault = Vault::create(&t.path().join("cofre")).unwrap();
    let pc = MachineIdentity { id: "aaaa111122223333".into(), pc_name: "PC".into(), user_name: "teste".into() };
    Mundo { t, s, vault, index: VaultIndex::default(), hashes: HashCache::default(), pc }
}

fn salvar(w: &Mundo, conteudo: &[u8]) {
    write(&w.s.app_data.join("JogoA/slot1.sav"), conteudo);
}

fn escanear(w: &Mundo) -> Game {
    let m = Manifest::from_yaml("Jogo A:\n  files:\n    <winAppData>/JogoA:\n      tags: [save]\n").unwrap();
    let input = ScanInput {
        sys: &w.s,
        manifest: Some(&m),
        installs: &Installs::default(),
        catalog: &[],
        local: &LocalState::default(),
        include_config: false,
    };
    scan(&input).into_iter().find(|g| g.id == "pc:Jogo A").unwrap()
}

fn backup(w: &mut Mundo, g: &Game, kind: SnapshotKind, label: Option<&str>, now: DateTime<Local>) -> Result<BackupOutcome, Error> {
    let policy = RetentionPolicy::default();
    let mut ctx = BackupCtx {
        vault: &w.vault,
        index: &mut w.index,
        hashes: &mut w.hashes,
        machine: &w.pc,
        hooks: &NoWait,
        now,
        app_version: "0.1.0",
        retention: &policy,
    };
    backup_game(&mut ctx, g, kind, label)
}

fn status(w: &mut Mundo, g: &Game) -> GameStatus {
    let fp = fingerprint_of_game(g, &mut w.hashes).unwrap();
    status_of(g, &fp, &w.index)
}

fn agora() -> DateTime<Local> {
    local(2026, 11, 25, 8, 0, 0)
}

#[test]
fn primeiro_backup_protege() {
    let mut w = mundo();
    salvar(&w, b"v1");
    let g = escanear(&w);
    assert_eq!(status(&mut w, &g), GameStatus::NoBackup);
    let BackupOutcome::Created(e) = backup(&mut w, &g, SnapshotKind::Manual, Some("antes de atualizar"), agora()).unwrap()
    else {
        panic!()
    };
    assert_eq!((e.kind, e.label.as_deref()), (SnapshotKind::Manual, Some("antes de atualizar")));
    assert_eq!(status(&mut w, &g), GameStatus::Protected);
    assert!(w.vault.root.join(&e.dir_name).join("jogo.json").is_file());
}

#[test]
fn auto_sem_mudanca_nao_duplica_mas_manual_sim() {
    let mut w = mundo();
    salvar(&w, b"v1");
    let g = escanear(&w);
    let BackupOutcome::Created(m1) = backup(&mut w, &g, SnapshotKind::Manual, None, agora()).unwrap() else { panic!() };
    let BackupOutcome::Unchanged(e) = backup(&mut w, &g, SnapshotKind::Auto, None, agora()).unwrap() else { panic!() };
    assert_eq!(e.file_name, m1.file_name);
    assert!(matches!(backup(&mut w, &g, SnapshotKind::Manual, None, agora()).unwrap(), BackupOutcome::Created(_)));
    assert_eq!(w.index.for_game("pc:Jogo A").len(), 2);
}

#[test]
fn mudou_vira_save_novo_e_auto_cria() {
    let mut w = mundo();
    salvar(&w, b"v1");
    let g = escanear(&w);
    backup(&mut w, &g, SnapshotKind::Manual, None, agora()).unwrap();
    salvar(&w, b"versao 2");
    let g2 = escanear(&w);
    assert_eq!(status(&mut w, &g2), GameStatus::NewSave);
    assert!(matches!(
        backup(&mut w, &g2, SnapshotKind::Auto, None, agora() + Duration::minutes(5)).unwrap(),
        BackupOutcome::Created(_)
    ));
    assert_eq!(status(&mut w, &g2), GameStatus::Protected);
}

#[test]
fn retencao_apaga_do_disco() {
    let mut w = mundo();
    let mut dir_name = String::new();
    for i in 0..12 {
        salvar(&w, format!("versao {i}").as_bytes());
        let g = escanear(&w);
        let out = backup(&mut w, &g, SnapshotKind::Auto, None, agora() + Duration::hours(i)).unwrap();
        let BackupOutcome::Created(e) = out else { panic!("backup {i} não criou") };
        dir_name = e.dir_name;
    }
    assert_eq!(w.index.for_game("pc:Jogo A").len(), 10);
    let zips = std::fs::read_dir(w.vault.root.join(dir_name))
        .unwrap()
        .filter(|f| f.as_ref().unwrap().file_name().to_string_lossy().ends_with(".zip"))
        .count();
    assert_eq!(zips, 10);
}

#[test]
fn cofre_desconectado_nao_grava_em_lugar_nenhum() {
    let mut w = mundo();
    salvar(&w, b"v1");
    let g = escanear(&w);
    let cofre = w.t.path().join("cofre");
    let movido = w.t.path().join("cofre-movido");
    std::fs::rename(&cofre, &movido).unwrap();
    let err = backup(&mut w, &g, SnapshotKind::Manual, None, agora()).unwrap_err();
    assert!(matches!(err, Error::VaultUnavailable(ref p) if *p == cofre));
    assert!(!cofre.exists());
    assert_eq!(std::fs::read_dir(&movido).unwrap().count(), 1);
}

#[test]
fn jogo_sem_arquivo_nao_tem_o_que_salvar() {
    let mut w = mundo();
    salvar(&w, b"v1");
    let mut g = escanear(&w);
    g.files.clear();
    assert!(matches!(backup(&mut w, &g, SnapshotKind::Manual, None, agora()).unwrap(), BackupOutcome::NothingToBackup));
    assert_eq!(status(&mut w, &g), GameStatus::NoSave);
}
