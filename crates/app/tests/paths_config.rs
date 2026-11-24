//! Pastas portáteis e config.json.
mod common;

use std::path::PathBuf;

use common::write;
use savecontrol_app::config::{AppConfig, WindowState, load_config, save_config};
use savecontrol_app::paths::resolve_paths;
use savecontrol_app::texts;

#[test]
fn dados_ao_lado_do_exe() {
    let t = tempfile::tempdir().unwrap();
    let exe = t.path().join("App");
    std::fs::create_dir_all(&exe).unwrap();
    let lad = t.path().join("lad");
    let p = resolve_paths(&exe, &lad);
    assert_eq!(p.exe_dir, exe);
    assert_eq!(p.data_dir, exe.join("SaveControl-dados"));
    assert_eq!(p.default_vault, exe.join("cofre"));
    assert!(!p.fallback);
    assert!(p.data_dir.is_dir());
    assert!(!p.data_dir.join(".teste-escrita").exists());
    assert!(!lad.exists(), "não pode criar nada no AppData");
}

#[test]
fn pasta_sem_escrita_usa_localappdata() {
    let t = tempfile::tempdir().unwrap();
    // Um arquivo no lugar da pasta: criar SaveControl-dados dentro dele falha.
    let exe = t.path().join("arquivo.txt");
    write(&exe, b"nao sou pasta");
    let lad = t.path().join("lad");
    let p = resolve_paths(&exe, &lad);
    assert_eq!(p.data_dir, lad.join("SaveControl"));
    assert_eq!(p.default_vault, lad.join("SaveControl").join("cofre"));
    assert!(p.fallback);
}

#[test]
fn config_ausente_vira_padrao() {
    let t = tempfile::tempdir().unwrap();
    let l = load_config(&t.path().join("config.json"));
    assert!(l.config.steam_covers);
    assert!(!l.config.first_run_done);
    assert!(!l.config.include_config);
    assert_eq!(l.config, AppConfig::default());
    assert!(l.warning.is_none());
}

#[test]
fn config_corrompida_volta_ao_padrao_e_avisa() {
    let t = tempfile::tempdir().unwrap();
    let f = t.path().join("config.json");
    write(&f, b"{lixo");
    let l = load_config(&f);
    assert_eq!(l.config, AppConfig::default());
    assert_eq!(l.warning.as_deref(), Some(texts::CONFIG_RESET));
    assert_eq!(std::fs::read_to_string(t.path().join("config.json.corrompido")).unwrap(), "{lixo");
}

#[test]
fn config_salva_e_rele() {
    let t = tempfile::tempdir().unwrap();
    let f = t.path().join("dados").join("config.json");
    let c = AppConfig {
        vault_dir: Some(PathBuf::from(r"E:\Backups\cofre")),
        first_run_done: true,
        steam_covers: false,
        window: Some(WindowState { x: 40, y: -8, width: 1280, height: 800, maximized: true }),
        ..Default::default()
    };
    save_config(&f, &c).unwrap();
    let l = load_config(&f);
    assert_eq!(l.config, c);
    assert!(l.warning.is_none());
}
