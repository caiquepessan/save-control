//! Regressões da revisão final: cada teste reproduz um problema encontrado
//! (perda de save ou de backup) e exige o comportamento seguro.
mod common;

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Duration, Local};
use common::{local, write};
use savecontrol_core::{
    backup::*,
    custom::CustomGame,
    engine::{Engine, EngineConfig},
    env::SystemPaths,
    error::{Error, Result},
    fingerprint::HashCache,
    fsglob::{FindResult, find_files},
    local_state::LocalState,
    machine::MachineIdentity,
    manifest::Manifest,
    manifest::store::{FetchResult, Fetcher, UpdateOutcome},
    model::Game,
    placeholder::{ExpandCtx, expand},
    process::*,
    registry::{FakeRegistry, Hive},
    restore::*,
    scan::{ScanInput, scan},
    status::GameStatus,
    stores::{InstalledGame, Installs, Store, detect_installs},
    targets::{TargetCtx, resolve_targets},
    vault::{
        Vault,
        format::*,
        index::{SnapshotEntry, VaultIndex, entry_from_meta},
        read::{extract_file, read_meta},
        retention::RetentionPolicy,
        write::{CopyHooks, NoWait, SourceFile, write_snapshot},
    },
};

fn ler(p: &Path) -> String {
    String::from_utf8_lossy(&std::fs::read(p).unwrap()).into_owned()
}

fn reg(nome: &str) -> FakeRegistry {
    let mut reg = FakeRegistry::default();
    reg.set(Hive::LocalMachine, r"SOFTWARE\Microsoft\Cryptography", "MachineGuid", &format!("guid-{nome}"));
    reg
}

struct UmaVez(RefCell<Option<Vec<u8>>>);

impl Fetcher for UmaVez {
    fn fetch(&self, _url: &str, _etag: Option<&str>) -> Result<FetchResult> {
        Ok(match self.0.borrow_mut().take() {
            Some(body) => FetchResult::Fetched { body, etag: Some("e1".into()) },
            None => FetchResult::NotModified,
        })
    }
}

const BASE: &str = "Jogo Teste:\n  files:\n    <winAppData>/JogoTeste:\n      tags: [save]\n";
const ID: &str = "pc:Jogo Teste";

fn base() -> UmaVez {
    UmaVez(RefCell::new(Some(BASE.as_bytes().to_vec())))
}

fn engine(t: &Path, nome: &str) -> (SystemPaths, Engine) {
    let s = SystemPaths::under_home(&t.join(format!("home-{nome}")), nome);
    let cfg = EngineConfig { data_dir: t.join(format!("dados-{nome}")), vault_dir: t.join("cofre"), include_config: false };
    savecontrol_core::vault::Vault::create(&cfg.vault_dir).unwrap();
    let e = Engine::open(cfg, s.clone(), Box::new(reg(nome))).unwrap();
    (s, e)
}

/// Conteúdo de um arquivo dentro do backup "antes de restaurar" mais novo do jogo.
fn no_seguro(e: &Engine, vault_dir: &Path, id: &str, alias: &str, rel: &str, tmp: &Path) -> String {
    let seg = e.history(id).into_iter().find(|x| x.kind == SnapshotKind::PreRestore).expect("sem antes-de-restaurar");
    let v = Vault::create(vault_dir).unwrap();
    extract_file(&seg.zip_path(&v), alias, rel, tmp).unwrap();
    ler(tmp)
}

// ---------------------------------------------------------------------------
// Mundo de baixo nível
struct Mundo {
    t: tempfile::TempDir,
    s: SystemPaths,
    vault: Vault,
    index: VaultIndex,
    hashes: HashCache,
    pc: MachineIdentity,
}

fn mundo() -> Mundo {
    let t = tempfile::tempdir().unwrap();
    let s = SystemPaths::under_home(&t.path().join("home"), "teste");
    let vault = Vault::create(&t.path().join("cofre")).unwrap();
    let pc = MachineIdentity { id: "aaaa111122223333".into(), pc_name: "PC".into(), user_name: "teste".into() };
    Mundo { t, s, vault, index: VaultIndex::default(), hashes: HashCache::default(), pc }
}

fn cg(id: &str, template: &str) -> CustomGame {
    CustomGame { id: id.into(), name: id.into(), save_templates: vec![template.into()], process_names: vec![] }
}

