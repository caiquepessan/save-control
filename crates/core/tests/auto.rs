//! Quando o backup automático acontece (spec §9), sem relógio de verdade.

use chrono::{DateTime, Duration, TimeZone, Utc};
use savecontrol_core::auto::*;

const J: &str = "man:jogo";

fn t0() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 25, 20, 0, 0).unwrap()
}

fn min(n: i64) -> Duration {
    Duration::minutes(n)
}

fn seg(n: i64) -> Duration {
    Duration::seconds(n)
}

fn vencidos(s: &Scheduler, at: DateTime<Utc>) -> Vec<(String, DueReason)> {
    s.due(at).into_iter().map(|d| (d.game_id, d.reason)).collect()
}

#[test]
fn jogo_aberto_so_faz_backup_ao_fechar_mais_10s() {
    let mut s = Scheduler::new(AutoRules::default());
    s.note_running(J, Running::Yes, t0());
    s.note_change(J, t0() + min(1));
    assert!(s.due(t0() + min(5)).is_empty(), "fez backup com o jogo aberto");
    s.note_running(J, Running::No, t0() + min(20));
    assert!(s.due(t0() + min(20) + seg(9)).is_empty());
    assert_eq!(vencidos(&s, t0() + min(20) + seg(10)), [(J.to_string(), DueReason::Closed)]);
}

#[test]
fn sessao_longa_faz_um_backup_a_cada_15_min() {
    let mut s = Scheduler::new(AutoRules::default());
    s.note_running(J, Running::Yes, t0());
    for m in 0..=14 {
        s.note_change(J, t0() + min(m));
    }
    assert!(s.due(t0() + min(14) + seg(59)).is_empty());
    assert_eq!(vencidos(&s, t0() + min(15)), [(J.to_string(), DueReason::LongSession)]);

    // Depois do backup, a próxima mudança só vence 15 min depois dela (nunca 2 em 15 min).
    s.backed_up(J, t0() + min(15));
    for m in 16..=30 {
        s.note_change(J, t0() + min(m));
    }
    assert!(s.due(t0() + min(30) + seg(59)).is_empty());
    assert_eq!(vencidos(&s, t0() + min(31)), [(J.to_string(), DueReason::LongSession)]);
}

#[test]
fn enxurrada_de_mudancas_nao_adianta_o_prazo() {
    let mut s = Scheduler::new(AutoRules::default());
    s.note_running(J, Running::Yes, t0());
    for i in 0..10_000 {
        s.note_change(J, t0() + Duration::milliseconds(i * 6));
    }
    assert!(s.due(t0() + min(14) + seg(59)).is_empty());
    assert_eq!(s.due(t0() + min(15)).len(), 1);
}

#[test]
fn sem_saber_se_esta_aberto_espera_60s_sem_mudanca() {
    let mut s = Scheduler::new(AutoRules::default());
    s.note_running(J, Running::Unknown, t0());
    s.note_change(J, t0());
    s.note_change(J, t0() + seg(30));
    assert!(s.due(t0() + seg(89)).is_empty());
    assert_eq!(vencidos(&s, t0() + seg(90)), [(J.to_string(), DueReason::Quiet)]);
}

#[test]
fn mudanca_com_o_jogo_fechado_espera_60s() {
    let mut s = Scheduler::new(AutoRules::default());
    s.note_running(J, Running::No, t0());
    s.note_change(J, t0() + seg(5));
    assert!(s.due(t0() + seg(64)).is_empty());
    assert_eq!(vencidos(&s, t0() + seg(65)), [(J.to_string(), DueReason::Quiet)]);
}

#[test]
fn falha_tenta_de_novo_em_5_min() {
    let mut s = Scheduler::new(AutoRules::default());
    s.note_change(J, t0());
    let t1 = t0() + seg(60);
    assert_eq!(s.due(t1).len(), 1);
    s.failed(J, t1);
    assert!(s.due(t1 + min(4) + seg(59)).is_empty());
    assert_eq!(s.due(t1 + min(5)).len(), 1);
}

#[test]
fn next_due_aponta_o_proximo_prazo() {
    let mut s = Scheduler::new(AutoRules::default());
    assert_eq!(s.next_due(t0()), None);
    s.note_change("man:a", t0());
    s.note_running("man:b", Running::Yes, t0());
    s.note_change("man:b", t0());
    assert_eq!(s.next_due(t0()), Some(t0() + seg(60)));
    s.backed_up("man:a", t0() + seg(60));
    assert_eq!(s.next_due(t0() + seg(60)), Some(t0() + min(15)));
}

#[test]
fn jogo_sem_mudanca_nunca_vence() {
    let mut s = Scheduler::new(AutoRules::default());
    s.note_running(J, Running::Yes, t0());
    s.note_running(J, Running::No, t0() + min(30));
    assert!(s.due(t0() + min(31)).is_empty());
    assert!(s.pending().is_empty());
    s.note_change(J, t0() + min(32));
    assert_eq!(s.pending(), [J.to_string()]);
}

#[test]
fn regras_ajustadas_mudam_os_prazos() {
    let regras = AutoRules { quiet: seg(20), after_close: seg(5), while_running: min(5), retry: min(1) };
    let mut s = Scheduler::new(AutoRules::default());
    s.set_rules(regras);
    s.note_change(J, t0());
    assert!(s.due(t0() + seg(19)).is_empty());
    assert_eq!(vencidos(&s, t0() + seg(20)), [(J.to_string(), DueReason::Quiet)]);
}
