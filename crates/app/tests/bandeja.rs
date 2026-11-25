//! O que o ícone da bandeja e a notificação do Windows mostram, a partir da
//! situação do backup automático (o src-tauri só desenha).

use chrono::FixedOffset;
use savecontrol_app::bandeja::{IconeBandeja, Pausa, bandeja, notificacao};
use savecontrol_app::dto::{AutoBackupView, AutoState, AutoStatusView, BackupOutcomeView, LastBackupView};

fn situacao(state: AutoState) -> AutoStatusView {
    AutoStatusView { state, paused_until: None, watching: 3, busy: false, last_backup: None }
}

fn ultimo(titulo: &str) -> Option<LastBackupView> {
    Some(LastBackupView { title: titulo.into(), at: "2026-11-25T20:00:00+00:00".into() })
}

/// Horário de Brasília (UTC-3).
fn brasilia() -> FixedOffset {
    FixedOffset::west_opt(3 * 3600).unwrap()
}

#[test]
fn ativo_mostra_quantos_jogos_vigia() {
    let b = bandeja(&situacao(AutoState::Ativo), brasilia());
    assert_eq!(b.titulo, "Save Control · Vigiando 3 jogos");
    assert_eq!(b.dica, "Save Control · Vigiando 3 jogos");
    assert_eq!(b.icone, IconeBandeja::Normal);
    assert_eq!(b.pausa, Pausa::Pausar);
    assert!(b.backup_ok);
}

#[test]
fn um_jogo_fica_no_singular() {
    let s = AutoStatusView { watching: 1, ..situacao(AutoState::Ativo) };
    assert_eq!(bandeja(&s, brasilia()).titulo, "Save Control · Vigiando 1 jogo");
}

#[test]
fn pausado_mostra_a_hora_local_e_retomar() {
    let s = AutoStatusView { paused_until: Some("2026-11-25T20:30:00+00:00".into()), ..situacao(AutoState::Pausado) };
    let b = bandeja(&s, brasilia());
    assert_eq!(b.dica, "Save Control · Vigiando 3 jogos · pausado até 17:30");
    assert_eq!(b.icone, IconeBandeja::Pausado);
    assert_eq!(b.pausa, Pausa::Retomar);
}

#[test]
fn pausado_ate_reabrir_o_app() {
    let b = bandeja(&situacao(AutoState::Pausado), brasilia());
    assert_eq!(b.dica, "Save Control · Vigiando 3 jogos · pausado até reabrir o app");
}

#[test]
fn cofre_fora_avisa_e_nao_deixa_fazer_backup() {
    let b = bandeja(&situacao(AutoState::CofreFora), brasilia());
    assert_eq!(b.dica, "Save Control · Vigiando 3 jogos · cofre indisponível");
    assert_eq!(b.icone, IconeBandeja::CofreFora);
    assert!(!b.backup_ok);
    assert_eq!(b.pausa, Pausa::Pausar);
}

#[test]
fn fazendo_backup_troca_o_icone() {
    let s = AutoStatusView { busy: true, last_backup: ultimo("Rota Norte"), ..situacao(AutoState::Ativo) };
    let b = bandeja(&s, brasilia());
    assert_eq!(b.icone, IconeBandeja::Backup);
    assert_eq!(b.dica, "Save Control · Vigiando 3 jogos · fazendo backup…");
}

#[test]
fn mostra_o_ultimo_backup() {
    let s = AutoStatusView { last_backup: ultimo("Vale das Lanternas"), ..situacao(AutoState::Ativo) };
    assert_eq!(bandeja(&s, brasilia()).dica, "Save Control · Vigiando 3 jogos · último backup: Vale das Lanternas");
}

#[test]
fn desligado_nao_tem_pausa() {
    let b = bandeja(&situacao(AutoState::Desligado), brasilia());
    assert_eq!(b.titulo, "Save Control · backup automático desligado");
    assert_eq!(b.dica, "Save Control · backup automático desligado");
    assert_eq!(b.icone, IconeBandeja::Pausado);
    assert_eq!(b.pausa, Pausa::Indisponivel);
    assert!(b.backup_ok, "o backup manual continua valendo");
}

#[test]
fn parado_mostra_so_o_nome() {
    let b = bandeja(&situacao(AutoState::Parado), brasilia());
    assert_eq!(b.titulo, "Save Control");
    assert_eq!(b.dica, "Save Control");
    assert_eq!(b.icone, IconeBandeja::Normal);
    assert_eq!(b.pausa, Pausa::Indisponivel);
    assert!(!b.backup_ok);
}

/// A dica do ícone da bandeja do Windows cabe em 127 caracteres (UTF-16).
#[test]
fn dica_longa_e_cortada_para_caber() {
    let titulo = "Um Jogo de Nome Muito Comprido ".repeat(6);
    let s = AutoStatusView { last_backup: ultimo(&titulo), ..situacao(AutoState::Ativo) };
    let dica = bandeja(&s, brasilia()).dica;
    assert!(dica.encode_utf16().count() <= 127, "{} unidades", dica.encode_utf16().count());
    assert!(dica.starts_with("Save Control · Vigiando 3 jogos · último backup: Um Jogo"), "{dica}");
    assert!(dica.ends_with('…'), "{dica}");
}

fn feito(id: &str, titulo: &str, n: usize, bytes: u64, capa: Option<&str>) -> AutoBackupView {
    AutoBackupView {
        id: id.into(),
        title: titulo.into(),
        outcome: BackupOutcomeView::Feito,
        file_count: n,
        total_size: bytes,
        cover: capa.map(String::from),
    }
}

#[test]
fn notificacao_de_um_jogo() {
    let n = notificacao(&[feito("pc:vale", "Vale das Lanternas", 3, 2048, Some(r"C:\capas\vale.png"))]).unwrap();
    assert_eq!(n.titulo, "Backup automático feito");
    assert_eq!(n.texto, "Vale das Lanternas · 3 arquivos · 2 KB. Você fechou o jogo e o save foi pro cofre.");
    assert_eq!(n.jogo, "pc:vale");
    assert_eq!(n.capa.as_deref(), Some(r"C:\capas\vale.png"));
}

#[test]
fn varios_jogos_viram_uma_notificacao_so() {
    let n = notificacao(&[feito("a", "Vale das Lanternas", 1, 10, Some("a.png")), feito("b", "Rota Norte", 2, 20, None)])
        .unwrap();
    assert_eq!(n.texto, "2 jogos: Vale das Lanternas, Rota Norte.");
    assert_eq!(n.jogo, "a", "\"Ver histórico\" abre o primeiro do lote");
    assert_eq!(n.capa, None);
}

#[test]
fn sem_backup_sem_notificacao() {
    assert!(notificacao(&[]).is_none());
}
