//! API da Engine usada pelo app (Plano 2).
mod common;

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::Path;

use common::write;
use savecontrol_core::{
    backup::BackupOutcome,
    custom::CustomGame,
    engine::{Engine, EngineConfig},
    env::SystemPaths,
    error::{Error, Result},
    local_state::LocalState,
    manifest::store::{FetchResult, Fetcher},
    process::{FakeProbe, RunningProcess},
    registry::{FakeRegistry, Hive},
    restore::RestoreOptions,
    status::GameStatus,
    vault::{Vault, format::SnapshotKind, index::SnapshotEntry},
};

fn reg(nome: &str) -> FakeRegistry {
    let mut reg = FakeRegistry::default();
    reg.set(Hive::LocalMachine, r"SOFTWARE\Microsoft\Cryptography", "MachineGuid", &format!("guid-{nome}"));
    reg
}

struct UmaVez(RefCell<Option<Vec<u8>>>);

impl UmaVez {
    fn com(texto: &str) -> UmaVez {
        UmaVez(RefCell::new(Some(texto.as_bytes().to_vec())))
    }
}

impl Fetcher for UmaVez {
    fn fetch(&self, _url: &str, _etag: Option<&str>) -> Result<FetchResult> {
        Ok(match self.0.borrow_mut().take() {
            Some(body) => FetchResult::Fetched { body, etag: Some("e1".into()) },
            None => FetchResult::NotModified,
        })
    }
}

fn engine(t: &Path, nome: &str) -> (SystemPaths, Engine) {
    let s = SystemPaths::under_home(&t.join(format!("home-{nome}")), nome);
    let cfg = EngineConfig { data_dir: t.join(format!("dados-{nome}")), vault_dir: t.join("cofre"), include_config: false };
    savecontrol_core::vault::Vault::create(&cfg.vault_dir).unwrap();
    let e = Engine::open(cfg, s.clone(), Box::new(reg(nome))).unwrap();
    (s, e)
}

fn criado(o: BackupOutcome) -> SnapshotEntry {
    match o {
        BackupOutcome::Created(e) => e,
        o => panic!("{o:?}"),
    }
}

fn norm(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/").trim_end_matches('/').to_lowercase()
}

fn ler(p: &Path) -> String {
    String::from_utf8_lossy(&std::fs::read(p).unwrap()).into_owned()
}

fn nenhum() -> FakeProbe {
    FakeProbe(vec![])
}

#[test]
fn engine_pode_ir_para_outra_thread() {
    fn send<T: Send>() {}
    send::<Engine>();
}

#[test]
fn ajustes_do_jogo_ficam_salvos() {
    let t = tempfile::tempdir().unwrap();
    let (s, mut e) = engine(t.path(), "a");
    let dir = s.app_data.join("JogoA");
    write(&dir.join("slot.sav"), b"1");
    let g = e.add_custom_game("Jogo A", &[dir.clone()], &[], None).unwrap();
    e.update_game_settings(&g.id, |st| st.title = Some("Meu nome".into())).unwrap();
    let jogos = t.path().join("Jogos");
    e.set_user_roots(vec![jogos.clone()]).unwrap();
    drop(e);

    let (_, e) = engine(t.path(), "a");
    assert_eq!(e.game_settings(&g.id).title.as_deref(), Some("Meu nome"));
    assert_eq!(e.user_roots(), [jogos]);
    assert_eq!(e.game_settings("pc:Nunca Visto"), Default::default());
}

#[test]
fn remover_jogo_manual_nao_volta_do_cofre() {
    let t = tempfile::tempdir().unwrap();
    let (s, mut e) = engine(t.path(), "a");
    let dir = s.app_data.join("JogoA");
    write(&dir.join("slot.sav"), b"1");
    let g = e.add_custom_game("Jogo A", &[dir], &[], None).unwrap();
    e.scan().unwrap();
    assert!(e.games().iter().any(|x| x.id == g.id));
    criado(e.backup(&g.id, SnapshotKind::Manual, None).unwrap());

    e.remove_custom_game(&g.id).unwrap();
    assert!(e.scan().unwrap().iter().all(|(x, _)| x.id != g.id));
    drop(e);

    let (_, mut e) = engine(t.path(), "a");
    assert!(e.scan().unwrap().iter().all(|(x, _)| x.id != g.id));
    assert!(e.game_settings(&g.id).ignored);
    assert!(e.ignored_ids().contains(&g.id));
    let local = LocalState::load(&t.path().join("dados-a/jogos.json")).unwrap();
    assert!(local.custom_games.iter().all(|c| c.id != g.id), "a definição voltou do cofre");
}

