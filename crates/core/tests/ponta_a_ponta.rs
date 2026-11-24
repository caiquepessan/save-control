mod common;

use std::cell::RefCell;
use std::path::Path;

use common::write;
use savecontrol_core::{
    backup::BackupOutcome,
    engine::{Engine, EngineConfig},
    env::SystemPaths,
    error::Result,
    manifest::store::{FetchResult, Fetcher, UpdateOutcome},
    model::Game,
    process::FakeProbe,
    registry::{FakeRegistry, Hive},
    restore::RestoreOptions,
    status::GameStatus,
    vault::format::SnapshotKind,
};

const BASE: &str = "Jogo Teste:\n  files:\n    <winAppData>/JogoTeste:\n      tags: [save]\n";
const ID: &str = "pc:Jogo Teste";

struct UmaVez(RefCell<Option<Vec<u8>>>);

impl Fetcher for UmaVez {
    fn fetch(&self, _url: &str, _etag: Option<&str>) -> Result<FetchResult> {
        Ok(match self.0.borrow_mut().take() {
            Some(body) => FetchResult::Fetched { body, etag: Some("e1".into()) },
            None => FetchResult::NotModified,
        })
    }
}

fn base() -> UmaVez {
    UmaVez(RefCell::new(Some(BASE.as_bytes().to_vec())))
}

fn pc(t: &Path, nome: &str) -> (SystemPaths, EngineConfig, FakeRegistry) {
    let s = SystemPaths::under_home(&t.join(format!("home-{nome}")), nome);
    let cfg = EngineConfig { data_dir: t.join(format!("dados-{nome}")), vault_dir: t.join("cofre"), include_config: false };
    let mut reg = FakeRegistry::default();
    reg.set(Hive::LocalMachine, r"SOFTWARE\Microsoft\Cryptography", "MachineGuid", &format!("guid-{nome}"));
    (s, cfg, reg)
}

fn jogo(eng: &mut Engine) -> (Game, GameStatus) {
    eng.scan().unwrap().into_iter().find(|(g, _)| g.id == ID).unwrap()
}

#[test]
fn ciclo_completo_e_outro_pc() {
    let t = tempfile::tempdir().unwrap();

    // ---- PC A ----
    let (sa, cfg_a, reg_a) = pc(t.path(), "a");
    let slot = sa.app_data.join("JogoTeste/slot1.sav");
    write(&slot, b"v1");
    let mut a = Engine::open(cfg_a, sa.clone(), Box::new(reg_a)).unwrap();
    assert_eq!(a.update_manifest(&base()).unwrap(), UpdateOutcome::Updated);
    assert_eq!(jogo(&mut a).1, GameStatus::NoBackup);

    let BackupOutcome::Created(manual) = a.backup(ID, SnapshotKind::Manual, Some("antes de atualizar")).unwrap() else {
        panic!()
    };
    assert_eq!(jogo(&mut a).1, GameStatus::Protected);

    write(&slot, b"versao 2");
    assert_eq!(jogo(&mut a).1, GameStatus::NewSave);
    assert!(matches!(a.backup(ID, SnapshotKind::Auto, None).unwrap(), BackupOutcome::Created(_)));
    assert_eq!(jogo(&mut a).1, GameStatus::Protected);

    // Joga mais um pouco: o save atual passa a ser diferente de todos os backups,
    // então restaurar precisa guardá-lo como "antes de restaurar" (design §10.4).
    write(&slot, b"versao 3");
    jogo(&mut a);
    a.restore(ID, &manual.file_name, &RestoreOptions::default(), &FakeProbe(vec![])).unwrap();
    assert_eq!(std::fs::read(&slot).unwrap(), b"v1");
    let mut kinds: Vec<SnapshotKind> = a.history(ID).into_iter().map(|e| e.kind).collect();
    kinds.sort_by_key(|k| k.suffix());
    assert_eq!(kinds, [SnapshotKind::PreRestore, SnapshotKind::Auto, SnapshotKind::Manual]);
    assert_eq!(jogo(&mut a).1, GameStatus::Protected);

    drop(a);
    let (_, cfg_a2, reg_a2) = pc(t.path(), "a");
    let a2 = Engine::open(cfg_a2, sa, Box::new(reg_a2)).unwrap();
    assert_eq!(a2.history(ID).len(), 3);
    let rotulo_a = a2.history(ID)[0].machine_label.clone();
    drop(a2);

    // ---- PC B: mesmo cofre, sem save nenhum ----
    let (sb, cfg_b, reg_b) = pc(t.path(), "b");
    let mut b = Engine::open(cfg_b, sb.clone(), Box::new(reg_b)).unwrap();
    b.update_manifest(&base()).unwrap();
    let (g, status) = jogo(&mut b);
    assert!(g.vault_only);
    assert_eq!(status, GameStatus::NoSave);
    assert_eq!(b.history(ID).len(), 3);

    b.restore(ID, &manual.file_name, &RestoreOptions::default(), &FakeProbe(vec![])).unwrap();
    let slot_b = sb.app_data.join("JogoTeste/slot1.sav");
    assert_eq!(std::fs::read(&slot_b).unwrap(), b"v1");
    let (g, status) = jogo(&mut b);
    assert!(!g.vault_only);
    assert_eq!(status, GameStatus::Protected);

    let BackupOutcome::Created(de_b) = b.backup(ID, SnapshotKind::Manual, None).unwrap() else { panic!() };
    assert_ne!(de_b.machine_label, rotulo_a);
    assert!(de_b.file_name.contains(&de_b.machine_label));
    let do_a = b.history(ID).into_iter().filter(|e| e.machine_label == rotulo_a).count();
    assert_eq!(do_a, 3);
}
