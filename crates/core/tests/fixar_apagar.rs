//! Retenção ajustável (geral e por jogo), fixar/desafixar e apagar backups.
mod common;

use std::path::Path;

use common::write;
use savecontrol_core::{
    backup::BackupOutcome,
    engine::{Engine, EngineConfig},
    env::SystemPaths,
    registry::{FakeRegistry, Hive},
    status::GameStatus,
    vault::{Vault, format::SnapshotKind, index::SnapshotEntry, retention::RetentionPolicy},
};

/// Uma Engine do "PC" `pc`, com o cofre compartilhado em `t/cofre`.
fn engine(t: &Path, pc: &str) -> (SystemPaths, Engine) {
    let s = SystemPaths::under_home(&t.join("home"), "teste");
    let cfg = EngineConfig { data_dir: t.join(format!("dados-{pc}")), vault_dir: t.join("cofre"), include_config: false };
    Vault::create(&cfg.vault_dir).unwrap();
    let mut reg = FakeRegistry::default();
    reg.set(Hive::LocalMachine, r"SOFTWARE\Microsoft\Cryptography", "MachineGuid", &format!("guid-{pc}"));
    let e = Engine::open(cfg, s.clone(), Box::new(reg)).unwrap();
    (s, e)
}

/// Jogo à mão "Jogo A" com um save; devolve o id.
fn jogo(e: &mut Engine, s: &SystemPaths) -> String {
    let dir = s.app_data.join("JogoA");
    write(&dir.join("slot.sav"), b"inicio");
    let g = e.add_custom_game("Jogo A", &[dir], &[], None).unwrap();
    e.scan().unwrap();
    g.id
}

/// Muda o save (conteúdo sempre diferente) e faz um backup do tipo pedido.
fn backup(e: &mut Engine, s: &SystemPaths, id: &str, n: usize, kind: SnapshotKind) -> SnapshotEntry {
    write(&s.app_data.join("JogoA").join("slot.sav"), format!("versao {n}").as_bytes());
    match e.backup(id, kind, None).unwrap() {
        BackupOutcome::Created(x) => x,
        o => panic!("{o:?}"),
    }
}

fn do_tipo(e: &Engine, id: &str, kind: SnapshotKind) -> Vec<SnapshotEntry> {
    e.history(id).into_iter().filter(|x| x.kind == kind).collect()
}

/// Guarda só os `n` automáticos mais novos (sem os "1 por dia / por semana").
fn so_os_ultimos(n: usize) -> RetentionPolicy {
    RetentionPolicy { auto_recent: n, auto_daily_days: 0, auto_weekly_weeks: 0, pre_restore_keep: 5 }
}

#[test]
fn retencao_geral_ajustada() {
    let t = tempfile::tempdir().unwrap();
    let (s, mut e) = engine(t.path(), "a");
    let id = jogo(&mut e, &s);
    e.set_retention(so_os_ultimos(3));
    for n in 1..=5 {
        backup(&mut e, &s, &id, n, SnapshotKind::Auto);
    }
    assert_eq!(do_tipo(&e, &id, SnapshotKind::Auto).len(), 3);
}

#[test]
fn retencao_do_jogo_vale_mais_que_a_geral() {
    let t = tempfile::tempdir().unwrap();
    let (s, mut e) = engine(t.path(), "a");
    let id = jogo(&mut e, &s);
    e.update_game_settings(&id, |st| st.retention = Some(so_os_ultimos(2))).unwrap();
    for n in 1..=4 {
        backup(&mut e, &s, &id, n, SnapshotKind::Auto);
    }
    assert_eq!(do_tipo(&e, &id, SnapshotKind::Auto).len(), 2, "a retenção do jogo foi ignorada");
}

