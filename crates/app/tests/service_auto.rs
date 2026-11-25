//! O serviço do app no Plano 3: backup automático, ajustes por jogo, fixar e
//! apagar, Atividade, "ocupado" e config só leitura.
mod common;

use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use common::{Mundo, mundo, pronto, servico, write};
use savecontrol_app::activity::ActivityKind;
use savecontrol_app::config::{WindowState, load_config};
use savecontrol_app::dto::*;
use savecontrol_app::paths::resolve_paths;
use savecontrol_app::service::{AppService, BusyFlag};
use savecontrol_app::texts;
use savecontrol_core::auto::Running;
use savecontrol_core::error::Result;
use savecontrol_core::manifest::store::{FetchResult, Fetcher};
use savecontrol_core::process::{FakeProbe, RunningProcess};
use savecontrol_core::registry::{FakeRegistry, Hive, RegistryReader};

/// Jogo à mão com um arquivo de save; devolve (id, pasta do save).
fn jogo(svc: &mut AppService, w: &Mundo, nome: &str, processos: &[&str]) -> (String, PathBuf) {
    let dir = w.sys.app_data.join(nome);
    write(&dir.join("slot.sav"), nome.as_bytes());
    let g = svc
        .add_game(AddGameRequest {
            name: nome.into(),
            save_dirs: vec![dir.display().to_string()],
            processes: processos.iter().map(|p| p.to_string()).collect(),
            ..Default::default()
        })
        .unwrap();
    (g.id, dir)
}

fn view(svc: &mut AppService, id: &str) -> GameView {
    svc.library().unwrap().games.into_iter().find(|g| g.id == id).unwrap()
}

fn config(w: &Mundo) -> savecontrol_app::config::AppConfig {
    load_config(&w.exe.join("SaveControl-dados").join("config.json")).config
}

#[test]
fn backup_automatico_registra_na_atividade() {
    let w = mundo();
    let mut svc = pronto(&w);
    let (a, _) = jogo(&mut svc, &w, "JogoA", &[]);
    let r = svc.auto_backup(&a).unwrap();
    assert_eq!((r.outcome, r.title.as_str(), r.file_count), (BackupOutcomeView::Feito, "JogoA", 1));
    assert_eq!(svc.history(&a).unwrap()[0].kind, SnapshotKindView::Auto);
    let at = svc.activity(10);
    assert_eq!((at[0].kind, at[0].game_id.as_deref()), (ActivityKind::BackupAuto, Some(a.as_str())));

    // Sem mudança: nenhum backup novo e nada novo na Atividade.
    assert_eq!(svc.auto_backup(&a).unwrap().outcome, BackupOutcomeView::SemMudanca);
    assert_eq!(svc.activity(10).iter().filter(|e| e.kind == ActivityKind::BackupAuto).count(), 1);
}

#[test]
fn acoes_manuais_entram_na_atividade() {
    let w = mundo();
    let mut svc = pronto(&w);
    let (a, dir) = jogo(&mut svc, &w, "JogoA", &[]);
    let m = svc.backup_game(&a, None).unwrap().snapshot.unwrap();
    write(&dir.join("slot.sav"), b"outro progresso");
    svc.restore(&a, &m.file_name, &RestoreRequest::default(), &FakeProbe(vec![])).unwrap();
    let kinds: Vec<ActivityKind> = svc.activity(10).iter().map(|e| e.kind).collect();
    assert_eq!(kinds, [ActivityKind::Restore, ActivityKind::BackupManual]);
}

#[test]
fn automatico_desligado_no_jogo_aparece_na_biblioteca() {
    let w = mundo();
    let mut svc = pronto(&w);
    let (a, _) = jogo(&mut svc, &w, "JogoA", &[]);
    assert!(view(&mut svc, &a).auto_backup);
    assert!(svc.can_back_up(&a));

    svc.set_game_auto(&a, false).unwrap();
    assert!(!view(&mut svc, &a).auto_backup);
    assert!(!svc.can_back_up(&a));

    svc.set_game_auto(&a, true).unwrap();
    svc.set_auto_backup(false).unwrap();
    assert!(view(&mut svc, &a).auto_backup, "a chave do jogo mostra o ajuste do jogo");
    assert!(!svc.can_back_up(&a), "com o automático geral desligado, nenhum jogo faz backup automático");
    assert!(!config(&w).auto_backup);
}

