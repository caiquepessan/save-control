//! Capas: da Steam (baixadas uma vez), escolhidas pelo usuário e a ordem entre elas.
mod common;

use std::cell::Cell;

use chrono::{DateTime, Duration, TimeZone, Utc};
use common::write;
use savecontrol_app::covers::{CoverChoice, CoverFetcher, CoverStore};
use savecontrol_core::local_state::LocalGameSettings;

struct Falso {
    resposta: Result<Option<Vec<u8>>, String>,
    chamadas: Cell<usize>,
}

impl Falso {
    fn com(resposta: Result<Option<Vec<u8>>, String>) -> Falso {
        Falso { resposta, chamadas: Cell::new(0) }
    }
}

impl CoverFetcher for Falso {
    fn get(&self, url: &str) -> Result<Option<Vec<u8>>, String> {
        assert!(url.starts_with("https://cdn.cloudflare.steamstatic.com/steam/apps/"), "{url}");
        assert!(url.ends_with("/library_600x900.jpg"), "{url}");
        self.chamadas.set(self.chamadas.get() + 1);
        self.resposta.clone()
    }
}

fn jpeg() -> Vec<u8> {
    let mut v = vec![0xFF, 0xD8, 0xFF, 0xE0];
    v.resize(2048, 7);
    v
}

fn agora() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 24, 12, 0, 0).unwrap()
}

#[test]
fn capa_steam_baixa_uma_vez() {
    let t = tempfile::tempdir().unwrap();
    let store = CoverStore { dir: t.path().join("capas") };
    let f = Falso::com(Ok(Some(jpeg())));
    let p = store.ensure_steam(10, &f, agora()).unwrap();
    assert_eq!(p, store.steam_path(10));
    assert_eq!(p.file_name().unwrap(), "steam-10.jpg");
    assert_eq!(std::fs::read(&p).unwrap(), jpeg());
    assert_eq!(store.ensure_steam(10, &f, agora()), Some(p));
    assert_eq!(f.chamadas.get(), 1);
}

#[test]
fn falha_nao_repete_por_7_dias() {
    let t = tempfile::tempdir().unwrap();
    let store = CoverStore { dir: t.path().join("capas") };
    let nao_tem = Falso::com(Ok(None));
    assert_eq!(store.ensure_steam(20, &nao_tem, agora()), None);
    assert_eq!(store.ensure_steam(20, &nao_tem, agora() + Duration::days(6)), None);
    assert_eq!(nao_tem.chamadas.get(), 1);
    assert_eq!(store.ensure_steam(20, &nao_tem, agora() + Duration::days(8)), None);
    assert_eq!(nao_tem.chamadas.get(), 2);

    let offline = Falso::com(Err("sem internet".into()));
    assert_eq!(store.ensure_steam(21, &offline, agora()), None);
    assert_eq!(store.ensure_steam(21, &offline, agora() + Duration::hours(1)), None);
    assert_eq!(offline.chamadas.get(), 1);
}

#[test]
fn resposta_que_nao_e_jpeg_conta_como_falha() {
    let t = tempfile::tempdir().unwrap();
    let store = CoverStore { dir: t.path().join("capas") };
    let html = Falso::com(Ok(Some(b"<html><body>erro</body></html>".to_vec())));
    assert_eq!(store.ensure_steam(30, &html, agora()), None);
    assert!(!store.steam_path(30).exists());
    let falhas = std::fs::read_to_string(store.dir.join("falhas.json")).unwrap();
    assert!(falhas.contains("\"30\""), "{falhas}");
}

#[test]
fn capa_escolhida_e_copiada() {
    let t = tempfile::tempdir().unwrap();
    let store = CoverStore { dir: t.path().join("capas") };
    let src = t.path().join("Minha Capa.PNG");
    write(&src, b"png-bytes");
    let p = store.import_custom(&src).unwrap();
    let nome = p.file_name().unwrap().to_string_lossy().into_owned();
    assert!(nome.starts_with("custom-") && nome.ends_with(".png"), "{nome}");
    assert_eq!(nome.len(), "custom-".len() + 8 + ".png".len());
    assert_eq!(p.parent().unwrap(), store.dir);
    assert_eq!(std::fs::read(&p).unwrap(), b"png-bytes");

    let txt = t.path().join("nota.txt");
    write(&txt, b"x");
    assert!(store.import_custom(&txt).is_err());
}

#[test]
fn ordem_de_preferencia_das_capas() {
    let t = tempfile::tempdir().unwrap();
    let store = CoverStore { dir: t.path().join("capas") };
    let padrao = LocalGameSettings::default();
    assert_eq!(store.choice(Some(40), &padrao), CoverChoice::Generated);

    write(&store.steam_path(40), &jpeg());
    assert_eq!(store.choice(Some(40), &padrao), CoverChoice::Steam(store.steam_path(40)));
    assert_eq!(store.choice(None, &padrao), CoverChoice::Generated);

    let src = t.path().join("capa.jpg");
    write(&src, b"jpg-bytes");
    let propria = store.import_custom(&src).unwrap();
    let ajustes = LocalGameSettings { cover: Some(propria.clone()), ..Default::default() };
    assert_eq!(store.choice(Some(40), &ajustes), CoverChoice::Custom(propria.clone()));

    std::fs::remove_file(&propria).unwrap();
    assert_eq!(store.choice(Some(40), &ajustes), CoverChoice::Steam(store.steam_path(40)));
}
