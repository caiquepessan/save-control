mod common;

use std::path::PathBuf;

use common::write;
use savecontrol_core::{
    registry::*,
    stores::{Store, steam, vdf},
};

fn esc(p: &std::path::Path) -> String {
    p.display().to_string().replace('\\', "\\\\")
}

#[test]
fn vdf_basico() {
    let v = vdf::parse("\"a\"\n{\n \"path\" \"C:\\\\Steam\" // comentário\n \"sub\" { \"x\" \"1\" }\n}").unwrap();
    assert_eq!(v.get("a").unwrap().get("path").unwrap().as_str(), Some("C:\\Steam"));
    assert_eq!(v.get("a").unwrap().get("SUB").unwrap().get("x").unwrap().as_str(), Some("1"));
}

#[test]
fn le_bibliotecas_jogos_e_usuarios() {
    let t = tempfile::tempdir().unwrap();
    let st = t.path().join("Steam");
    let lib2 = t.path().join("Lib2");
    write(
        &st.join("steamapps/libraryfolders.vdf"),
        format!(
            "\"libraryfolders\" {{ \"0\" {{ \"path\" \"{}\" }} \"1\" {{ \"path\" \"{}\" }} }}",
            esc(&st),
            esc(&lib2)
        )
        .as_bytes(),
    );
    write(
        &st.join("steamapps/appmanifest_1000.acf"),
        b"\"AppState\" { \"appid\" \"1000\" \"name\" \"Jogo A\" \"installdir\" \"JogoA\" }",
    );
    write(
        &lib2.join("steamapps/appmanifest_2000.acf"),
        b"\"AppState\" { \"appid\" \"2000\" \"name\" \"Jogo B\" \"installdir\" \"Jogo B Dir\" }",
    );
    for u in ["12345", "67890", "ac"] {
        std::fs::create_dir_all(st.join("userdata").join(u)).unwrap();
    }
    let info = steam::read_steam(&st).unwrap();
    assert_eq!(info.libraries.len(), 2);
    assert_eq!(info.user_ids, ["12345", "67890"]);
    let b = info.games.iter().find(|g| g.store_game_id.as_deref() == Some("2000")).unwrap();
    assert_eq!(b.folder_name, "Jogo B Dir");
    assert_eq!(b.base, lib2.join("steamapps/common/Jogo B Dir"));
    assert_eq!(b.store, Store::Steam);
}

#[test]
fn acha_steam_pelo_registro() {
    let mut reg = FakeRegistry::default();
    reg.set(Hive::CurrentUser, r"Software\Valve\Steam", "SteamPath", "c:/program files (x86)/steam");
    assert_eq!(steam::find_steam_root(&reg), Some(PathBuf::from("c:/program files (x86)/steam")));
    assert_eq!(steam::find_steam_root(&FakeRegistry::default()), None);
}
