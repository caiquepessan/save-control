mod common;

use std::path::{Path, PathBuf};

use common::write;
use savecontrol_core::{
    fingerprint::{HashCache, fingerprint_of_meta},
    machine::MachineIdentity,
    vault::{Vault, format::*, index::VaultIndex, write::*},
};

fn pc(id: &str) -> MachineIdentity {
    MachineIdentity { id: id.into(), pc_name: "PC".into(), user_name: "teste".into() }
}

fn meta(machine: MachineIdentity) -> SnapshotMeta {
    SnapshotMeta {
        format: FORMAT,
        game_id: "pc:Jogo A".into(),
        title: "Jogo A".into(),
        kind: SnapshotKind::Auto,
        label: None,
        created_at: chrono::Utc::now(),
        app_version: "0.1.0".into(),
        machine,
        roots: vec![SnapshotRoot {
            alias: "r0".into(),
            template: "<winAppData>/JogoA".into(),
            bindings: Default::default(),
            original_path: "C:/x".into(),
        }],
        files: vec![],
    }
}

/// Cofre com uma pasta de jogo e um backup gravado de verdade; devolve (cofre, pasta, zip).
fn com_backup(t: &Path, machine: MachineIdentity, file_name: &str) -> (Vault, PathBuf, PathBuf, SnapshotMeta) {
    let src = t.join("src");
    write(&src.join("slot1.sav"), b"um");
    let v = Vault::create(&t.join("cofre")).unwrap();
    let dir = v.game_dir("pc:Jogo A", "Jogo A").unwrap();
    let sources = [SourceFile { abs_path: src.join("slot1.sav"), root_alias: "r0".into(), rel_path: "slot1.sav".into() }];
    let (zip, m) = write_snapshot(&dir, file_name, meta(machine), &sources, &NoWait).unwrap();
    (v, dir, zip, m)
}

#[test]
fn refresh_acha_zips_e_ignora_sobras() {
    let t = tempfile::tempdir().unwrap();
    let (v, dir, zip, _) = com_backup(t.path(), pc("aaaa111122223333"), "2026-11-24_10-00-00_PC-AAAA_auto.zip");
    let src = t.path().join("src");
    let sources = [SourceFile { abs_path: src.join("slot1.sav"), root_alias: "r0".into(), rel_path: "slot1.sav".into() }];
    write_snapshot(&dir, "2026-11-24_11-00-00_PC-AAAA_auto.zip", meta(pc("aaaa111122223333")), &sources, &NoWait)
        .unwrap();
    std::fs::write(dir.join("2026-11-24_12-00-00_PC-AAAA_auto.zip.parcial"), b"x").unwrap();
    std::fs::copy(&zip, dir.join("2026-11-24_10-00-00_PC-AAAA_auto (1).zip")).unwrap();

    let mut idx = VaultIndex::default();
    idx.refresh(&v).unwrap();
    assert_eq!(idx.entries.len(), 2);
    std::fs::remove_file(&zip).unwrap();
    idx.refresh(&v).unwrap();
    assert_eq!(idx.entries.len(), 1);
    assert_eq!(idx.entries[0].file_name, "2026-11-24_11-00-00_PC-AAAA_auto.zip");
}

#[test]
fn identico_vale_entre_pcs() {
    let t = tempfile::tempdir().unwrap();
    let (v, _, _, m) = com_backup(t.path(), pc("bbbb111122223333"), "2026-11-24_10-00-00_PC-BBBB_auto.zip");
    let mut idx = VaultIndex::default();
    idx.refresh(&v).unwrap();
    let achado = idx.find_identical("pc:Jogo A", &fingerprint_of_meta(&m)).unwrap();
    assert_eq!(achado.machine_id, "bbbb111122223333");
    assert!(idx.find_identical("pc:Outro", &fingerprint_of_meta(&m)).is_none());
}

#[test]
fn fixar_renomeia_e_vira_pinned() {
    let t = tempfile::tempdir().unwrap();
    let (v, dir, _, _) = com_backup(t.path(), pc("aaaa111122223333"), "2026-11-24_10-00-00_PC-AAAA_auto.zip");
    let dir_name = dir.file_name().unwrap().to_str().unwrap();
    let novo = v.pin(dir_name, "2026-11-24_10-00-00_PC-AAAA_auto.zip").unwrap();
    assert_eq!(novo, "2026-11-24_10-00-00_PC-AAAA_fixado.zip");
    let mut idx = VaultIndex::default();
    idx.refresh(&v).unwrap();
    assert_eq!(idx.entries[0].kind, SnapshotKind::Pinned);
}

#[test]
fn cache_de_hash_nao_rele() {
    let t = tempfile::tempdir().unwrap();
    let f = t.path().join("a.sav");
    std::fs::write(&f, b"conteudo").unwrap();
    let modified = chrono::DateTime::<chrono::Utc>::from(std::fs::metadata(&f).unwrap().modified().unwrap());
    let mut cache = HashCache::default();
    let h1 = cache.hash(&f, 8, modified).unwrap();
    assert_eq!(h1, blake3::hash(b"conteudo").to_hex().to_string());
    std::fs::remove_file(&f).unwrap();
    assert_eq!(cache.hash(&f, 8, modified).unwrap(), h1);
}