#[test]
fn reescanear_um_jogo_atualiza_o_status() {
    let t = tempfile::tempdir().unwrap();
    let (s, mut e) = engine(t.path(), "a");
    let slot = s.app_data.join("JogoA/slot.sav");
    write(&slot, b"v1");
    let g = e.add_custom_game("Jogo A", &[slot.parent().unwrap().to_path_buf()], &[], None).unwrap();
    e.scan().unwrap();
    criado(e.backup(&g.id, SnapshotKind::Manual, None).unwrap());
    assert_eq!(e.rescan_game(&g.id).unwrap().1, GameStatus::Protected);

    std::thread::sleep(std::time::Duration::from_millis(30));
    write(&slot, b"v2, depois de jogar");
    let (jogo, status) = e.rescan_game(&g.id).unwrap();
    assert_eq!(status, GameStatus::NewSave);
    assert_eq!(jogo.total_size, "v2, depois de jogar".len() as u64);
}

#[test]
fn previa_mostra_destino_tamanho_e_jogo_aberto() {
    let t = tempfile::tempdir().unwrap();
    let (s, mut e) = engine(t.path(), "a");
    let dir = s.app_data.join("JogoA");
    write(&dir.join("a.sav"), b"abc");
    write(&dir.join("b.sav"), b"12345");
    let g = e.add_custom_game("Jogo A", &[dir.clone()], &["JogoA.exe".into()], None).unwrap();
    e.scan().unwrap();
    let m = criado(e.backup(&g.id, SnapshotKind::Manual, None).unwrap());

    let p = e.restore_preview(&g.id, &m.file_name, &BTreeMap::new(), &nenhum()).unwrap();
    assert_eq!(p.entry.file_name, m.file_name);
    assert_eq!(p.targets.len(), 1);
    assert_eq!(p.targets[0].alias, "r0");
    assert_eq!(p.targets[0].template, "<winAppData>/JogoA");
    assert_eq!(norm(&p.targets[0].path), norm(&dir));
    assert_eq!((p.file_count, p.total_size, p.running), (2, 8, false));
    assert_eq!(e.snapshot_meta(&g.id, &m.file_name).unwrap().files.len(), 2);

    let aberto = FakeProbe(vec![RunningProcess { name: "JogoA.exe".into(), exe_path: None }]);
    assert!(e.restore_preview(&g.id, &m.file_name, &BTreeMap::new(), &aberto).unwrap().running);
}

/// No PC "a", um jogo à mão portátil (save dentro da pasta do jogo) com um backup.
fn portatil_no_pc_a(t: &Path) -> (CustomGame, SnapshotEntry) {
    let (_, mut a) = engine(t, "a");
    let gd = t.join("JogosA").join("Portatil");
    write(&gd.join("saves/s1.sav"), b"x");
    let g = a.add_custom_game("Portátil", &[gd.join("saves")], &[], Some(&gd)).unwrap();
    assert_eq!(g.save_templates, ["<gameDir>/saves"]);
    assert_eq!(a.game_settings(&g.id).game_dir, Some(gd.clone()));
    a.scan().unwrap();
    let m = criado(a.backup(&g.id, SnapshotKind::Manual, None).unwrap());
    (g, m)
}

#[test]
fn pasta_do_jogo_resolve_destino_em_outro_pc() {
    let t = tempfile::tempdir().unwrap();
    let (g, m) = portatil_no_pc_a(t.path());

    let (_, mut b) = engine(t.path(), "b");
    b.scan().unwrap();
    let err = b.restore_preview(&g.id, &m.file_name, &BTreeMap::new(), &nenhum()).unwrap_err();
    assert!(matches!(err, Error::NeedsTarget { ref root, .. } if root == "<gameDir>/saves"), "{err:?}");

    let gd_b = t.path().join("JogosB").join("Portatil");
    b.update_game_settings(&g.id, |s| s.game_dir = Some(gd_b.clone())).unwrap();
    b.scan().unwrap();
    let p = b.restore_preview(&g.id, &m.file_name, &BTreeMap::new(), &nenhum()).unwrap();
    assert_eq!(norm(&p.targets[0].path), norm(&gd_b.join("saves")));
}

#[test]
fn destino_escolhido_fica_lembrado() {
    let t = tempfile::tempdir().unwrap();
    let (g, m) = portatil_no_pc_a(t.path());

    let (_, mut b) = engine(t.path(), "b");
    b.scan().unwrap();
    let x = t.path().join("EscolhidoNoB");
    let opts = RestoreOptions {
        overrides: BTreeMap::from([("r0".to_string(), x.clone())]),
        remember: true,
        ..Default::default()
    };
    b.restore(&g.id, &m.file_name, &opts, &nenhum()).unwrap();
    assert_eq!(ler(&x.join("s1.sav")), "x");
    assert_eq!(b.game_settings(&g.id).target_choices.get("<gameDir>/saves"), Some(&x));

    let p = b.restore_preview(&g.id, &m.file_name, &BTreeMap::new(), &nenhum()).unwrap();
    assert_eq!(norm(&p.targets[0].path), norm(&x));
}

