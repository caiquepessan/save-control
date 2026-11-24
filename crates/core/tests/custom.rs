mod common;

use std::path::Path;

use common::fake_home;
use savecontrol_core::{custom::CustomGame, local_state::*, placeholder::*, templatize::templatize};

#[test]
fn templatize_pastas_conhecidas() {
    let (_t, s) = fake_home();
    assert_eq!(templatize(&s.app_data.join("Jogo/saves"), &s, None), "<winAppData>/Jogo/saves");
    assert_eq!(
        templatize(&s.local_app_data_low.join("Estúdio/Jogo"), &s, None),
        "<winLocalAppDataLow>/Estúdio/Jogo"
    );
    assert_eq!(templatize(&s.documents.join("Jogo [EU]"), &s, None), "<winDocuments>/Jogo [[]EU[]]");
    assert_eq!(
        templatize(Path::new("D:/Downloads/Algo/user/save"), &s, None),
        "D:/Downloads/Algo/user/save"
    );
    assert_eq!(
        templatize(Path::new("D:/Jogos/X/saves"), &s, Some(Path::new("D:/Jogos/X"))),
        "<gameDir>/saves"
    );
}

#[test]
fn templatize_ida_e_volta() {
    let (_t, s) = fake_home();
    for p in [s.app_data.join("Jogo/saves"), s.documents.join("Jogo [EU]"), s.home.join("Saved Games/X")] {
        let t = templatize(&p, &s, None);
        let e = expand(&t, &ExpandCtx { sys: &s, installs: &[], store_user_ids: &[], game_dir: None, game_name: "x" });
        assert_eq!(e[0].root_path, p);
    }
}

#[test]
fn estado_local_salva_e_carrega() {
    let (t, s) = fake_home();
    let f = t.path().join("dados/jogos.json");
    assert_eq!(LocalState::load(&f).unwrap(), LocalState::default());
    let mut st = LocalState::default();
    st.custom_games.push(CustomGame::new("Meu Jogo", &[s.app_data.join("MJ")], &["mj.exe".into()], None, &s));
    st.settings.insert("pc:Jogo A".into(), LocalGameSettings { ignored: true, ..Default::default() });
    st.save(&f).unwrap();
    assert_eq!(LocalState::load(&f).unwrap(), st);
}

#[test]
fn jogo_manual_novo() {
    let (_t, s) = fake_home();
    let g = CustomGame::new("Meu Jogo", &[s.app_data.join("MJ")], &[], None, &s);
    assert!(g.id.starts_with("man:"));
    assert_eq!(g.save_templates, ["<winAppData>/MJ"]);
}
