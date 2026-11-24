mod common;

use common::{fake_home, write};
use savecontrol_core::{catalog::*, env::SystemPaths};

fn achados(root: &std::path::Path, s: &SystemPaths) -> Vec<CatalogMatch> {
    find_catalog_games(&Catalog::builtin(), s, &[root.to_path_buf()])
}

#[test]
fn catalogo_embutido() {
    let ids: Vec<_> = Catalog::builtin().entries.into_iter().map(|e| e.id).collect();
    assert_eq!(ids, ["zelda64recomp", "banjorecomp", "project64", "retroarch"]);
}

#[test]
fn recomp_portatil() {
    let (t, s) = fake_home();
    let dir = t.path().join("Jogos/Banjo Recompiled");
    write(&dir.join("BanjoRecompiled.exe"), b"");
    write(&dir.join("portable.txt"), b"");
    let m = achados(&t.path().join("Jogos"), &s);
    assert_eq!(m.len(), 1);
    assert_eq!((m[0].entry_id.as_str(), m[0].game_dir.as_deref()), ("banjorecomp", Some(dir.as_path())));
    assert_eq!(m[0].save_templates, ["<gameDir>/saves"]);
}

#[test]
fn recomp_sem_portable_txt_usa_appdata() {
    let (t, s) = fake_home();
    write(&t.path().join("Jogos/Z/Zelda64Recompiled.exe"), b"");
    let m = achados(&t.path().join("Jogos"), &s);
    assert_eq!(m[0].save_templates, ["<winLocalAppData>/Zelda64Recompiled/saves"]);
}

#[test]
fn recomp_so_no_appdata() {
    let (t, s) = fake_home();
    write(&s.local_app_data.join("Zelda64Recompiled/saves/x.bin"), b"1");
    let m = achados(&t.path().join("Jogos"), &s);
    assert_eq!((m[0].entry_id.as_str(), m[0].game_dir.clone()), ("zelda64recomp", None));
}

#[test]
fn exe_ignora_maiusculas_e_profundidade_maxima() {
    let (t, s) = fake_home();
    write(&t.path().join("Jogos/B/banjorecompiled.EXE"), b"");
    write(&t.path().join("Jogos/a/b/c/Zelda64Recompiled.exe"), b"");
    let m = achados(&t.path().join("Jogos"), &s);
    assert_eq!(m.len(), 1);
    assert_eq!(m[0].entry_id, "banjorecomp");
}