fn um(w: &Mundo, c: &CustomGame) -> Game {
    let local = LocalState { custom_games: vec![c.clone()], ..Default::default() };
    let input =
        ScanInput { sys: &w.s, manifest: None, installs: &Installs::default(), catalog: &[], local: &local, include_config: false };
    scan(&input).into_iter().find(|g| g.id == c.id).unwrap()
}

fn backup_em(w: &mut Mundo, g: &Game, kind: SnapshotKind, now: DateTime<Local>) -> BackupOutcome {
    let policy = RetentionPolicy::default();
    let mut ctx = BackupCtx {
        vault: &w.vault,
        index: &mut w.index,
        hashes: &mut w.hashes,
        machine: &w.pc,
        hooks: &NoWait,
        now,
        app_version: "0.1.0",
        retention: &policy,
    };
    backup_game(&mut ctx, g, kind, None).unwrap()
}

fn restaurar_com(
    w: &mut Mundo,
    g: &Game,
    e: &SnapshotEntry,
    opts: &RestoreOptions,
    users: Vec<String>,
    now: DateTime<Local>,
    hooks: &dyn CopyHooks,
) -> std::result::Result<RestoreReport, Error> {
    let policy = RetentionPolicy::default();
    let lembrado = BTreeMap::new();
    let alvo = TargetCtx { sys: &w.s, installs: vec![], store_user_ids: users, game_dirs: vec![], remembered: &lembrado };
    let mut ctx = BackupCtx {
        vault: &w.vault,
        index: &mut w.index,
        hashes: &mut w.hashes,
        machine: &w.pc,
        hooks,
        now,
        app_version: "0.1.0",
        retention: &policy,
    };
    restore_snapshot(&mut ctx, &alvo, &FakeProbe(vec![]), g, &[], e, opts)
}

fn restaurar_em(
    w: &mut Mundo,
    g: &Game,
    e: &SnapshotEntry,
    opts: &RestoreOptions,
    users: Vec<String>,
    now: DateTime<Local>,
) -> std::result::Result<RestoreReport, Error> {
    restaurar_com(w, g, e, opts, users, now, &NoWait)
}

fn criado(o: BackupOutcome) -> SnapshotEntry {
    match o {
        BackupOutcome::Created(e) => e,
        o => panic!("{o:?}"),
    }
}

// ---------------------------------------------------------------------------
// C1 (a_): recomp baixado numa segunda pasta mantém o mesmo jogo, e restaurar
// guarda o save atual e respeita o jogo aberto.
#[test]
fn a_recomp_em_duas_pastas_continua_o_mesmo_jogo() {
    let t = tempfile::tempdir().unwrap();
    let (s, mut e) = engine(t.path(), "a");
    let jogos = t.path().join("Jogos");
    write(&jogos.join("Zelda v1/Zelda64Recompiled.exe"), b"");
    let save = s.local_app_data.join("Zelda64Recompiled/saves/save.bin");
    write(&save, b"v1");
    e.set_user_roots(vec![jogos.clone()]).unwrap();
    e.scan().unwrap();
    let m = criado(e.backup("cat:zelda64recomp", SnapshotKind::Manual, None).unwrap());

    write(&jogos.join("Zelda v2/Zelda64Recompiled.exe"), b"");
    write(&save, b"progresso novo");
    let r = e.scan().unwrap();
    let zeldas: Vec<_> = r.iter().filter(|(g, _)| g.id.starts_with("cat:zelda64recomp")).collect();
    assert_eq!(zeldas.len(), 1);
    assert_eq!(zeldas[0].0.id, "cat:zelda64recomp");
    assert!(!zeldas[0].0.vault_only);
    assert_eq!(zeldas[0].1, GameStatus::NewSave);

    let aberto = FakeProbe(vec![RunningProcess { name: "Zelda64Recompiled.exe".into(), exe_path: None }]);
    let err = e.restore("cat:zelda64recomp", &m.file_name, &RestoreOptions::default(), &aberto).unwrap_err();
    assert!(matches!(err, Error::GameRunning(_)));
    assert_eq!(ler(&save), "progresso novo");

    let rep = e.restore("cat:zelda64recomp", &m.file_name, &RestoreOptions::default(), &FakeProbe(vec![])).unwrap();
    assert!(rep.safety.is_some());
    assert_eq!(ler(&save), "v1");
    let tmp = t.path().join("conf.bin");
    assert_eq!(no_seguro(&e, &t.path().join("cofre"), "cat:zelda64recomp", "r0", "save.bin", &tmp), "progresso novo");
}

