mod common;

use common::{fake_home, write};
use savecontrol_core::{env::SystemPaths, fsglob::*, placeholder::*};

fn one(s: &SystemPaths, t: &str) -> Expansion {
    expand(t, &ExpandCtx { sys: s, installs: &[], store_user_ids: &[], game_dir: None, game_name: "Jogo" })
        .remove(0)
}

fn rels(f: &Found) -> Vec<&str> {
    f.files.iter().map(|x| x.rel_path.as_str()).collect()
}

#[test]
fn pasta_inclui_tudo_abaixo() {
    let (_t, s) = fake_home();
    write(&s.app_data.join("Jogo/a.sav"), b"1");
    write(&s.app_data.join("Jogo/sub/b.sav"), b"22");
    let FindResult::Found(f) = find_files(&one(&s, "<winAppData>/Jogo"), &s) else { panic!() };
    assert_eq!(rels(&f), ["a.sav", "sub/b.sav"]);
    assert_eq!(f.root_template, "<winAppData>/Jogo");
    assert_eq!(f.files[1].size, 2);
}

#[test]
fn glob_filtra_extensao() {
    let (_t, s) = fake_home();
    write(&s.app_data.join("Jogo/x.sav"), b"1");
    write(&s.app_data.join("Jogo/y.txt"), b"1");
    let FindResult::Found(f) = find_files(&one(&s, "<winAppData>/Jogo/*.sav"), &s) else { panic!() };
    assert_eq!(rels(&f), ["x.sav"]);
}

#[test]
fn ignora_maiusculas() {
    let (_t, s) = fake_home();
    write(&s.app_data.join("Jogo/Slot1.SAV"), b"1");
    let FindResult::Found(f) = find_files(&one(&s, "<winAppData>/Jogo/slot*.sav"), &s) else { panic!() };
    assert_eq!(rels(&f), ["Slot1.SAV"]);
}

#[test]
fn raiz_que_e_arquivo() {
    let (_t, s) = fake_home();
    write(&s.app_data.join("Jogo/perfil.dat"), b"1");
    let FindResult::Found(f) = find_files(&one(&s, "<winAppData>/Jogo/perfil.dat"), &s) else { panic!() };
    assert_eq!(f.root_template, "<winAppData>/Jogo");
    assert_eq!(f.root_dir, s.app_data.join("Jogo"));
    assert_eq!(rels(&f), ["perfil.dat"]);
}

#[test]
fn nada_encontrado() {
    let (_t, s) = fake_home();
    assert_eq!(find_files(&one(&s, "<winAppData>/Nada"), &s), FindResult::Nothing);
}

#[test]
fn caminho_amplo_e_ignorado() {
    let (_t, s) = fake_home();
    write(&s.documents.join("qualquer.txt"), b"1");
    assert_eq!(find_files(&one(&s, "<winDocuments>"), &s), FindResult::IgnoredBroad);
    assert_eq!(find_files(&one(&s, "<home>"), &s), FindResult::IgnoredBroad);
}

#[test]
fn curinga_solto_em_pasta_ampla_e_ignorado() {
    let (_t, s) = fake_home();
    write(&s.documents.join("Pessoal/carta.doc"), b"1");
    write(&s.home.join("perfil.sav"), b"1");
    assert_eq!(find_files(&one(&s, "<winDocuments>/*"), &s), FindResult::IgnoredBroad);
    assert_eq!(find_files(&one(&s, "<winDocuments>/*.*"), &s), FindResult::IgnoredBroad);
    let FindResult::Found(f) = find_files(&one(&s, "<home>/*.sav"), &s) else { panic!() };
    assert_eq!(rels(&f), ["perfil.sav"]);
}

#[test]
fn colchetes_literais() {
    let (_t, s) = fake_home();
    write(&s.app_data.join("Jogo [EU]/save.dat"), b"1");
    let FindResult::Found(f) =
        find_files(&one(&s, "<winAppData>/Jogo [[]EU]/save.dat"), &s) else { panic!() };
    assert_eq!(f.root_template, "<winAppData>/Jogo [[]EU]");
    assert_eq!(rels(&f), ["save.dat"]);
}
