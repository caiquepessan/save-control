mod common;

use common::{fake_home, write};
use savecontrol_core::{
    catalog::*, custom::CustomGame, env::SystemPaths, local_state::*, manifest::Manifest, model::*, scan::*,
    stores::*,
};

fn run(
    s: &SystemPaths,
    m: Option<&Manifest>,
    i: &Installs,
    c: &[CatalogMatch],
    l: &LocalState,
    cfg: bool,
) -> Vec<Game> {
    scan(&ScanInput { sys: s, manifest: m, installs: i, catalog: c, local: l, include_config: cfg })
}

#[test]
fn acha_save_no_appdata_sem_instalacao() {
    let (_t, s) = fake_home();
    write(&s.app_data.join("JogoA/saves/slot1.sav"), b"x");
    let m = Manifest::from_yaml("Jogo A:\n  files:\n    <winAppData>/JogoA/saves:\n      tags: [save]\n").unwrap();
    let g = run(&s, Some(&m), &Installs::default(), &[], &LocalState::default(), false);
    assert_eq!(g.len(), 1);
    assert_eq!((g[0].id.as_str(), &g[0].source), ("pc:Jogo A", &GameSource::ManifestOnly));
    assert_eq!(g[0].files[0].rel_path, "slot1.sav");
    assert_eq!(g[0].save_roots[0].template, "<winAppData>/JogoA/saves");
}

#[test]
fn jogo_da_steam_com_base() {
    let (t, s) = fake_home();
    let base = t.path().join("Steam/steamapps/common/JogoS");
    write(&base.join("Saves/a.sav"), b"x");
    let i = Installs {
        steam_user_ids: vec![],
        games: vec![InstalledGame {
            store: Store::Steam,
            root: t.path().join("Steam"),
            folder_name: "JogoS".into(),
            base: base.clone(),
            store_game_id: Some("1000".into()),
            display_name: None,
        }],
    };
    let m = Manifest::from_yaml("Jogo S:\n  files:\n    <base>/Saves:\n      tags: [save]\n  steam:\n    id: 1000\n").unwrap();
    let g = run(&s, Some(&m), &i, &[], &LocalState::default(), false);
    assert_eq!(g[0].source, GameSource::Steam);
    assert_eq!(g[0].install_dirs, [base.clone()]);
    assert_eq!(g[0].steam_app_id, Some(1000));
    assert_eq!(g[0].save_roots[0].bindings["base"], base.display().to_string().replace('\\', "/"));
    assert!(g[0].warnings.contains(&Warning::InsideGameFolder));
}

#[test]
fn so_config_depende_do_ajuste() {
    let (_t, s) = fake_home();
    write(&s.documents.join("JogoB/settings.cfg"), b"x");
    let m = Manifest::from_yaml("Jogo B:\n  files:\n    <winDocuments>/JogoB/settings.cfg:\n      tags: [config]\n").unwrap();
    assert!(run(&s, Some(&m), &Installs::default(), &[], &LocalState::default(), false).is_empty());
    assert_eq!(run(&s, Some(&m), &Installs::default(), &[], &LocalState::default(), true).len(), 1);
}

#[test]
fn caminho_amplo_vira_aviso() {
    let (_t, s) = fake_home();
    write(&s.documents.join("qualquer.doc"), b"x");
    write(&s.app_data.join("Real/save.dat"), b"x");
    let m = Manifest::from_yaml(
        "Jogo L:\n  files:\n    <winDocuments>:\n      tags: [save]\n    <winAppData>/Real/save.dat:\n      tags: [save]\n",
    )
    .unwrap();
    let g = run(&s, Some(&m), &Installs::default(), &[], &LocalState::default(), false);
    assert_eq!(g[0].files.len(), 1);
    assert!(g[0].warnings.contains(&Warning::IgnoredBroadPath("<winDocuments>".into())));
}

#[test]
fn pasta_downloads_vira_aviso() {
    let (t, s) = fake_home();
    let dir = t.path().join("D/Downloads/Algo/save");
    write(&dir.join("s.dat"), b"x");
    let mut l = LocalState::default();
    l.custom_games.push(CustomGame::new("Algo", &[dir.clone()], &[], &s));
    let g = run(&s, None, &Installs::default(), &[], &l, false);
    assert!(g[0].warnings.contains(&Warning::RiskyLocation(dir)));
}

#[test]
fn emulador_separado_por_jogo() {
    let (t, s) = fake_home();
    let pj = t.path().join("PJ64");
    for f in ["ZELDA.sra", "ZELDA.eep", "MARIO.eep"] {
        write(&pj.join("Save").join(f), b"x");
    }
    let c = [CatalogMatch {
        entry_id: "project64".into(),
        name: "Project64".into(),
        game_dir: Some(pj),
        save_templates: vec!["<gameDir>/Save/*".into()],
        split: Some(SplitRule::ByFileStem),
        processes: vec!["Project64.exe".into()],
    }];
    let g = run(&s, None, &Installs::default(), &c, &LocalState::default(), false);
    let z = g.iter().find(|x| x.id == "cat:project64:zelda").unwrap();
    assert_eq!((z.files.len(), z.title.as_str()), (2, "ZELDA · Project64"));
    assert!(g.iter().any(|x| x.id == "cat:project64:mario"));
}

#[test]
fn ignorado_some_e_manual_usa_game_dir() {
    let (t, s) = fake_home();
    let gd = t.path().join("Jogos/X");
    write(&gd.join("saves/a.sav"), b"x");
    let mut l = LocalState::default();
    let cg = CustomGame {
        id: "man:1".into(),
        name: "X".into(),
        save_templates: vec!["<gameDir>/saves".into()],
        process_names: vec![],
    };
    l.custom_games.push(cg);
    l.settings.insert("man:1".into(), LocalGameSettings { game_dir: Some(gd), ..Default::default() });
    assert_eq!(run(&s, None, &Installs::default(), &[], &l, false)[0].files.len(), 1);
    l.settings.get_mut("man:1").unwrap().ignored = true;
    assert!(run(&s, None, &Installs::default(), &[], &l, false).is_empty());
}

#[test]
fn instalado_sem_save_entra_vazio() {
    let (t, s) = fake_home();
    let base = t.path().join("Jogos/JogoV");
    std::fs::create_dir_all(&base).unwrap();
    let i = Installs {
        steam_user_ids: vec![],
        games: vec![InstalledGame {
            store: Store::Other,
            root: t.path().join("Jogos"),
            folder_name: "JogoV".into(),
            base,
            store_game_id: None,
            display_name: None,
        }],
    };
    let m = Manifest::from_yaml("JogoV:\n  files:\n    <base>/save:\n      tags: [save]\n").unwrap();
    let g = run(&s, Some(&m), &i, &[], &LocalState::default(), false);
    assert_eq!((g.len(), g[0].files.len()), (1, 0));
}
