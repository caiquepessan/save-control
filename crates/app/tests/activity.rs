//! Atividade (atividade.jsonl) e logs (logs\save-control.log).

use std::io::Write;

use chrono::{DateTime, Duration, TimeZone, Utc};
use savecontrol_app::activity::{ActivityEntry, ActivityKind, ActivityLog};
use savecontrol_app::logs::Logger;

fn t0() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 25, 20, 0, 0).unwrap()
}

fn evento(n: i64) -> ActivityEntry {
    ActivityEntry {
        at: t0() + Duration::minutes(n),
        kind: ActivityKind::BackupAuto,
        game_id: Some("man:a".into()),
        title: Some("Jogo A".into()),
        detail: format!("evento {n}"),
    }
}

fn detalhes(v: &[ActivityEntry]) -> Vec<String> {
    v.iter().map(|e| e.detail.clone()).collect()
}

#[test]
fn atividade_mais_nova_primeiro() {
    let t = tempfile::tempdir().unwrap();
    let log = ActivityLog::new(t.path().join("atividade.jsonl"));
    for n in 1..=3 {
        log.append(&evento(n)).unwrap();
    }
    assert_eq!(detalhes(&log.recent(10)), ["evento 3", "evento 2", "evento 1"]);
    assert_eq!(detalhes(&log.recent(2)), ["evento 3", "evento 2"]);
}

#[test]
fn atividade_vazia_sem_arquivo() {
    let t = tempfile::tempdir().unwrap();
    assert!(ActivityLog::new(t.path().join("atividade.jsonl")).recent(10).is_empty());
}

#[test]
fn linha_quebrada_e_ignorada() {
    let t = tempfile::tempdir().unwrap();
    let p = t.path().join("atividade.jsonl");
    let log = ActivityLog::new(p.clone());
    log.append(&evento(1)).unwrap();
    // Uma linha pela metade (ex.: o PC desligou no meio da gravação).
    std::fs::OpenOptions::new().append(true).open(&p).unwrap().write_all(b"{\"at\": \"2026-11-2").unwrap();
    std::fs::OpenOptions::new().append(true).open(&p).unwrap().write_all(b"\n").unwrap();
    log.append(&evento(2)).unwrap();
    assert_eq!(detalhes(&log.recent(10)), ["evento 2", "evento 1"]);
}

#[test]
fn atividade_grande_guarda_a_metade_mais_nova() {
    let t = tempfile::tempdir().unwrap();
    let p = t.path().join("atividade.jsonl");
    let log = ActivityLog::with_limit(p.clone(), 4_000);
    for n in 1..=100 {
        log.append(&evento(n)).unwrap();
    }
    assert!(std::fs::metadata(&p).unwrap().len() <= 4_000, "o arquivo passou do limite");
    let todos = log.recent(1_000);
    assert_eq!(todos[0].detail, "evento 100", "perdeu o mais novo");
    assert!(todos.len() < 100 && todos.len() >= 10, "{} eventos sobraram", todos.len());
}

#[test]
fn log_gira_e_guarda_3() {
    let t = tempfile::tempdir().unwrap();
    let logs = t.path().join("logs");
    let l = Logger::with_limit(logs.clone(), 300);
    for n in 0..200 {
        l.error(&format!("falha número {n} ao copiar um save"));
    }
    let mut nomes: Vec<String> =
        std::fs::read_dir(&logs).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    nomes.sort();
    assert_eq!(nomes, ["save-control.1.log", "save-control.2.log", "save-control.3.log", "save-control.log"]);
    let atual = std::fs::read_to_string(logs.join("save-control.log")).unwrap();
    assert!(atual.contains("ERRO falha número 199"), "a última linha não está no log atual: {atual}");
}