// C1 (b_): outro PC sem a base, com save local: restaurar guarda o save local antes.
#[test]
fn b_outro_pc_sem_base_guarda_o_save_local() {
    let t = tempfile::tempdir().unwrap();
    let (sa, mut a) = engine(t.path(), "a");
    write(&sa.app_data.join("JogoTeste/slot1.sav"), b"v1");
    assert_eq!(a.update_manifest(&base()).unwrap(), UpdateOutcome::Updated);
    a.scan().unwrap();
    let m = criado(a.backup(ID, SnapshotKind::Manual, None).unwrap());
    drop(a);

    let (sb, mut b) = engine(t.path(), "b");
    let slot_b = sb.app_data.join("JogoTeste/slot1.sav");
    write(&slot_b, b"save local do PC B");
    let r = b.scan().unwrap();
    assert!(r.iter().find(|(g, _)| g.id == ID).unwrap().0.vault_only);
    let rep = b.restore(ID, &m.file_name, &RestoreOptions::default(), &FakeProbe(vec![])).unwrap();
    assert!(rep.safety.is_some());
    assert_eq!(ler(&slot_b), "v1");
    let tmp = t.path().join("conf.bin");
    assert_eq!(no_seguro(&b, &t.path().join("cofre"), ID, "r0", "slot1.sav", &tmp), "save local do PC B");
}

// C3 (c_): o save mudou depois do último scan; restaurar guarda a versão de agora.
#[test]
fn c_scan_velho_ainda_guarda_o_save_atual() {
    let t = tempfile::tempdir().unwrap();
    let (sa, mut a) = engine(t.path(), "a");
    let slot = sa.app_data.join("JogoTeste/slot1.sav");
    write(&slot, b"v1");
    a.update_manifest(&base()).unwrap();
    a.scan().unwrap();
    let m1 = criado(a.backup(ID, SnapshotKind::Manual, None).unwrap());
    write(&slot, b"versao 2");
    a.scan().unwrap();
    criado(a.backup(ID, SnapshotKind::Auto, None).unwrap());

    std::thread::sleep(std::time::Duration::from_millis(30));
    write(&slot, b"versao 3 - horas de jogo, sem scan");
    a.restore(ID, &m1.file_name, &RestoreOptions::default(), &FakeProbe(vec![])).unwrap();
    assert_eq!(ler(&slot), "v1");
    let tmp = t.path().join("conf.bin");
    assert_eq!(no_seguro(&a, &t.path().join("cofre"), ID, "r0", "slot1.sav", &tmp), "versao 3 - horas de jogo, sem scan");
}

// C3: backup automático com scan velho também enxerga o save de agora.
#[test]
fn c2_backup_com_scan_velho_pega_o_save_atual() {
    let t = tempfile::tempdir().unwrap();
    let (sa, mut a) = engine(t.path(), "a");
    let slot = sa.app_data.join("JogoTeste/slot1.sav");
    write(&slot, b"v1");
    a.update_manifest(&base()).unwrap();
    a.scan().unwrap();
    criado(a.backup(ID, SnapshotKind::Auto, None).unwrap());
    std::thread::sleep(std::time::Duration::from_millis(30));
    write(&slot, b"mudou sem scan");
    assert!(matches!(a.backup(ID, SnapshotKind::Auto, None).unwrap(), BackupOutcome::Created(_)));
}

// C2 (d1_): restaurar o "antes de restaurar" mais antigo não pode apagá-lo.
#[test]
fn d1_restaurar_o_antes_de_restaurar_mais_antigo() {
    let mut w = mundo();
    let c = cg("man:a", "<winAppData>/JogoA");
    let slot = w.s.app_data.join("JogoA/slot1.sav");
    let t0 = local(2026, 11, 20, 8, 0, 0);
    let mut pre = Vec::new();
    for i in 0..5 {
        write(&slot, format!("estado {i} {}", "x".repeat(i)).as_bytes());
        let g = um(&w, &c);
        pre.push(criado(backup_em(&mut w, &g, SnapshotKind::PreRestore, t0 + Duration::hours(i as i64))));
    }
    let alvo = pre[0].clone();
    write(&slot, b"save atual, diferente de todos");
    let g = um(&w, &c);
    restaurar_em(&mut w, &g, &alvo, &RestoreOptions::default(), vec![], t0 + Duration::days(1)).unwrap();
    assert!(alvo.zip_path(&w.vault).exists());
    assert_eq!(ler(&slot), "estado 0 ");
}

