use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use savecontrol_core::{
    env::SystemPaths,
    error::Error,
    machine::MachineIdentity,
    targets::{TargetCtx, resolve_targets},
    vault::format::*,
};

fn meta_com(template: &str, bindings: &[(&str, &str)]) -> SnapshotMeta {
    SnapshotMeta {
        format: FORMAT,
        game_id: "pc:J".into(),
        title: "J".into(),
        kind: SnapshotKind::Manual,
        label: None,
        created_at: chrono::Utc::now(),
        app_version: "0.1.0".into(),
        machine: MachineIdentity { id: "aaaa".into(), pc_name: "A".into(), user_name: "a".into() },
        roots: vec![SnapshotRoot {
            alias: "r0".into(),
            template: template.into(),
            bindings: bindings.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
            original_path: "X:/orig".into(),
        }],
        files: vec![],
    }
}

fn sys_de(home: &str) -> SystemPaths {
    SystemPaths::under_home(Path::new(home), "u")
}

fn ctx<'a>(s: &'a SystemPaths, ids: &[&str], gd: Option<&str>, lembrado: &'a BTreeMap<String, PathBuf>) -> TargetCtx<'a> {
    TargetCtx {
        sys: s,
        installs: vec![],
        store_user_ids: ids.iter().map(|x| x.to_string()).collect(),
        game_dirs: gd.map(PathBuf::from).into_iter().collect(),
        remembered: lembrado,
    }
}

fn um(r: Result<Vec<(String, PathBuf)>, Error>) -> PathBuf {
    let v = r.unwrap();
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].0, "r0");
    v[0].1.clone()
}

#[test]
fn outro_usuario_outra_home() {
    let s = sys_de("C:/Users/b");
    let nada = BTreeMap::new();
    let p = um(resolve_targets(&meta_com("<winAppData>/Jogo", &[]), &ctx(&s, &[], None, &nada), &BTreeMap::new()));
    assert_eq!(p, PathBuf::from("C:/Users/b/AppData/Roaming/Jogo"));
}

#[test]
fn steam_user_id_unico_troca_sozinho() {
    let s = sys_de("C:/Users/b");
    let nada = BTreeMap::new();
    let m = meta_com("<winLocalAppData>/J/<storeUserId>", &[("storeUserId", "111")]);
    let p = um(resolve_targets(&m, &ctx(&s, &["222"], None, &nada), &BTreeMap::new()));
    assert_eq!(p, PathBuf::from("C:/Users/b/AppData/Local/J/222"));
}

#[test]
fn steam_user_id_original_tem_preferencia() {
    let s = sys_de("C:/Users/b");
    let nada = BTreeMap::new();
    let m = meta_com("<winLocalAppData>/J/<storeUserId>", &[("storeUserId", "111")]);
    let p = um(resolve_targets(&m, &ctx(&s, &["111", "333"], None, &nada), &BTreeMap::new()));
    assert_eq!(p, PathBuf::from("C:/Users/b/AppData/Local/J/111"));
}

#[test]
fn varios_candidatos_pedem_escolha() {
    let s = sys_de("C:/Users/b");
    let nada = BTreeMap::new();
    let m = meta_com("<winLocalAppData>/J/<storeUserId>", &[("storeUserId", "111")]);
    let err = resolve_targets(&m, &ctx(&s, &["222", "333"], None, &nada), &BTreeMap::new()).unwrap_err();
    let Error::NeedsTarget { root, candidates } = err else { panic!() };
    assert_eq!(root, "<winLocalAppData>/J/<storeUserId>");
    assert_eq!(candidates.len(), 2);
}

#[test]
fn game_dir_lembrado_e_override() {
    let s = sys_de("C:/Users/b");
    let nada = BTreeMap::new();
    let m = meta_com("<gameDir>/saves", &[("gameDir", "D:/Jogos/X")]);
    let err = resolve_targets(&m, &ctx(&s, &[], None, &nada), &BTreeMap::new()).unwrap_err();
    assert!(matches!(err, Error::NeedsTarget { ref candidates, .. } if candidates.is_empty()));

    let lembrado = BTreeMap::from([("<gameDir>/saves".to_string(), PathBuf::from("E:/X/saves"))]);
    assert_eq!(um(resolve_targets(&m, &ctx(&s, &[], None, &lembrado), &BTreeMap::new())), PathBuf::from("E:/X/saves"));

    let forcado = BTreeMap::from([("r0".to_string(), PathBuf::from("F:/Y"))]);
    assert_eq!(um(resolve_targets(&m, &ctx(&s, &[], None, &lembrado), &forcado)), PathBuf::from("F:/Y"));
}

#[test]
fn absoluto_fica_igual() {
    let s = sys_de("C:/Users/b");
    let nada = BTreeMap::new();
    let p = um(resolve_targets(&meta_com("D:/Algo/save", &[]), &ctx(&s, &[], None, &nada), &BTreeMap::new()));
    assert_eq!(p, PathBuf::from("D:/Algo/save"));
}
