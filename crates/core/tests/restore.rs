mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Duration, Local};
use common::{local, write};
use savecontrol_core::{
    backup::*,
    custom::CustomGame,
    env::SystemPaths,
    error::Error,
    fingerprint::HashCache,
    local_state::LocalState,
    machine::MachineIdentity,
    model::Game,
    process::*,
    restore::*,
    scan::{ScanInput, scan},
    stores::Installs,
    targets::TargetCtx,
    vault::{
        Vault,
        format::SnapshotKind,
        index::{SnapshotEntry, VaultIndex},
        read::{extract_file, read_meta},
        retention::RetentionPolicy,
        write::NoWait,
    },
};

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

fn escanear(w: &Mundo, jogos: &[CustomGame]) -> Vec<Game> {
    let local = LocalState { custom_games: jogos.to_vec(), ..Default::default() };
    let input = ScanInput {
        sys: &w.s,
        manifest: None,
        installs: &Installs::default(),
        catalog: &[],
        local: &local,
        include_config: false,
    };
    scan(&input)
}

fn um(w: &Mundo, c: &CustomGame) -> Game {
    escanear(w, std::slice::from_ref(c)).into_iter().find(|g| g.id == c.id).unwrap()
}

fn agora() -> DateTime<Local> {
    local(2026, 11, 25, 8, 0, 0)
}

fn backup(w: &mut Mundo, g: &Game) -> SnapshotEntry {
    let policy = RetentionPolicy::default();
    let mut ctx = BackupCtx {
        vault: &w.vault,
        index: &mut w.index,
        hashes: &mut w.hashes,
        machine: &w.pc,
        hooks: &NoWait,
        now: agora(),
        app_version: "0.1.0",
        retention: &policy,
    };
    match backup_game(&mut ctx, g, SnapshotKind::Manual, None).unwrap() {
        BackupOutcome::Created(e) => e,
        o => panic!("{o:?}"),
    }
}

fn restaurar(
    w: &mut Mundo,
    g: &Game,
    outros: &[Game],
    e: &SnapshotEntry,
    opts: &RestoreOptions,
    probe: &dyn ProcessProbe,
) -> Result<RestoreReport, Error> {
    let policy = RetentionPolicy::default();
    let lembrado = BTreeMap::new();
    let alvo = TargetCtx { sys: &w.s, installs: vec![], store_user_ids: vec![], game_dirs: vec![], remembered: &lembrado };
    let mut ctx = BackupCtx {
        vault: &w.vault,
        index: &mut w.index,
        hashes: &mut w.hashes,
        machine: &w.pc,
        hooks: &NoWait,
        now: agora() + Duration::hours(1),
        app_version: "0.1.0",
        retention: &policy,
    };
    restore_snapshot(&mut ctx, &alvo, probe, g, outros, e, opts)
}

fn ler(p: &Path) -> Vec<u8> {
    std::fs::read(p).unwrap()
}

/// slot1="v1" no backup; depois slot1="versao 2" e um slot2 novo.
fn cenario(w: &mut Mundo, c: &CustomGame) -> (SnapshotEntry, Game, PathBuf) {
    let dir = w.s.app_data.join("JogoA");
    write(&dir.join("slot1.sav"), b"v1");
    let m1 = backup(w, &um(w, c));
    write(&dir.join("slot1.sav"), b"versao 2");
    write(&dir.join("slot2.sav"), b"novo");
    (m1, um(w, c), dir)
}

#[test]
fn volta_exatamente_a_versao() {
    let mut w = mundo();
    let c = cg("man:a", "<winAppData>/JogoA");
    let (m1, g, dir) = cenario(&mut w, &c);
    let r = restaurar(&mut w, &g, &[], &m1, &RestoreOptions::default(), &FakeProbe(vec![])).unwrap();
    assert_eq!((r.written, r.removed), (1, 1));
    assert_eq!(ler(&dir.join("slot1.sav")), b"v1");
    assert!(!dir.join("slot2.sav").exists());
    let seguro = r.safety.unwrap().zip_path(&w.vault);
    let nomes: Vec<String> = read_meta(&seguro).unwrap().files.into_iter().map(|f| f.rel_path).collect();
    assert_eq!(nomes, ["slot1.sav", "slot2.sav"]);
    let tmp = w.t.path().join("conferir.sav");
    extract_file(&seguro, "r0", "slot1.sav", &tmp).unwrap();
    assert_eq!(ler(&tmp), b"versao 2");
}

#[test]
fn jogo_aberto_bloqueia_sem_mexer() {
    let mut w = mundo();
    let mut c = cg("man:a", "<winAppData>/JogoA");
    c.process_names = vec!["jogo.exe".into()];
    let (m1, g, dir) = cenario(&mut w, &c);
    let probe = FakeProbe(vec![RunningProcess { name: "JOGO.EXE".into(), exe_path: None }]);
    let err = restaurar(&mut w, &g, &[], &m1, &RestoreOptions::default(), &probe).unwrap_err();
    assert!(matches!(err, Error::GameRunning(_)));
    assert_eq!(ler(&dir.join("slot1.sav")), b"versao 2");
    assert!(w.index.for_game("man:a").iter().all(|e| e.kind != SnapshotKind::PreRestore));
}