// C2 (d2_): automático antigo, guardado só pela regra semanal, restaurado meses depois.
#[test]
fn d2_restaurar_automatico_antigo() {
    let mut w = mundo();
    let c = cg("man:a", "<winAppData>/JogoA");
    let slot = w.s.app_data.join("JogoA/slot1.sav");
    let mut autos = Vec::new();
    let mut dias = vec![local(2026, 11, 2, 12, 0, 0)];
    dias.extend((9..=18).map(|d| local(2026, 11, d, 12, 0, 0)));
    for (i, dia) in dias.iter().enumerate() {
        write(&slot, format!("auto {i} {}", "y".repeat(i)).as_bytes());
        let g = um(&w, &c);
        autos.push(criado(backup_em(&mut w, &g, SnapshotKind::Auto, *dia)));
    }
    let alvo = autos[0].clone();
    write(&slot, b"save atual depois de meses");
    let g = um(&w, &c);
    restaurar_em(&mut w, &g, &alvo, &RestoreOptions::default(), vec![], local(2027, 2, 10, 12, 0, 0)).unwrap();
    assert!(alvo.zip_path(&w.vault).exists());
    assert_eq!(ler(&slot), "auto 0 ");
}

fn duas_contas(w: &Mundo) -> (Manifest, Installs) {
    let m = Manifest::from_yaml(
        "Jogo J:\n  files:\n    <winLocalAppData>/J/<storeUserId>:\n      tags: [save]\n  steam:\n    id: 1000\n",
    )
    .unwrap();
    let steam = w.t.path().join("Steam");
    let inst = Installs {
        steam_user_ids: vec!["111".into(), "222".into()],
        games: vec![InstalledGame {
            store: Store::Steam,
            root: steam.clone(),
            folder_name: "J".into(),
            base: steam.join("steamapps/common/J"),
            store_game_id: Some("1000".into()),
            display_name: None,
        }],
    };
    (m, inst)
}

// I1 (e_): duas contas da Steam; falha no meio; o desfazer devolve cada save à sua conta.
#[test]
fn e_desfazer_com_duas_contas_nao_mistura() {
    let mut w = mundo();
    let ra = w.s.local_app_data.join("J/111");
    let rb = w.s.local_app_data.join("J/222");
    write(&ra.join("save.dat"), b"A-v1");
    write(&rb.join("save.dat"), b"B-v1");
    write(&rb.join("slot2.dat"), b"x");
    let (m, inst) = duas_contas(&w);
    let escanear = |w: &Mundo| {
        let local = LocalState::default();
        scan(&ScanInput { sys: &w.s, manifest: Some(&m), installs: &inst, catalog: &[], local: &local, include_config: false })
            .remove(0)
    };
    let t0 = local(2026, 11, 25, 8, 0, 0);
    let g0 = escanear(&w);
    let m1 = criado(backup_em(&mut w, &g0, SnapshotKind::Manual, t0));
    write(&ra.join("save.dat"), b"A-v2");
    write(&rb.join("save.dat"), b"B-v2");
    std::fs::remove_file(rb.join("slot2.dat")).unwrap();
    std::fs::create_dir_all(rb.join("slot2.dat")).unwrap();
    let g = escanear(&w);
    let users = vec!["111".to_string(), "222".to_string()];
    assert!(restaurar_em(&mut w, &g, &m1, &RestoreOptions::default(), users, t0 + Duration::hours(1)).is_err());
    assert_eq!(ler(&ra.join("save.dat")), "A-v2");
    assert_eq!(ler(&rb.join("save.dat")), "B-v2");
}

// I1: duas raízes do backup não podem cair no mesmo destino em silêncio.
#[test]
fn e2_duas_contas_no_mesmo_destino_pedem_escolha() {
    let w = mundo();
    let root = |alias: &str, id: &str| SnapshotRoot {
        alias: alias.into(),
        template: "<winLocalAppData>/J/<storeUserId>".into(),
        bindings: BTreeMap::from([("storeUserId".to_string(), id.to_string())]),
        original_path: PathBuf::from(format!("X:/J/{id}")),
    };
    let meta = SnapshotMeta {
        format: FORMAT,
        game_id: "pc:Jogo J".into(),
        title: "Jogo J".into(),
        kind: SnapshotKind::Manual,
        label: None,
        created_at: chrono::Utc::now(),
        app_version: "0.1.0".into(),
        machine: w.pc.clone(),
        roots: vec![root("r0", "111"), root("r1", "222")],
        files: vec![],
    };
    let lembrado = BTreeMap::new();
    let ctx = TargetCtx { sys: &w.s, installs: vec![], store_user_ids: vec!["333".into()], game_dirs: vec![], remembered: &lembrado };
    assert!(matches!(resolve_targets(&meta, &ctx, &BTreeMap::new()), Err(Error::NeedsTarget { .. })));
}

