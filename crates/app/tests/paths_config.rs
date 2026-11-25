//! Pastas portáteis e config.json.
mod common;

use std::path::PathBuf;

use common::write;
use savecontrol_app::config::{AppConfig, AutoTimes, WindowState, load_config, save_config};
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

/// Um config.json do Plano 2 (sem os campos do automático) abre com os
/// valores do spec: retenção §8.2 e tempos §9, automático ligado.
#[test]
fn config_antiga_ganha_os_padroes() {
    let t = tempfile::tempdir().unwrap();
    let p = t.path().join("config.json");
    write(&p, br#"{"vault_dir": null, "first_run_done": true, "steam_covers": true, "include_config": false}"#);
    let c = load_config(&p).config;
    assert!(c.first_run_done);
    assert!(c.auto_backup);
    assert!(c.close_to_tray);
    assert!(!c.notifications, "as notificações começam desligadas (registro só quando o usuário liga)");
    let r = &c.retention;
    assert_eq!((r.auto_recent, r.auto_daily_days, r.auto_weekly_weeks, r.pre_restore_keep), (10, 7, 4, 5));
    let a = c.auto_times;
    assert_eq!((a.quiet_secs, a.after_close_secs, a.while_running_mins), (60, 10, 15));
}

#[test]
fn tempos_viram_regras_do_agendador() {
    use chrono::Duration;
    let r = AutoTimes { quiet_secs: 20, after_close_secs: 5, while_running_mins: 7 }.rules();
    assert_eq!((r.quiet, r.after_close, r.while_running), (Duration::seconds(20), Duration::seconds(5), Duration::minutes(7)));
}

#[test]
fn config_que_nao_da_para_ler_fica_so_leitura() {
    let t = tempfile::tempdir().unwrap();
    let p = t.path().join("config.json");
    // Uma pasta no lugar do arquivo: ler dá erro, mas não é "não existe".
    std::fs::create_dir_all(&p).unwrap();
    let l = load_config(&p);
    assert!(l.read_only, "uma config que não deu para ler seria sobrescrita");
    assert_eq!(l.warning.as_deref(), Some(texts::CONFIG_UNREADABLE));
    assert!(p.is_dir(), "a config original foi mexida");
}
