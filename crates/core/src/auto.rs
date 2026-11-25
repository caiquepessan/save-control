//! Quando fazer o backup automático (spec §9). Não tem relógio próprio: quem
//! chama passa a hora, então as regras são testadas sem esperar de verdade.
//!
//! | Situação                              | Backup                                   |
//! |---------------------------------------|------------------------------------------|
//! | Dá pra saber que o jogo está aberto   | quando ele fecha, mais `after_close`     |
//! | Jogo aberto por muito tempo           | no máximo 1 a cada `while_running`       |
//! | Não dá pra saber (ou mudou já fechado) | depois de `quiet` sem mudanças           |

use std::collections::BTreeMap;

use chrono::{DateTime, Duration, Utc};

#[derive(Clone, Debug, PartialEq)]
pub struct AutoRules {
    /// Sem saber se o jogo está aberto (ou mudança com ele fechado): tempo sem mudanças.
    pub quiet: Duration,
    /// Espera depois que o jogo fecha.
    pub after_close: Duration,
    /// Com o jogo aberto: no máximo 1 backup a cada.
    pub while_running: Duration,
    /// Depois de uma falha (arquivo travado, save que não estabiliza): nova tentativa em.
    pub retry: Duration,
}

impl Default for AutoRules {
    fn default() -> Self {
        AutoRules {
            quiet: Duration::seconds(60),
            after_close: Duration::seconds(10),
            while_running: Duration::minutes(15),
            retry: Duration::minutes(5),
        }
    }
}

/// Se o jogo está aberto agora. `Unknown` = não dá para saber (sem processo nem pasta conhecidos).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Running {
    Yes,
    No,
    #[default]
    Unknown,
}

/// Por que o backup venceu. Só `Closed` gera notificação.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DueReason {
    Closed,
    LongSession,
    Quiet,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Due {
    pub game_id: String,
    pub reason: DueReason,
}

#[derive(Clone, Debug, Default)]
struct Track {
    /// Primeira mudança ainda sem backup.
    dirty_since: Option<DateTime<Utc>>,
    last_change: Option<DateTime<Utc>>,
    running: Running,
    closed_at: Option<DateTime<Utc>>,
    last_backup: Option<DateTime<Utc>>,
    retry_at: Option<DateTime<Utc>>,
}

pub struct Scheduler {
    rules: AutoRules,
    games: BTreeMap<String, Track>,
}

impl Scheduler {
    pub fn new(rules: AutoRules) -> Scheduler {
        Scheduler { rules, games: BTreeMap::new() }
    }

    /// Tempos novos (Configurações) valem para os prazos a partir de agora.
    pub fn set_rules(&mut self, rules: AutoRules) {
        self.rules = rules;
    }

    fn track(&mut self, game_id: &str) -> &mut Track {
        self.games.entry(game_id.to_string()).or_default()
    }

    pub fn note_change(&mut self, game_id: &str, at: DateTime<Utc>) {
        let t = self.track(game_id);
        t.dirty_since.get_or_insert(at);
        t.last_change = Some(t.last_change.map_or(at, |l| l.max(at)));
    }

    pub fn note_running(&mut self, game_id: &str, running: Running, at: DateTime<Utc>) {
        let t = self.track(game_id);
        if t.running == Running::Yes && running == Running::No {
            t.closed_at = Some(at);
        }
        t.running = running;
    }

    /// Quando o backup pendente deste jogo vence, e por quê.
    fn due_at(&self, t: &Track) -> Option<(DateTime<Utc>, DueReason)> {
        let dirty = t.dirty_since?;
        let (at, reason) = match t.running {
            // Nunca dois backups em menos de `while_running`, nem logo depois da primeira mudança.
            Running::Yes => (t.last_backup.map_or(dirty, |b| b.max(dirty)) + self.rules.while_running, DueReason::LongSession),
            // A mudança aconteceu com o jogo aberto: o backup sai quando ele fecha.
            Running::No if t.closed_at.is_some_and(|c| c >= dirty) => {
                (t.closed_at.unwrap_or(dirty) + self.rules.after_close, DueReason::Closed)
            }
            _ => (t.last_change.unwrap_or(dirty) + self.rules.quiet, DueReason::Quiet),
        };
        Some((t.retry_at.map_or(at, |r| r.max(at)), reason))
    }

    pub fn due(&self, now: DateTime<Utc>) -> Vec<Due> {
        self.games
            .iter()
            .filter_map(|(id, t)| self.due_at(t).filter(|(at, _)| *at <= now).map(|(_, reason)| Due { game_id: id.clone(), reason }))
            .collect()
    }

    /// O backup saiu (ou o save era igual a um que já existe): nada pendente.
    pub fn backed_up(&mut self, game_id: &str, at: DateTime<Utc>) {
        let t = self.track(game_id);
        t.dirty_since = None;
        t.last_change = None;
        t.closed_at = None;
        t.retry_at = None;
        t.last_backup = Some(at);
    }

    /// Não deu certo agora: continua pendente e tenta de novo depois de `retry`.
    pub fn failed(&mut self, game_id: &str, at: DateTime<Utc>) {
        let retry = self.rules.retry;
        self.track(game_id).retry_at = Some(at + retry);
    }

    /// O próximo prazo (pode já ter passado). `None` sem nada pendente.
    pub fn next_due(&self, _now: DateTime<Utc>) -> Option<DateTime<Utc>> {
        self.games.values().filter_map(|t| self.due_at(t)).map(|(at, _)| at).min()
    }

    /// Jogos com mudança ainda sem backup.
    pub fn pending(&self) -> Vec<String> {
        self.games.iter().filter(|(_, t)| t.dirty_since.is_some()).map(|(id, _)| id.clone()).collect()
    }

    pub fn forget(&mut self, game_id: &str) {
        self.games.remove(game_id);
    }
}