#[test]
fn retencao_ajustada_vale_no_proximo_backup() {
    let w = mundo();
    let mut svc = pronto(&w);
    let (a, dir) = jogo(&mut svc, &w, "JogoA", &[]);
    svc.set_retention(RetentionView { auto_recent: 2, auto_daily_days: 0, auto_weekly_weeks: 0 }).unwrap();
    for n in 0..4 {
        write(&dir.join("slot.sav"), format!("versão {n}").as_bytes());
        svc.auto_backup(&a).unwrap();
    }
    let autos = svc.history(&a).unwrap().iter().filter(|s| s.kind == SnapshotKindView::Auto).count();
    assert_eq!(autos, 2);
    let r = config(&w).retention;
    assert_eq!((r.auto_recent, r.pre_restore_keep), (2, 5), "\"antes de restaurar\" continua em 5");

    let so_do_jogo = RetentionView { auto_recent: 1, auto_daily_days: 0, auto_weekly_weeks: 0 };
    svc.set_game_retention(&a, Some(so_do_jogo.clone())).unwrap();
    assert_eq!(view(&mut svc, &a).retention, Some(so_do_jogo));
}

#[test]
fn fixar_e_apagar_pelo_servico() {
    let w = mundo();
    let mut svc = pronto(&w);
    let (a, dir) = jogo(&mut svc, &w, "JogoA", &[]);
    svc.auto_backup(&a).unwrap();
    write(&dir.join("slot.sav"), "segunda versão".as_bytes());
    svc.auto_backup(&a).unwrap();
    let velho = svc.history(&a).unwrap()[1].file_name.clone();

    let f = svc.pin_backup(&a, &velho, true).unwrap();
    assert_eq!(f.kind, SnapshotKindView::Fixado);
    assert_eq!(svc.history(&a).unwrap().iter().filter(|s| s.kind == SnapshotKindView::Fixado).count(), 1);

    svc.delete_backup(&a, &f.file_name).unwrap();
    assert_eq!(svc.history(&a).unwrap().len(), 1);
    assert_eq!(svc.activity(10)[0].kind, ActivityKind::Notice);
}

/// Anota se o "ocupado" estava ligado quando a Engine leu o registro.
struct RegistroEspia {
    base: FakeRegistry,
    busy: BusyFlag,
    viu_ocupado: Arc<AtomicBool>,
    leituras: Arc<AtomicUsize>,
}

impl RegistryReader for RegistroEspia {
    fn string(&self, hive: Hive, key: &str, value: &str) -> Option<String> {
        self.leituras.fetch_add(1, Ordering::SeqCst);
        if self.busy.is_busy() {
            self.viu_ocupado.store(true, Ordering::SeqCst);
        }
        self.base.string(hive, key, value)
    }

    fn subkeys(&self, hive: Hive, key: &str) -> Vec<String> {
        self.leituras.fetch_add(1, Ordering::SeqCst);
        if self.busy.is_busy() {
            self.viu_ocupado.store(true, Ordering::SeqCst);
        }
        self.base.subkeys(hive, key)
    }
}

/// Escanear não protege nenhum save: a janela tem de poder fechar durante ele.
#[test]
fn escanear_nao_liga_o_ocupado() {
    let w = mundo();
    let busy = BusyFlag::default();
    let viu = Arc::new(AtomicBool::new(false));
    let leituras = Arc::new(AtomicUsize::new(0));
    let (b, v, l) = (busy.clone(), viu.clone(), leituras.clone());
    let fabrica = Box::new(move || -> Box<dyn RegistryReader + Send> {
        let mut base = FakeRegistry::default();
        base.set(Hive::LocalMachine, r"SOFTWARE\Microsoft\Cryptography", "MachineGuid", "guid-teste");
        Box::new(RegistroEspia { base, busy: b.clone(), viu_ocupado: v.clone(), leituras: l.clone() })
    });
    let mut svc = AppService::new(resolve_paths(&w.exe, &w.lad), w.sys.clone(), fabrica, busy.clone());
    svc.finish_first_run(&w.exe.join("cofre")).unwrap();
    leituras.store(0, Ordering::SeqCst);
    svc.scan().unwrap();
    assert!(leituras.load(Ordering::SeqCst) > 0, "o escaneamento não leu o registro: o espião não serve");
    assert!(!viu.load(Ordering::SeqCst), "o escaneamento ligou o \"ocupado\"");
}

#[test]
fn sair_espera_o_backup_em_andamento() {
    let busy = BusyFlag::default();
    let guarda = busy.guard();
    let h = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(300));
        drop(guarda);
    });
    assert!(!busy.wait_idle(Duration::from_millis(50)), "disse que estava livre com o backup rodando");
    let t = Instant::now();
    assert!(busy.wait_idle(Duration::from_secs(2)));
    assert!(t.elapsed() >= Duration::from_millis(150), "não esperou o backup terminar");
    h.join().unwrap();
}

