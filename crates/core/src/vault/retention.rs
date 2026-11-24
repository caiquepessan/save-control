//! Limpeza automática dos backups, por jogo e por PC.

use std::collections::HashSet;

use chrono::{DateTime, Datelike, Duration, Local, NaiveDate};

use crate::vault::format::SnapshotKind;
use crate::vault::index::SnapshotEntry;

#[derive(Clone, Debug)]
pub struct RetentionPolicy {
    pub auto_recent: usize,
    pub auto_daily_days: u32,
    pub auto_weekly_weeks: u32,
    pub pre_restore_keep: usize,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        RetentionPolicy { auto_recent: 10, auto_daily_days: 7, auto_weekly_weeks: 4, pre_restore_keep: 5 }
    }
}

fn local_date(e: &SnapshotEntry) -> NaiveDate {
    e.created_at.with_timezone(&Local).date_naive()
}

/// Backups a apagar. Só considera os deste PC; `manual` e `fixado` nunca saem.
/// Automáticos: ficam os N mais novos + o mais novo de cada um dos últimos dias
/// + o mais novo de cada uma das últimas semanas (ISO).
pub fn plan_prune<'a>(
    entries: &[&'a SnapshotEntry],
    machine_id: &str,
    now: DateTime<Local>,
    policy: &RetentionPolicy,
) -> Vec<&'a SnapshotEntry> {
    let newest_first = |kind: SnapshotKind| {
        let mut v: Vec<&'a SnapshotEntry> =
            entries.iter().copied().filter(|e| e.machine_id == machine_id && e.kind == kind).collect();
        v.sort_by(|a, b| b.created_at.cmp(&a.created_at).then_with(|| b.file_name.cmp(&a.file_name)));
        v
    };

    let mut prune: Vec<&'a SnapshotEntry> = newest_first(SnapshotKind::PreRestore)
        .into_iter()
        .skip(policy.pre_restore_keep)
        .collect();

    let autos = newest_first(SnapshotKind::Auto);
    let mut keep: HashSet<usize> = (0..autos.len().min(policy.auto_recent)).collect();
    let today = now.date_naive();
    let newest_in = |from: NaiveDate, to_exclusive: NaiveDate| {
        autos.iter().position(|e| {
            let d = local_date(e);
            d >= from && d < to_exclusive
        })
    };
    for back in 0..i64::from(policy.auto_daily_days) {
        let day = today - Duration::days(back);
        keep.extend(newest_in(day, day + Duration::days(1)));
    }
    let monday = today - Duration::days(i64::from(today.weekday().num_days_from_monday()));
    for back in 0..i64::from(policy.auto_weekly_weeks) {
        let start = monday - Duration::weeks(back);
        keep.extend(newest_in(start, start + Duration::weeks(1)));
    }
    prune.extend(autos.iter().enumerate().filter(|(i, _)| !keep.contains(i)).map(|(_, e)| *e));
    prune
}