// I3 (f_): jogo numa biblioteca secundária da Steam acha o save em <root>/userdata.
#[test]
fn f_steam_biblioteca_secundaria_acha_userdata() {
    let t = tempfile::tempdir().unwrap();
    let s = SystemPaths::under_home(&t.path().join("home"), "teste");
    let st = t.path().join("Steam");
    let lib2 = t.path().join("Lib2");
    let esc = |p: &Path| p.display().to_string().replace('\\', "\\\\");
    write(
        &st.join("steamapps/libraryfolders.vdf"),
        format!("\"libraryfolders\" {{ \"0\" {{ \"path\" \"{}\" }} \"1\" {{ \"path\" \"{}\" }} }}", esc(&st), esc(&lib2))
            .as_bytes(),
    );
    write(&st.join("userdata/12345/1000/remote/save.dat"), b"x");
    let m = Manifest::from_yaml(
        "Jogo S:\n  files:\n    <root>/userdata/<storeUserId>/<storeGameId>/remote:\n      tags: [save]\n  steam:\n    id: 1000\n",
    )
    .unwrap();
    let mut r = FakeRegistry::default();
    r.set(Hive::CurrentUser, r"Software\Valve\Steam", "SteamPath", &st.display().to_string());
    let local = LocalState::default();
    let achados = |onde: &Path| {
        let acf = onde.join("steamapps/appmanifest_1000.acf");
        write(&acf, b"\"AppState\" { \"appid\" \"1000\" \"name\" \"Jogo S\" \"installdir\" \"JogoS\" }");
        let i = detect_installs(&s, &r, &t.path().join("sem-epic"), &[]);
        let g = scan(&ScanInput { sys: &s, manifest: Some(&m), installs: &i, catalog: &[], local: &local, include_config: false });
        std::fs::remove_file(&acf).unwrap();
        g[0].files.len()
    };
    assert_eq!((achados(&st), achados(&lib2)), (1, 1));
}

// C1 (g_): restaurar com destino forçado numa pasta que já tem save guarda esse save antes.
#[test]
fn g_destino_forcado_guarda_o_que_estava_la() {
    let mut w = mundo();
    let c = cg("man:a", "<winAppData>/JogoA");
    write(&w.s.app_data.join("JogoA/slot1.sav"), b"v1");
    let t0 = local(2026, 11, 25, 8, 0, 0);
    let g0 = um(&w, &c);
    let m1 = criado(backup_em(&mut w, &g0, SnapshotKind::Manual, t0));
    let destino = w.t.path().join("PastaDoJogoNestePC/saves");
    write(&destino.join("slot1.sav"), b"save que ja estava no destino");
    let g = um(&w, &c);
    let opts = RestoreOptions { overrides: BTreeMap::from([("r0".to_string(), destino.clone())]), ..Default::default() };
    let r = restaurar_em(&mut w, &g, &m1, &opts, vec![], t0 + Duration::hours(1)).unwrap();
    assert_eq!(ler(&destino.join("slot1.sav")), "v1");
    let seg = r.safety.expect("sem backup de segurança");
    let tmp = w.t.path().join("conf.bin");
    extract_file(&seg.zip_path(&w.vault), "r0", "slot1.sav", &tmp).unwrap();
    assert_eq!(ler(&tmp), "save que ja estava no destino");
}

// I5 (j_): pastas amplas que escapavam da trava.
#[test]
fn j_pastas_amplas_sao_ignoradas() {
    let t = tempfile::tempdir().unwrap();
    let s = SystemPaths::under_home(&t.path().join("home"), "teste");
    write(&s.app_data.join("OutroPrograma/dados.db"), b"1");
    write(&s.local_app_data.join("Navegador/perfil.db"), b"1");
    write(&s.documents.join("My Games/JogoX/save.dat"), b"1");
    write(&s.documents.join("My Games/JogoY/save.dat"), b"1");
    let resultado = |tpl: &str| {
        let ctx = ExpandCtx { sys: &s, installs: &[], store_user_ids: &[], game_dir: None, game_name: "x" };
        let e = expand(tpl, &ctx).remove(0);
        find_files(&e, &s)
    };
    for tpl in ["<home>/AppData", "<home>/AppData/*/*", "<winDocuments>/My Games", "<winDocuments>/.", "<winAppData>/../Local"] {
        assert_eq!(resultado(tpl), FindResult::IgnoredBroad, "{tpl}");
    }
    assert!(matches!(resultado("<winDocuments>/My Games/JogoX"), FindResult::Found(_)));
}

