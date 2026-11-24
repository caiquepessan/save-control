//! Visão da biblioteca: como um `Game` do núcleo aparece para a interface.
mod common;

use std::collections::BTreeMap;
use std::path::PathBuf;

use chrono::{TimeZone, Utc};
use common::write;
use savecontrol_app::covers::CoverStore;
use savecontrol_app::dto::*;
use savecontrol_app::view::{ViewCtx, game_view, snapshot_view};
use savecontrol_core::{
    catalog::Catalog,
    custom::CustomGame,
    env::SystemPaths,
    error::Error,
    fingerprint::Fingerprint,
    local_state::LocalGameSettings,
    model::{Game, GameSource, SaveFile, SaveRoot, Warning},
    status::GameStatus,
    vault::{format::SnapshotKind, index::SnapshotEntry},
};

struct Mundo {
    _t: tempfile::TempDir,
    sys: SystemPaths,
    catalog: Catalog,
    covers: CoverStore,
}

fn mundo() -> Mundo {
    let t = tempfile::tempdir().unwrap();
    let sys = SystemPaths::under_home(&t.path().join("home"), "teste");
    let covers = CoverStore { dir: t.path().join("capas") };
    Mundo { _t: t, sys, catalog: Catalog::builtin(), covers }
}

impl Mundo {
    fn ctx(&self) -> ViewCtx<'_> {
        ViewCtx { sys: &self.sys, catalog: &self.catalog, machine_id: "pc-aqui", covers: &self.covers }
    }
}

fn jogo(id: &str, title: &str, source: GameSource) -> Game {
    Game {
        id: id.into(),
        title: title.into(),
        source,
        install_dirs: vec![],
        game_dir: None,
        save_roots: vec![],
        files: vec![],
        last_saved: None,
        total_size: 0,
        steam_app_id: None,
        process_names: vec![],
        custom: None,
        warnings: vec![],
        vault_only: false,
    }
}

fn entrada(game_id: &str, machine_id: &str, dia: u32) -> SnapshotEntry {
    SnapshotEntry {
        game_id: game_id.into(),
        dir_name: "Jogo".into(),
        file_name: format!("2026-11-{dia:02}_10-00-00_PC-AAAA_manual.zip"),
        kind: SnapshotKind::Manual,
        label: None,
        created_at: Utc.with_ymd_and_hms(2026, 11, dia, 10, 0, 0).unwrap(),
        machine_id: machine_id.into(),
        machine_label: "NOTEBOOK-1-ABCD".into(),
        pc_name: "NOTEBOOK-1".into(),
        fingerprint: Fingerprint::default(),
        total_size: 10,
        file_count: 1,
    }
}

fn vazio() -> LocalGameSettings {
    LocalGameSettings::default()
}

#[test]
fn rotulo_e_tipo_pela_origem() {
    let w = mundo();
    let v = |g: &Game| game_view(&w.ctx(), g, GameStatus::NoSave, &[], &vazio());

    let steam = v(&jogo("pc:Jogo S", "Jogo S", GameSource::Steam));
    assert_eq!((steam.kind, steam.source_key, steam.source_label.as_str()), (GameKind::Pc, SourceKey::Steam, "Steam"));
    let epic = v(&jogo("pc:Jogo E", "Jogo E", GameSource::Epic));
    assert_eq!((epic.source_key, epic.source_label.as_str()), (SourceKey::Epic, "Epic Games"));
    let base = v(&jogo("pc:Jogo B", "Jogo B", GameSource::ManifestOnly));
    assert_eq!((base.kind, base.source_key, base.source_label.as_str()), (GameKind::Pc, SourceKey::Pc, "PC"));

    let banjo = v(&jogo("cat:banjorecomp", "Banjo: Recompiled", GameSource::Catalog));
    assert_eq!((banjo.kind, banjo.source_key, banjo.source_label.as_str()), (GameKind::Recomp, SourceKey::Catalogo, "Banjo: Recompiled"));
    let mario = v(&jogo("cat:project64:mario", "MARIO · Project64", GameSource::Catalog));
    assert_eq!((mario.kind, mario.source_label.as_str()), (GameKind::Emulador, "Project64"));

    let mut manual = jogo("man:1", "Meu Jogo", GameSource::Custom);
    manual.custom = Some(CustomGame { id: "man:1".into(), name: "Meu Jogo".into(), save_templates: vec![], process_names: vec![] });
    let m = v(&manual);
    assert_eq!((m.kind, m.source_key, m.source_label.as_str(), m.is_custom), (GameKind::Manual, SourceKey::Manual, "Adicionado à mão", true));

    let mut velho = jogo("pc:Velho", "Velho", GameSource::Steam);
    velho.vault_only = true;
    let o = v(&velho);
    assert!(o.vault_only);
    assert_eq!(o.source_label, "Só no cofre");
    assert!(!steam.is_custom);
}