#[test]
fn resumo_do_cofre() {
    let t = tempfile::tempdir().unwrap();
    let (s, mut e) = engine(t.path(), "a");
    let slot = s.app_data.join("JogoA/slot.sav");
    write(&slot, b"v1");
    let g = e.add_custom_game("Jogo A", &[slot.parent().unwrap().to_path_buf()], &[], None).unwrap();
    e.scan().unwrap();
    criado(e.backup(&g.id, SnapshotKind::Manual, None).unwrap());
    write(&slot, b"v2 maior");
    criado(e.backup(&g.id, SnapshotKind::Manual, None).unwrap());

    let r = e.vault_summary();
    assert_eq!(norm(&r.path), norm(&t.path().join("cofre")));
    assert_eq!(norm(e.vault_dir()), norm(&t.path().join("cofre")));
    assert!(r.available);
    assert_eq!(r.versions, 2);
    let v = Vault::create(&t.path().join("cofre")).unwrap();
    let soma: u64 = e.history(&g.id).iter().map(|x| std::fs::metadata(x.zip_path(&v)).unwrap().len()).sum();
    assert_eq!(r.zip_bytes, soma);
    assert!(r.disk_total.is_some() && r.disk_free.is_some());

    std::fs::rename(t.path().join("cofre"), t.path().join("cofre-movido")).unwrap();
    assert!(!e.vault_summary().available);
}

const BASE_COM_CONFIG: &str = "Jogo C:\n  files:\n    <winAppData>/JogoC/save.dat:\n      tags: [save]\n    <winAppData>/JogoC/config.ini:\n      tags: [config]\n";

#[test]
fn incluir_configuracoes_vale_no_proximo_escaneamento() {
    let t = tempfile::tempdir().unwrap();
    let (s, mut e) = engine(t.path(), "a");
    write(&s.app_data.join("JogoC/save.dat"), b"1");
    write(&s.app_data.join("JogoC/config.ini"), b"2");
    e.update_manifest(&UmaVez::com(BASE_COM_CONFIG)).unwrap();
    let arquivos = |e: &mut Engine| {
        e.scan().unwrap().into_iter().find(|(g, _)| g.id == "pc:Jogo C").unwrap().0.files.len()
    };
    assert!(!e.include_config());
    assert_eq!(arquivos(&mut e), 1);
    e.set_include_config(true);
    assert!(e.include_config());
    assert_eq!(arquivos(&mut e), 2);
}

#[test]
fn has_manifest_so_depois_de_baixar() {
    let t = tempfile::tempdir().unwrap();
    let (_, mut e) = engine(t.path(), "a");
    assert!(!e.has_manifest());
    e.update_manifest(&UmaVez::com(BASE_COM_CONFIG)).unwrap();
    assert!(e.has_manifest());
}

#[test]
fn changed_games_ve_save_alterado() {
    let t = tempfile::tempdir().unwrap();
    let (s, mut e) = engine(t.path(), "a");
    let dir = s.app_data.join("JogoA");
    write(&dir.join("slot.sav"), b"1");
    let g = e.add_custom_game("Jogo A", &[dir.clone()], &[], None).unwrap();
    e.scan().unwrap();
    assert!(e.changed_games().is_empty(), "nada mudou e mesmo assim apareceu");
    write(&dir.join("slot.sav"), b"progresso novo, maior");
    assert_eq!(e.changed_games(), [g.id.clone()]);
    write(&dir.join("slot2.sav"), b"x");
    assert_eq!(e.changed_games(), [g.id]);
}

#[test]
fn games_for_path_acha_o_jogo_pela_raiz() {
    let t = tempfile::tempdir().unwrap();
    let (s, mut e) = engine(t.path(), "a");
    let dir = s.app_data.join("JogoA");
    write(&dir.join("slot.sav"), b"1");
    let g = e.add_custom_game("Jogo A", &[dir.clone()], &[], None).unwrap();
    e.scan().unwrap();
    assert_eq!(e.games_for_path(&dir.join("slot.sav")), [g.id.clone()]);
    let maiusculo = std::path::PathBuf::from(dir.join("SLOT.SAV").to_string_lossy().to_uppercase());
    assert_eq!(e.games_for_path(&maiusculo), [g.id]);
    assert!(e.games_for_path(&s.app_data.join("OutroJogo").join("x.sav")).is_empty());
    assert!(e.games_for_path(&s.app_data.join("JogoAB").join("x.sav")).is_empty(), "pasta com nome parecido não é do jogo");
}

#[test]
fn sabe_quando_da_para_ver_o_jogo_aberto() {
    let t = tempfile::tempdir().unwrap();
    let (s, mut e) = engine(t.path(), "a");
    let com = s.app_data.join("ComProcesso");
    let sem = s.app_data.join("SemProcesso");
    write(&com.join("slot.sav"), b"1");
    write(&sem.join("slot.sav"), b"1");
    let a = e.add_custom_game("Com processo", &[com], &["Jogo.exe".to_string()], None).unwrap();
    let b = e.add_custom_game("Sem processo", &[sem], &[], None).unwrap();
    e.scan().unwrap();
    let achar = |id: &str| e.games().iter().find(|g| g.id == id).unwrap().clone();
    assert!(savecontrol_core::process::can_tell_running(&achar(&a.id)));
    assert!(!savecontrol_core::process::can_tell_running(&achar(&b.id)));
}