// C1 (k_): só-no-cofre, restauração falha no meio: o save local que já existia volta.
#[test]
fn k_desfazer_devolve_o_save_que_ja_existia() {
    let t = tempfile::tempdir().unwrap();
    let (sa, mut a) = engine(t.path(), "a");
    write(&sa.app_data.join("JogoTeste/slot1.sav"), b"v1");
    write(&sa.app_data.join("JogoTeste/slot2.sav"), b"s2");
    a.update_manifest(&base()).unwrap();
    a.scan().unwrap();
    let m = criado(a.backup(ID, SnapshotKind::Manual, None).unwrap());
    drop(a);

    let (sb, mut b) = engine(t.path(), "b");
    let slot_b = sb.app_data.join("JogoTeste/slot1.sav");
    write(&slot_b, b"save local do PC B");
    std::fs::create_dir_all(sb.app_data.join("JogoTeste/slot2.sav")).unwrap();
    b.scan().unwrap();
    assert!(b.restore(ID, &m.file_name, &RestoreOptions::default(), &FakeProbe(vec![])).is_err());
    assert_eq!(ler(&slot_b), "save local do PC B");
}

// I2 (a2_): recomp portátil + saves no AppData continuam sendo um jogo só, com o histórico.
#[test]
fn a2_portatil_mais_appdata_continua_um_jogo_so() {
    let t = tempfile::tempdir().unwrap();
    let (s, mut e) = engine(t.path(), "a");
    let jogos = t.path().join("Jogos");
    let dir = jogos.join("Banjo Recompiled");
    write(&dir.join("BanjoRecompiled.exe"), b"");
    write(&dir.join("portable.txt"), b"");
    write(&dir.join("saves/save.bin"), b"portatil v1");
    e.set_user_roots(vec![jogos.clone()]).unwrap();
    e.scan().unwrap();
    criado(e.backup("cat:banjorecomp", SnapshotKind::Manual, None).unwrap());
    write(&s.local_app_data.join("BanjoRecompiled/saves/save.bin"), b"appdata");
    let r = e.scan().unwrap();
    let banjos: Vec<_> = r.iter().filter(|(g, _)| g.id.starts_with("cat:banjorecomp")).collect();
    assert_eq!(banjos.len(), 1);
    let mut templates: Vec<_> = banjos[0].0.save_roots.iter().map(|r| r.template.clone()).collect();
    templates.sort();
    assert_eq!(templates, ["<gameDir>/saves", "<winLocalAppData>/BanjoRecompiled/saves"]);
    assert_eq!(e.history("cat:banjorecomp").len(), 1);
}

// I2 (l_): a base renomeia o título e deixa alias: o histórico continua no jogo.
#[test]
fn l_titulo_renomeado_mantem_o_historico() {
    let t = tempfile::tempdir().unwrap();
    let (sa, mut a) = engine(t.path(), "a");
    let slot = sa.app_data.join("JogoTeste/slot1.sav");
    write(&slot, b"v1");
    a.update_manifest(&base()).unwrap();
    a.scan().unwrap();
    let m = criado(a.backup(ID, SnapshotKind::Manual, None).unwrap());

    std::fs::remove_file(t.path().join("dados-a/base/ultima-verificacao.txt")).unwrap();
    let nova = "Jogo Teste - Edicao Definitiva:\n  files:\n    <winAppData>/JogoTeste:\n      tags: [save]\nJogo Teste:\n  alias: Jogo Teste - Edicao Definitiva\n";
    assert_eq!(a.update_manifest(&UmaVez(RefCell::new(Some(nova.as_bytes().to_vec())))).unwrap(), UpdateOutcome::Updated);
    write(&slot, b"progresso de hoje");
    let r = a.scan().unwrap();
    let novo_id = "pc:Jogo Teste - Edicao Definitiva";
    assert!(r.iter().all(|(g, _)| g.id != ID));
    assert_eq!(r.iter().find(|(g, _)| g.id == novo_id).unwrap().1, GameStatus::NewSave);
    assert_eq!(a.history(novo_id).len(), 1);
    let rep = a.restore(novo_id, &m.file_name, &RestoreOptions::default(), &FakeProbe(vec![])).unwrap();
    assert!(rep.safety.is_some());
    assert_eq!(ler(&slot), "v1");
}