#[test]
fn titulo_do_ajuste_tem_prioridade() {
    let w = mundo();
    let g = jogo("pc:Jogo S", "Jogo S", GameSource::Steam);
    let s = LocalGameSettings { title: Some("Meu nome".into()), ..Default::default() };
    let v = game_view(&w.ctx(), &g, GameStatus::NoSave, &[], &s);
    assert_eq!((v.title.as_str(), v.original_title.as_str()), ("Meu nome", "Jogo S"));
    assert_eq!(game_view(&w.ctx(), &g, GameStatus::NoSave, &[], &vazio()).title, "Jogo S");
}

#[test]
fn ultimo_backup_e_contagem_vem_do_historico() {
    let w = mundo();
    let mut g = jogo("man:1", "Jogo A", GameSource::Custom);
    let raiz = w.sys.app_data.join("JogoA");
    g.save_roots = vec![SaveRoot { template: "<winAppData>/JogoA".into(), bindings: BTreeMap::new(), path: raiz.clone() }];
    let quando = Utc.with_ymd_and_hms(2026, 11, 24, 9, 30, 0).unwrap();
    g.files = ["a.sav", "b.sav"]
        .iter()
        .map(|n| SaveFile { root: 0, rel_path: n.to_string(), abs_path: raiz.join(n), size: 4, modified: quando })
        .collect();
    g.total_size = 8;
    g.last_saved = Some(quando);
    let hist = [entrada("man:1", "pc-aqui", 22), entrada("man:1", "pc-aqui", 20)];
    let v = game_view(&w.ctx(), &g, GameStatus::NewSave, &hist, &vazio());
    assert_eq!(v.status, StatusView::SaveNovo);
    assert_eq!(v.backup_count, 2);
    assert_eq!(v.last_backup.as_deref(), Some("2026-11-22T10:00:00+00:00"));
    assert_eq!(v.last_saved.as_deref(), Some("2026-11-24T09:30:00+00:00"));
    assert_eq!((v.file_count, v.total_size), (2, 8));
    assert_eq!(v.roots.len(), 1);
    assert_eq!(v.roots[0].template, "<winAppData>/JogoA");
    assert_eq!(v.roots[0].files.iter().map(|f| f.rel_path.as_str()).collect::<Vec<_>>(), ["a.sav", "b.sav"]);
    assert!(!v.roots[0].path.contains('/'), "caminho para mostrar no Windows: {}", v.roots[0].path);
}

#[test]
fn avisos_viram_codigos() {
    let w = mundo();
    let mut g = jogo("pc:Jogo S", "Jogo S", GameSource::Steam);
    g.warnings = vec![
        Warning::RiskyLocation(w.sys.downloads.join("Jogo")),
        Warning::RiskyLocation(w.sys.desktop.join("Saves")),
        Warning::StoreCloud("steam".into()),
        Warning::InsideGameFolder,
        Warning::RegistryNotIncluded,
        Warning::IgnoredBroadPath("<home>".into()),
    ];
    let v = game_view(&w.ctx(), &g, GameStatus::NoSave, &[], &vazio());
    let got: Vec<(WarningKind, Option<&str>)> = v.warnings.iter().map(|x| (x.kind, x.detail.as_deref())).collect();
    assert_eq!(
        got,
        [
            (WarningKind::PastaArriscada, Some("Downloads")),
            (WarningKind::PastaArriscada, Some("Área de Trabalho")),
            (WarningKind::Nuvem, Some("Steam")),
            (WarningKind::DentroDaPastaDoJogo, None),
            (WarningKind::Registro, None),
            (WarningKind::PastaAmpla, Some("<home>")),
        ]
    );
}

