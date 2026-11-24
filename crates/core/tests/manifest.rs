use savecontrol_core::manifest::{Manifest, ManifestFile};

fn m() -> Manifest {
    Manifest::from_yaml(include_str!("fixtures/manifest.yaml")).unwrap()
}

fn paths(v: Vec<&ManifestFile>) -> Vec<&str> {
    v.iter().map(|f| f.path.as_str()).collect()
}

#[test]
fn le_campos_e_ignora_desconhecidos() {
    let m = m();
    assert_eq!(m.games.len(), 4);
    let a = &m.games["Jogo A"];
    assert_eq!(paths(a.save_files_for_windows(false)), ["<winAppData>/JogoA/saves"]);
    assert_eq!(a.save_files_for_windows(true).len(), 2);
    assert_eq!(a.install_dirs, ["JogoA"]);
    assert_eq!(a.steam_id, Some(1000));
    assert!(a.cloud.steam && a.has_registry);
}

#[test]
fn config_so_quando_pedido() {
    let m = m();
    assert!(m.games["Jogo B"].save_files_for_windows(false).is_empty());
    assert_eq!(m.games["Jogo B"].save_files_for_windows(true).len(), 1);
}

#[test]
fn alias_aponta_o_jogo_real() {
    assert_eq!(m().entry("Jogo C").unwrap().0, "Jogo A");
}

#[test]
fn sem_tag_conta_como_save() {
    let m = Manifest::from_yaml("Jogo E:\n  files:\n    <winAppData>/E/save: {}\nJogo F:\n").unwrap();
    assert_eq!(m.games["Jogo E"].save_files_for_windows(false).len(), 1);
    assert!(m.games["Jogo F"].files.is_empty());
}

#[test]
fn so_outros_sistemas_fica_de_fora() {
    assert!(m().games["Jogo D"].save_files_for_windows(true).is_empty());
}
