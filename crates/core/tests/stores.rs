mod common;

use common::write;
use savecontrol_core::{registry::*, stores::*};

#[test]
fn epic_le_itens_e_pula_invalidos() {
    let t = tempfile::tempdir().unwrap();
    let loc = t.path().join("Epic/JogoEpic").display().to_string().replace('\\', "\\\\");
    write(
        &t.path().join("m/a.item"),
        format!(r#"{{"DisplayName":"Jogo Epic","InstallLocation":"{loc}","AppName":"abc"}}"#).as_bytes(),
    );
    write(&t.path().join("m/ruim.item"), b"{oops");
    let g = epic::epic_installed(&t.path().join("m"));
    assert_eq!(g.len(), 1);
    assert_eq!((g[0].folder_name.as_str(), g[0].store), ("JogoEpic", Store::Epic));
    assert_eq!(g[0].root, t.path().join("Epic"));
    assert_eq!(g[0].store_game_id.as_deref(), Some("abc"));
}

#[test]
fn gog_le_registro() {
    let mut reg = FakeRegistry::default();
    let k = r"SOFTWARE\WOW6432Node\GOG.com\Games";
    reg.add_subkey(Hive::LocalMachine, k, "1207658924");
    reg.set(Hive::LocalMachine, &format!(r"{k}\1207658924"), "path", "D:/GOG/Jogo G");
    reg.set(Hive::LocalMachine, &format!(r"{k}\1207658924"), "gameName", "Jogo G");
    let g = gog::gog_installed(&reg);
    assert_eq!(
        (g[0].folder_name.as_str(), g[0].store, g[0].store_game_id.as_deref()),
        ("Jogo G", Store::Gog, Some("1207658924"))
    );
}

#[test]
fn pasta_do_usuario() {
    let t = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(t.path().join("Jogo Um")).unwrap();
    std::fs::create_dir_all(t.path().join("jogo-dois")).unwrap();
    write(&t.path().join("x.txt"), b"1");
    let g = other::other_installed(t.path());
    assert_eq!(g.len(), 2);
    assert!(g.iter().all(|x| x.store == Store::Other && x.base == t.path().join(&x.folder_name)));
}

#[test]
fn candidatos_por_id_pasta_e_titulo() {
    let i = Installs {
        steam_user_ids: vec![],
        games: vec![
            InstalledGame {
                store: Store::Steam,
                root: "D:/S".into(),
                folder_name: "JogoA".into(),
                base: "D:/S/steamapps/common/JogoA".into(),
                store_game_id: Some("1000".into()),
                display_name: None,
            },
            InstalledGame {
                store: Store::Other,
                root: "D:/J".into(),
                folder_name: "jogo b".into(),
                base: "D:/J/jogo b".into(),
                store_game_id: None,
                display_name: None,
            },
        ],
    };
    assert_eq!(i.candidates_for("Qualquer", &[], &[1000]).len(), 1);
    assert_eq!(i.candidates_for("Qualquer", &["JOGOA".into()], &[]).len(), 1);
    assert_eq!(i.candidates_for("Jogo B", &[], &[])[0].base, std::path::PathBuf::from("D:/J/jogo b"));
    assert!(i.candidates_for("Nada", &[], &[]).is_empty());
}