// I4: o backup lê e grava um arquivo por vez (não junta tudo na memória).
struct ConfereParcial(PathBuf, RefCell<Vec<u64>>);

impl CopyHooks for ConfereParcial {
    fn after_read(&self, _path: &Path) {
        let tamanho = std::fs::read_dir(&self.0)
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().ends_with(".parcial"))
            // No Windows o tamanho vindo da listagem da pasta só atualiza quando o
            // arquivo fecha; `fs::metadata` abre o arquivo e lê o tamanho de agora.
            .map(|e| std::fs::metadata(e.path()).unwrap().len())
            .sum();
        self.1.borrow_mut().push(tamanho);
    }
}

#[test]
fn backup_le_e_grava_um_arquivo_por_vez() {
    let mut w = mundo();
    let dir = w.s.app_data.join("JogoA");
    write(&dir.join("a.sav"), &vec![7u8; 4096]);
    write(&dir.join("b.sav"), &vec![9u8; 4096]);
    let c = cg("man:a", "<winAppData>/JogoA");
    let g = um(&w, &c);
    let pasta = w.vault.game_dir("man:a", "man:a").unwrap();
    let hooks = ConfereParcial(pasta, RefCell::new(vec![]));
    let policy = RetentionPolicy::default();
    let mut ctx = BackupCtx {
        vault: &w.vault,
        index: &mut w.index,
        hashes: &mut w.hashes,
        machine: &w.pc,
        hooks: &hooks,
        now: local(2026, 11, 25, 8, 0, 0),
        app_version: "0.1.0",
        retention: &policy,
    };
    criado(backup_game(&mut ctx, &g, SnapshotKind::Manual, None).unwrap());
    let tamanhos = hooks.1.borrow().clone();
    assert_eq!(tamanhos.len(), 2);
    assert!(tamanhos[1] > tamanhos[0], "o 1º arquivo já devia estar no zip ao ler o 2º: {tamanhos:?}");
}