#[test]
fn nao_apaga_arquivo_de_outro_jogo_na_mesma_pasta() {
    let mut w = mundo();
    let pasta = w.s.app_data.join("Emu/Save");
    write(&pasta.join("a.sav"), b"a1");
    let a = cg("man:a", "<winAppData>/Emu/Save");
    let b = cg("man:b", "<winAppData>/Emu/Save/b.sav");
    let ga0 = um(&w, &a);
    let ma = backup(&mut w, &ga0);
    write(&pasta.join("b.sav"), b"b1");
    let jogos = escanear(&w, &[a.clone(), b.clone()]);
    let ga = jogos.iter().find(|g| g.id == "man:a").unwrap().clone();
    let gb = jogos.iter().find(|g| g.id == "man:b").unwrap().clone();
    restaurar(&mut w, &ga, &[gb], &ma, &RestoreOptions::default(), &FakeProbe(vec![])).unwrap();
    assert!(pasta.join("b.sav").exists());
    assert_eq!(ler(&pasta.join("a.sav")), b"a1");
}

#[test]
fn restauracao_seletiva() {
    let mut w = mundo();
    let c = cg("man:a", "<winAppData>/JogoA");
    let (m1, g, dir) = cenario(&mut w, &c);
    let opts = RestoreOptions {
        only: Some(BTreeSet::from([("r0".to_string(), "slot1.sav".to_string())])),
        ..Default::default()
    };
    let r = restaurar(&mut w, &g, &[], &m1, &opts, &FakeProbe(vec![])).unwrap();
    assert_eq!(ler(&dir.join("slot1.sav")), b"v1");
    assert!(dir.join("slot2.sav").exists());
    assert_eq!(r.removed, 0);
}

#[test]
fn restaurar_em_outra_pasta() {
    let mut w = mundo();
    let c = cg("man:a", "<winAppData>/JogoA");
    let (m1, g, dir) = cenario(&mut w, &c);
    let destino = w.t.path().join("destino");
    let opts = RestoreOptions { overrides: BTreeMap::from([("r0".to_string(), destino.clone())]), ..Default::default() };
    restaurar(&mut w, &g, &[], &m1, &opts, &FakeProbe(vec![])).unwrap();
    assert_eq!(ler(&destino.join("slot1.sav")), b"v1");
    assert_eq!(ler(&dir.join("slot1.sav")), b"versao 2");
    assert!(dir.join("slot2.sav").exists());
}

#[test]
fn acentos_e_caminho_longo_ida_e_volta() {
    let mut w = mundo();
    let base = w.s.app_data.join("Ação Jogos").join("Usuário João");
    let mut fundo = base.clone();
    let mut i = 0;
    while fundo.join("save.dat").to_string_lossy().len() <= 300 {
        fundo = fundo.join(format!("pasta-com-nome-bem-comprido-{i:02}"));
        i += 1;
    }
    let arquivo = fundo.join("save.dat");
    write(&arquivo, b"progresso original");
    let c = cg("man:longo", "<winAppData>/Ação Jogos/Usuário João");
    let g0 = um(&w, &c);
    let m = backup(&mut w, &g0);
    write(&arquivo, b"estragado");
    let g = um(&w, &c);
    restaurar(&mut w, &g, &[], &m, &RestoreOptions::default(), &FakeProbe(vec![])).unwrap();
    assert!(arquivo.to_string_lossy().len() > 300);
    assert_eq!(ler(&arquivo), b"progresso original");
}

#[test]
fn falha_no_meio_desfaz() {
    let mut w = mundo();
    let c = cg("man:a", "<winAppData>/JogoA");
    let dir = w.s.app_data.join("JogoA");
    write(&dir.join("slot1.sav"), b"v1");
    write(&dir.join("slot2.sav"), b"s2");
    let g0 = um(&w, &c);
    let m1 = backup(&mut w, &g0);
    write(&dir.join("slot1.sav"), b"versao 2");
    std::fs::remove_file(dir.join("slot2.sav")).unwrap();
    std::fs::create_dir_all(dir.join("slot2.sav")).unwrap();
    let g = um(&w, &c);
    assert!(restaurar(&mut w, &g, &[], &m1, &RestoreOptions::default(), &FakeProbe(vec![])).is_err());
    assert_eq!(ler(&dir.join("slot1.sav")), b"versao 2");
    let sobras: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n.contains("savecontrol-tmp"))
        .collect();
    assert!(sobras.is_empty(), "temporários deixados: {sobras:?}");
}

#[test]
fn sondagem_real_ve_processos() {
    assert!(!SystemProbe.running().is_empty());
}