#[test]
fn precisa_da_pasta_do_jogo() {
    let w = mundo();
    let mut g = jogo("man:2", "Portátil", GameSource::Custom);
    g.custom = Some(CustomGame {
        id: "man:2".into(),
        name: "Portátil".into(),
        save_templates: vec!["<gameDir>/saves".into()],
        process_names: vec![],
    });
    let sem = game_view(&w.ctx(), &g, GameStatus::NoSave, &[], &vazio());
    assert!(sem.needs_game_dir);
    assert_eq!(sem.game_dir, None);

    let pasta = PathBuf::from(r"D:\Jogos\Portatil");
    let s = LocalGameSettings { game_dir: Some(pasta.clone()), ..Default::default() };
    let com = game_view(&w.ctx(), &g, GameStatus::NoSave, &[], &s);
    assert!(!com.needs_game_dir);
    assert_eq!(com.game_dir.as_deref(), Some(r"D:\Jogos\Portatil"));
}

#[test]
fn destinos_lembrados_aparecem() {
    let w = mundo();
    let g = jogo("man:2", "Portátil", GameSource::Custom);
    let s = LocalGameSettings {
        target_choices: BTreeMap::from([("<gameDir>/saves".to_string(), PathBuf::from(r"E:\Saves"))]),
        ..Default::default()
    };
    let v = game_view(&w.ctx(), &g, GameStatus::NoSave, &[], &s);
    assert_eq!(v.remembered_targets.len(), 1);
    assert_eq!((v.remembered_targets[0].template.as_str(), v.remembered_targets[0].path.as_str()), ("<gameDir>/saves", r"E:\Saves"));
}

#[test]
fn capa_da_steam_quando_baixada() {
    let w = mundo();
    let mut g = jogo("pc:Jogo S", "Jogo S", GameSource::Steam);
    g.steam_app_id = Some(70);
    assert_eq!(game_view(&w.ctx(), &g, GameStatus::NoSave, &[], &vazio()).cover.kind, CoverKind::Gerada);
    write(&w.covers.steam_path(70), b"jpg");
    let v = game_view(&w.ctx(), &g, GameStatus::NoSave, &[], &vazio());
    assert_eq!(v.cover.kind, CoverKind::Steam);
    assert!(v.cover.path.as_deref().unwrap().ends_with("steam-70.jpg"));
}

#[test]
fn backup_de_outro_pc_e_marcado() {
    let w = mundo();
    let outro = snapshot_view(&w.ctx(), &entrada("pc:X", "outro-pc", 20));
    assert!(!outro.from_this_pc);
    assert_eq!(outro.pc_name, "NOTEBOOK-1");
    assert_eq!(outro.kind, SnapshotKindView::Manual);
    assert_eq!(outro.created_at, "2026-11-20T10:00:00+00:00");
    assert!(snapshot_view(&w.ctx(), &entrada("pc:X", "pc-aqui", 20)).from_this_pc);
}

#[test]
fn erros_viram_tipos() {
    let aberto = ErrorView::from(Error::GameRunning("Jogo A".into()));
    assert_eq!(aberto.kind, ErrorKind::JogoAberto);
    assert_eq!(aberto.message, "Feche Jogo A antes de restaurar");

    let destino = ErrorView::from(Error::NeedsTarget { root: "<gameDir>/saves".into(), candidates: vec![PathBuf::from("D:/a")] });
    assert_eq!(destino.kind, ErrorKind::PrecisaDestino);
    assert_eq!(destino.root.as_deref(), Some("<gameDir>/saves"));
    assert_eq!(destino.candidates, [r"D:\a"]);

    assert_eq!(ErrorView::from(Error::VaultUnavailable(PathBuf::from("E:/cofre"))).kind, ErrorKind::CofreIndisponivel);
    assert_eq!(ErrorView::from(Error::NotFound("x".into())).kind, ErrorKind::NaoEncontrado);
    assert_eq!(ErrorView::from(Error::Parse("x".into())).kind, ErrorKind::Erro);
}