// I6: formato mais novo não é aberto nem sobrescrito.
#[test]
fn cofre_e_backup_de_formato_novo_sao_recusados() {
    let t = tempfile::tempdir().unwrap();
    let cofre = t.path().join("cofre");
    std::fs::create_dir_all(&cofre).unwrap();
    std::fs::write(cofre.join("cofre.json"), br#"{"format": 99, "id": "x"}"#).unwrap();
    assert!(matches!(Vault::open(&cofre), Err(Error::FormatTooNew { found: 99, .. })));

    let w = mundo();
    let src = w.t.path().join("src");
    write(&src.join("a.sav"), b"1");
    let dir = w.vault.game_dir("pc:J", "J").unwrap();
    let meta = SnapshotMeta {
        format: FORMAT + 1,
        game_id: "pc:J".into(),
        title: "J".into(),
        kind: SnapshotKind::Manual,
        label: None,
        created_at: chrono::Utc::now(),
        app_version: "9.0.0".into(),
        machine: w.pc.clone(),
        roots: vec![SnapshotRoot { alias: "r0".into(), template: "<winAppData>/J".into(), bindings: Default::default(), original_path: "C:/J".into() }],
        files: vec![],
    };
    let fonte = [SourceFile { abs_path: src.join("a.sav"), root_alias: "r0".into(), rel_path: "a.sav".into() }];
    let (zip, _) = write_snapshot(&dir, "2026-11-25_08-00-00_PC-AAAA_manual.zip", meta, &fonte, &NoWait).unwrap();
    assert!(matches!(read_meta(&zip), Err(Error::FormatTooNew { .. })));
}

#[test]
fn jogo_json_ilegivel_nao_e_pasta_livre() {
    let w = mundo();
    let ocupada = w.vault.root.join("Jogo");
    std::fs::create_dir_all(&ocupada).unwrap();
    std::fs::write(ocupada.join("jogo.json"), b"{ formato que esta versao nao entende").unwrap();
    assert_ne!(w.vault.game_dir("pc:Outro", "Jogo").unwrap(), ocupada);
}

// Caminho no zip que tenta sair da pasta de destino é recusado.
#[test]
fn zip_com_caminho_que_escapa_e_recusado() {
    let mut w = mundo();
    let c = cg("man:a", "<winAppData>/JogoA");
    let jogo_dir = w.s.app_data.join("JogoA");
    write(&jogo_dir.join("slot1.sav"), b"atual");
    let src = w.t.path().join("src");
    write(&src.join("x.txt"), b"malicioso");
    let g = um(&w, &c);
    let dir = w.vault.game_dir("man:a", "man:a").unwrap();
    let meta = SnapshotMeta {
        format: FORMAT,
        game_id: "man:a".into(),
        title: "man:a".into(),
        kind: SnapshotKind::Manual,
        label: None,
        created_at: chrono::Utc::now(),
        app_version: "0.1.0".into(),
        machine: w.pc.clone(),
        roots: vec![SnapshotRoot { alias: "r0".into(), template: "<winAppData>/JogoA".into(), bindings: Default::default(), original_path: jogo_dir.clone() }],
        files: vec![],
    };
    let fonte = [SourceFile { abs_path: src.join("x.txt"), root_alias: "r0".into(), rel_path: "../fora.txt".into() }];
    let nome = "2026-11-25_08-00-00_PC-AAAA_manual.zip";
    let (zip, meta) = write_snapshot(&dir, nome, meta, &fonte, &NoWait).unwrap();
    let entry = entry_from_meta(&meta, dir.file_name().unwrap().to_str().unwrap(), nome, SnapshotKind::Manual);
    assert!(zip.exists());
    let err = restaurar_em(&mut w, &g, &entry, &RestoreOptions::default(), vec![], local(2026, 11, 25, 9, 0, 0)).unwrap_err();
    assert!(matches!(err, Error::UnsafeRestore(_)));
    assert!(!w.s.app_data.join("fora.txt").exists());
    assert_eq!(ler(&jogo_dir.join("slot1.sav")), "atual");
}

// I5: restauração que apagaria muitos arquivos fora do backup é recusada sem mexer em nada.
#[test]
fn restauracao_que_apagaria_muitos_arquivos_e_recusada() {
    let mut w = mundo();
    let c = cg("man:a", "<winAppData>/JogoA");
    let dir = w.s.app_data.join("JogoA");
    write(&dir.join("slot1.sav"), b"v1");
    let t0 = local(2026, 11, 25, 8, 0, 0);
    let g0 = um(&w, &c);
    let m1 = criado(backup_em(&mut w, &g0, SnapshotKind::Manual, t0));
    write(&dir.join("slot1.sav"), b"v2");
    for i in 0..30 {
        write(&dir.join(format!("outro-{i}.dat")), b"x");
    }
    let g = um(&w, &c);
    let err = restaurar_em(&mut w, &g, &m1, &RestoreOptions::default(), vec![], t0 + Duration::hours(1)).unwrap_err();
    assert!(matches!(err, Error::UnsafeRestore(_)));
    assert_eq!(ler(&dir.join("slot1.sav")), "v2");
    assert!((0..30).all(|i| dir.join(format!("outro-{i}.dat")).exists()));
}

// I7: quando nem o desfazer dá certo, o erro diz onde está o save de antes.
struct ApagaSeguranca(PathBuf);

impl CopyHooks for ApagaSeguranca {
    fn before_rollback(&self) {
        for d in std::fs::read_dir(&self.0).unwrap().flatten() {
            for f in std::fs::read_dir(d.path()).into_iter().flatten().flatten() {
                if f.file_name().to_string_lossy().contains("antes-de-restaurar") {
                    std::fs::remove_file(f.path()).unwrap();
                }
            }
        }
    }
}

#[test]
fn desfazer_que_falha_avisa_onde_esta_o_save() {
    let mut w = mundo();
    let c = cg("man:a", "<winAppData>/JogoA");
    let dir = w.s.app_data.join("JogoA");
    write(&dir.join("slot1.sav"), b"v1");
    write(&dir.join("slot2.sav"), b"s2");
    let t0 = local(2026, 11, 25, 8, 0, 0);
    let g0 = um(&w, &c);
    let m1 = criado(backup_em(&mut w, &g0, SnapshotKind::Manual, t0));
    write(&dir.join("slot1.sav"), b"versao 2");
    std::fs::remove_file(dir.join("slot2.sav")).unwrap();
    std::fs::create_dir_all(dir.join("slot2.sav")).unwrap();
    let g = um(&w, &c);
    let hooks = ApagaSeguranca(w.vault.root.clone());
    let err =
        restaurar_com(&mut w, &g, &m1, &RestoreOptions::default(), vec![], t0 + Duration::hours(1), &hooks).unwrap_err();
    let Error::RollbackFailed { safety, .. } = err else { panic!("{err:?}") };
    assert!(safety.contains("antes-de-restaurar"), "{safety}");
}