#[test]
fn config_so_leitura_nao_e_gravada() {
    let w = mundo();
    let cfg = w.exe.join("SaveControl-dados").join("config.json");
    std::fs::create_dir_all(&cfg).unwrap();
    let mut svc = servico(&w, BusyFlag::default());
    assert!(svc.state().warnings.iter().any(|x| x == texts::CONFIG_UNREADABLE));
    svc.save_window(WindowState { x: 1, y: 2, width: 1200, height: 800, maximized: false }).unwrap();
    assert!(cfg.is_dir(), "a config que não deu para ler foi sobrescrita");
}

/// Base de jogos de PC fixa, com um jogo que tem save e configuração.
struct BaseFixa(RefCell<Option<Vec<u8>>>);

impl Fetcher for BaseFixa {
    fn fetch(&self, _url: &str, _etag: Option<&str>) -> Result<FetchResult> {
        Ok(match self.0.borrow_mut().take() {
            Some(body) => FetchResult::Fetched { body, etag: Some("e1".into()) },
            None => FetchResult::NotModified,
        })
    }
}

#[test]
fn incluir_config_do_jogo_muda_os_arquivos() {
    let w = mundo();
    let mut svc = pronto(&w);
    write(&w.sys.app_data.join("JogoCfg").join("saves").join("slot.sav"), b"save");
    write(&w.sys.app_data.join("JogoCfg").join("opcoes.ini"), b"config");
    let yaml = "Jogo Cfg:\n  files:\n    <winAppData>/JogoCfg/saves:\n      tags: [save]\n    <winAppData>/JogoCfg/opcoes.ini:\n      tags: [config]\n";
    svc.update_manifest(&BaseFixa(RefCell::new(Some(yaml.as_bytes().to_vec())))).unwrap();
    svc.scan().unwrap();
    let id = "pc:Jogo Cfg";
    assert_eq!(view(&mut svc, id).file_count, 1);
    svc.set_game_include_config(id, Some(true)).unwrap();
    let v = view(&mut svc, id);
    assert_eq!((v.file_count, v.include_config), (2, Some(true)));
}

#[test]
fn mudanca_marca_save_novo_sem_hash() {
    let w = mundo();
    let mut svc = pronto(&w);
    let (a, _) = jogo(&mut svc, &w, "JogoA", &[]);
    svc.backup_game(&a, None).unwrap();
    let (b, _) = jogo(&mut svc, &w, "JogoB", &[]);
    assert_eq!(view(&mut svc, &a).status, StatusView::Protegido);
    svc.mark_changed(&a);
    svc.mark_changed(&b);
    assert_eq!(view(&mut svc, &a).status, StatusView::SaveNovo);
    assert_eq!(view(&mut svc, &b).status, StatusView::SemBackup);
}

#[test]
fn tempos_fora_da_faixa_dao_erro() {
    let w = mundo();
    let mut svc = pronto(&w);
    let t = |q, c, r| AutoTimesView { quiet_secs: q, after_close_secs: c, while_running_mins: r };
    assert!(svc.set_auto_times(t(2, 10, 15)).is_err());
    assert!(svc.set_auto_times(t(60, 601, 15)).is_err());
    assert!(svc.set_auto_times(t(60, 10, 121)).is_err());
    svc.set_auto_times(t(20, 5, 7)).unwrap();
    assert_eq!(svc.auto_rules().quiet, chrono::Duration::seconds(20));
    assert_eq!(svc.settings().unwrap().auto_times, t(20, 5, 7));
    assert_eq!(config(&w).auto_times.while_running_mins, 7);
}

#[test]
fn ajustes_da_janela_e_das_notificacoes_ficam_salvos() {
    let w = mundo();
    let mut svc = pronto(&w);
    assert!(svc.close_to_tray());
    svc.set_close_to_tray(false).unwrap();
    svc.set_notifications(true).unwrap();
    let c = config(&w);
    assert_eq!((c.close_to_tray, c.notifications), (false, true));
    let s = svc.settings().unwrap();
    assert_eq!((s.close_to_tray, s.notifications, s.start_with_windows), (false, true, false));
}

#[test]
fn estado_de_aberto_por_jogo() {
    let w = mundo();
    let mut svc = pronto(&w);
    let (com, _) = jogo(&mut svc, &w, "ComProcesso", &["Com.exe"]);
    let (fechado, _) = jogo(&mut svc, &w, "Fechado", &["Fechado.exe"]);
    let (sem, _) = jogo(&mut svc, &w, "SemProcesso", &[]);
    let probe = FakeProbe(vec![RunningProcess { name: "Com.exe".into(), exe_path: None }]);
    let estados = svc.running_states(&probe);
    let de = |id: &str| estados.iter().find(|(x, _)| x == id).map(|(_, r)| *r);
    assert_eq!((de(&com), de(&fechado), de(&sem)), (Some(Running::Yes), Some(Running::No), Some(Running::Unknown)));
}
