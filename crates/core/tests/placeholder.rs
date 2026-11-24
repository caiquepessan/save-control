mod common;

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use savecontrol_core::{env::SystemPaths, placeholder::*};

fn sys() -> SystemPaths {
    SystemPaths::under_home(Path::new("C:/Users/teste"), "teste")
}

fn ctx<'a>(
    s: &'a SystemPaths,
    i: &'a [InstallCandidate],
    ids: &'a [String],
    gd: Option<&'a Path>,
) -> ExpandCtx<'a> {
    ExpandCtx { sys: s, installs: i, store_user_ids: ids, game_dir: gd, game_name: "Jogo" }
}

#[test]
fn expande_pasta_do_sistema() {
    let s = sys();
    let e = expand("<winAppData>/Jogo/saves", &ctx(&s, &[], &[], None));
    assert_eq!(e.len(), 1);
    assert_eq!(e[0].root_template, "<winAppData>/Jogo/saves");
    assert_eq!(e[0].root_path, PathBuf::from("C:/Users/teste/AppData/Roaming/Jogo/saves"));
    assert_eq!(e[0].tail_glob, None);
    assert!(e[0].bindings.is_empty());
}

#[test]
fn separa_raiz_do_glob() {
    let s = sys();
    let e = expand("<winDocuments>/Jogo/*.sav", &ctx(&s, &[], &[], None));
    assert_eq!(e[0].root_template, "<winDocuments>/Jogo");
    assert_eq!(e[0].root_path, PathBuf::from("C:/Users/teste/Documents/Jogo"));
    assert_eq!(e[0].tail_glob.as_deref(), Some("*.sav"));
}

#[test]
fn base_gera_uma_expansao_por_instalacao() {
    let s = sys();
    let i = [
        InstallCandidate {
            root: "D:/Steam".into(),
            game: "Jogo".into(),
            base: "D:/Steam/steamapps/common/Jogo".into(),
            store_game_id: Some("1000".into()),
        },
        InstallCandidate {
            root: "E:/Jogos".into(),
            game: "Jogo".into(),
            base: "E:/Jogos/Jogo".into(),
            store_game_id: None,
        },
    ];
    let e = expand("<base>/Saves", &ctx(&s, &i, &[], None));
    assert_eq!(e.len(), 2);
    assert_eq!(e[0].root_template, "<base>/Saves");
    assert_eq!(e[0].bindings["base"], "D:/Steam/steamapps/common/Jogo");
    assert_eq!(e[1].root_path, PathBuf::from("E:/Jogos/Jogo/Saves"));
}

#[test]
fn base_sem_instalacao_nao_expande() {
    assert!(expand("<base>/Saves", &ctx(&sys(), &[], &[], None)).is_empty());
}

#[test]
fn store_user_id_desconhecido_vira_glob() {
    let s = sys();
    let e = expand("<winLocalAppData>/Jogo/<storeUserId>/save.dat", &ctx(&s, &[], &[], None));
    assert_eq!(e.len(), 1);
    assert_eq!(e[0].root_template, "<winLocalAppData>/Jogo");
    assert_eq!(e[0].tail_glob.as_deref(), Some("*/save.dat"));
}

#[test]
fn store_user_id_conhecido_vira_binding() {
    let s = sys();
    let ids = ["111".to_string(), "222".to_string()];
    let e = expand("<winLocalAppData>/Jogo/<storeUserId>/save.dat", &ctx(&s, &[], &ids, None));
    assert_eq!(e.len(), 2);
    assert_eq!(e[1].root_template, "<winLocalAppData>/Jogo/<storeUserId>/save.dat");
    assert_eq!(e[1].bindings["storeUserId"], "222");
    assert_eq!(e[1].root_path, PathBuf::from("C:/Users/teste/AppData/Local/Jogo/222/save.dat"));
}

#[test]
fn colchete_escapado_e_literal() {
    let s = sys();
    let e = expand("<winAppData>/Jogo [[]EU]/save.dat", &ctx(&s, &[], &[], None));
    assert_eq!(e[0].tail_glob, None);
    assert_eq!(e[0].root_path, PathBuf::from("C:/Users/teste/AppData/Roaming/Jogo [EU]/save.dat"));
}

#[test]
fn placeholder_desconhecido_nao_expande() {
    assert!(expand("<xdgData>/jogo", &ctx(&sys(), &[], &[], None)).is_empty());
}

#[test]
fn game_dir_interno() {
    let s = sys();
    let e = expand("<gameDir>/saves", &ctx(&s, &[], &[], Some(Path::new("D:/Jogos/Banjo"))));
    assert_eq!(e[0].root_path, PathBuf::from("D:/Jogos/Banjo/saves"));
    assert_eq!(e[0].bindings["gameDir"], "D:/Jogos/Banjo");
}

#[test]
fn resolve_root_usa_bindings() {
    let s = sys();
    let b = BTreeMap::from([("base".to_string(), "E:/Games/X".to_string())]);
    assert_eq!(resolve_root("<base>/Save", &s, &b), Some(PathBuf::from("E:/Games/X/Save")));
    assert_eq!(resolve_root("<base>/Save", &s, &BTreeMap::new()), None);
    assert_eq!(
        resolve_root("<home>/Jogo", &s, &BTreeMap::new()),
        Some(PathBuf::from("C:/Users/teste/Jogo"))
    );
}
