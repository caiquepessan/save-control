//! O que o ícone da bandeja e a notificação do Windows mostram (spec §9, §13),
//! a partir da situação do backup automático. O src-tauri só desenha.

use chrono::{DateTime, FixedOffset};

use crate::dto::{AutoBackupView, AutoState, AutoStatusView};
use crate::texts;

pub const ABRIR: &str = "Abrir Save Control";
pub const BACKUP_DE_TUDO: &str = "Backup de tudo agora";
pub const PAUSAR: &str = "Pausar backup automático";
pub const PAUSAR_30_MIN: &str = "Por 30 minutos";
pub const PAUSAR_2_H: &str = "Por 2 horas";
pub const PAUSAR_ATE_REABRIR: &str = "Até reabrir o app";
pub const RETOMAR: &str = "Retomar backup automático";
pub const ABRIR_COFRE: &str = "Abrir pasta do cofre";
pub const SAIR: &str = "Sair";
/// Dica do ícone enquanto o "Sair" espera o backup em andamento.
pub const TERMINANDO: &str = "Save Control · Terminando o backup…";
pub const NOTIFICACAO_TITULO: &str = "Backup automático feito";
pub const VER_HISTORICO: &str = "Ver histórico";
pub const DISPENSAR: &str = "Dispensar";

/// A dica do ícone da bandeja do Windows tem 128 posições (UTF-16), com o fim.
const DICA_MAX: usize = 127;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IconeBandeja {
    Normal,
    /// Fazendo backup agora.
    Backup,
    Pausado,
    CofreFora,
}

/// O item de pausa do menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pausa {
    /// "Pausar backup automático ›" com as três opções.
    Pausar,
    /// "Retomar backup automático".
    Retomar,
    /// "Pausar…" desabilitado (automático desligado ou parado).
    Indisponivel,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Bandeja {
    /// Primeira linha do menu (desabilitada).
    pub titulo: String,
    /// Dica do ícone.
    pub dica: String,
    pub icone: IconeBandeja,
    pub pausa: Pausa,
    /// "Backup de tudo agora" e "Abrir pasta do cofre" habilitados.
    pub backup_ok: bool,
}

pub fn bandeja(s: &AutoStatusView, fuso: FixedOffset) -> Bandeja {
    let titulo = match s.state {
        AutoState::Parado => "Save Control".to_string(),
        AutoState::Desligado => "Save Control · backup automático desligado".to_string(),
        _ => format!("Save Control · Vigiando {} {}", s.watching, if s.watching == 1 { "jogo" } else { "jogos" }),
    };
    let detalhe = if s.busy {
        Some("fazendo backup…".to_string())
    } else {
        match s.state {
            AutoState::Pausado => Some(match s.paused_until.as_deref().map(DateTime::parse_from_rfc3339) {
                Some(Ok(t)) => format!("pausado até {}", t.with_timezone(&fuso).format("%H:%M")),
                _ => "pausado até reabrir o app".to_string(),
            }),
            AutoState::CofreFora => Some("cofre indisponível".to_string()),
            AutoState::Ativo => s.last_backup.as_ref().map(|b| format!("último backup: {}", b.title)),
            AutoState::Desligado | AutoState::Parado => None,
        }
    };
    let dica = match detalhe {
        Some(d) => cortar(&format!("{titulo} · {d}"), DICA_MAX),
        None => cortar(&titulo, DICA_MAX),
    };
    let icone = if s.busy {
        IconeBandeja::Backup
    } else {
        match s.state {
            AutoState::Pausado | AutoState::Desligado => IconeBandeja::Pausado,
            AutoState::CofreFora => IconeBandeja::CofreFora,
            AutoState::Ativo | AutoState::Parado => IconeBandeja::Normal,
        }
    };
    let pausa = match s.state {
        AutoState::Pausado => Pausa::Retomar,
        AutoState::Ativo | AutoState::CofreFora => Pausa::Pausar,
        AutoState::Desligado | AutoState::Parado => Pausa::Indisponivel,
    };
    let backup_ok = !matches!(s.state, AutoState::Parado | AutoState::CofreFora);
    Bandeja { titulo, dica, icone, pausa, backup_ok }
}

/// Corta em `max` unidades UTF-16, terminando com "…".
fn cortar(texto: &str, max: usize) -> String {
    if texto.encode_utf16().count() <= max {
        return texto.to_string();
    }
    let mut out = String::new();
    let mut usadas = 1; // o "…"
    for c in texto.chars() {
        usadas += c.len_utf16();
        if usadas > max {
            break;
        }
        out.push(c);
    }
    format!("{}…", out.trim_end())
}

/// Uma notificação por lote de backups automáticos.
#[derive(Clone, Debug, PartialEq)]
pub struct Notificacao {
    pub titulo: String,
    pub texto: String,
    /// Jogo do "Ver histórico" e do clique no aviso: o primeiro do lote.
    pub jogo: String,
    /// Capa do jogo, só quando o lote tem um jogo.
    pub capa: Option<String>,
}

pub fn notificacao(itens: &[AutoBackupView]) -> Option<Notificacao> {
    let primeiro = itens.first()?;
    let (texto, capa) = if itens.len() == 1 {
        (
            format!(
                "{} · {}. Você fechou o jogo e o save foi pro cofre.",
                primeiro.title,
                texts::arquivos(primeiro.file_count, primeiro.total_size)
            ),
            primeiro.cover.clone(),
        )
    } else {
        let titulos: Vec<&str> = itens.iter().map(|i| i.title.as_str()).collect();
        (format!("{} jogos: {}.", itens.len(), titulos.join(", ")), None)
    };
    Some(Notificacao { titulo: NOTIFICACAO_TITULO.to_string(), texto, jogo: primeiro.id.clone(), capa })
}