#[test]
fn fixar_um_automatico_o_protege_da_limpeza() {
    let t = tempfile::tempdir().unwrap();
    let (s, mut e) = engine(t.path(), "a");
    let id = jogo(&mut e, &s);
    e.set_retention(so_os_ultimos(1));
    let a1 = backup(&mut e, &s, &id, 1, SnapshotKind::Auto);
    let fixado = e.pin_snapshot(&id, &a1.file_name, true).unwrap();
    assert_eq!(fixado.kind, SnapshotKind::Pinned);
    assert!(fixado.file_name.ends_with("_fixado.zip"), "{}", fixado.file_name);
    backup(&mut e, &s, &id, 2, SnapshotKind::Auto);
    backup(&mut e, &s, &id, 3, SnapshotKind::Auto);
    assert_eq!(do_tipo(&e, &id, SnapshotKind::Pinned).len(), 1, "a limpeza apagou o fixado");
    assert_eq!(do_tipo(&e, &id, SnapshotKind::Auto).len(), 1);
    assert!(t.path().join("cofre").join(&fixado.dir_name).join(&fixado.file_name).is_file());

    let de_volta = e.pin_snapshot(&id, &fixado.file_name, false).unwrap();
    assert_eq!(de_volta.kind, SnapshotKind::Auto);
    assert!(de_volta.file_name.ends_with("_auto.zip"), "{}", de_volta.file_name);
}

#[test]
fn fixar_so_vale_para_automatico() {
    let t = tempfile::tempdir().unwrap();
    let (s, mut e) = engine(t.path(), "a");
    let id = jogo(&mut e, &s);
    let m = backup(&mut e, &s, &id, 1, SnapshotKind::Manual);
    assert!(e.pin_snapshot(&id, &m.file_name, true).is_err());
    let h = e.history(&id);
    assert_eq!((h.len(), h[0].kind, h[0].file_name.as_str()), (1, SnapshotKind::Manual, m.file_name.as_str()));
}

#[test]
fn fixar_backup_de_outro_pc() {
    let t = tempfile::tempdir().unwrap();
    let (s, mut a) = engine(t.path(), "a");
    let id = jogo(&mut a, &s);
    let do_a = backup(&mut a, &s, &id, 1, SnapshotKind::Auto);

    let (_, mut b) = engine(t.path(), "b");
    b.scan().unwrap();
    let fixado = b.pin_snapshot(&id, &do_a.file_name, true).unwrap();
    assert_eq!(fixado.machine_id, do_a.machine_id, "o backup fixado não é o do outro PC");
    assert_eq!(do_tipo(&b, &id, SnapshotKind::Pinned).len(), 1);

    a.scan().unwrap();
    assert_eq!(do_tipo(&a, &id, SnapshotKind::Pinned).len(), 1, "o outro PC não viu o backup fixado");
    assert!(do_tipo(&a, &id, SnapshotKind::Auto).is_empty());
}

#[test]
fn apagar_backup() {
    let t = tempfile::tempdir().unwrap();
    let (s, mut e) = engine(t.path(), "a");
    let id = jogo(&mut e, &s);
    let velho = backup(&mut e, &s, &id, 1, SnapshotKind::Auto);
    backup(&mut e, &s, &id, 2, SnapshotKind::Auto);
    e.delete_snapshot(&id, &velho.file_name).unwrap();
    assert_eq!(e.history(&id).len(), 1);
    assert!(e.history(&id).iter().all(|x| x.file_name != velho.file_name));
    assert!(!t.path().join("cofre").join(&velho.dir_name).join(&velho.file_name).exists(), "o zip continuou no cofre");
}

#[test]
fn apagar_o_ultimo_backup_deixa_sem_backup() {
    let t = tempfile::tempdir().unwrap();
    let (s, mut e) = engine(t.path(), "a");
    let id = jogo(&mut e, &s);
    let unico = backup(&mut e, &s, &id, 1, SnapshotKind::Manual);
    e.delete_snapshot(&id, &unico.file_name).unwrap();
    let status = e.scan().unwrap().into_iter().find(|(g, _)| g.id == id).unwrap().1;
    assert_eq!(status, GameStatus::NoBackup);
}
