mod common;

use chrono::Duration;
use common::local;
use savecontrol_core::{
    fingerprint::Fingerprint,
    vault::{format::SnapshotKind, index::SnapshotEntry, retention::*},
};

fn e(n: usize, kind: SnapshotKind, at: chrono::DateTime<chrono::Local>, pc: &str) -> SnapshotEntry {
    SnapshotEntry {
        game_id: "g".into(),
        dir_name: "G".into(),
        file_name: format!("{n}.zip"),
        kind,
        label: None,
        created_at: at.with_timezone(&chrono::Utc),
        machine_id: pc.into(),
        machine_label: pc.into(),
        pc_name: pc.into(),
        fingerprint: Fingerprint::default(),
        total_size: 1,
        file_count: 1,
    }
}

fn podados(v: &[SnapshotEntry], now: chrono::DateTime<chrono::Local>) -> Vec<String> {
    let refs: Vec<&SnapshotEntry> = v.iter().collect();
    let mut p: Vec<String> =
        plan_prune(&refs, "A", now, &RetentionPolicy::default()).into_iter().map(|x| x.file_name.clone()).collect();
    p.sort();
    p
}

#[test]
fn quinze_no_mesmo_dia_ficam_dez() {
    let now = local(2026, 11, 25, 23, 0, 0);
    let v: Vec<_> = (0..15).map(|h| e(h, SnapshotKind::Auto, local(2026, 11, 25, 6 + h as u32, 0, 0), "A")).collect();
    assert_eq!(podados(&v, now), ["0.zip", "1.zip", "2.zip", "3.zip", "4.zip"]);
}

#[test]
fn um_por_dia_por_56_dias() {
    let now = local(2026, 11, 25, 13, 0, 0);
    let inicio = local(2026, 10, 1, 12, 0, 0);
    let v: Vec<_> = (0..56).map(|d| e(d, SnapshotKind::Auto, inicio + Duration::days(d as i64), "A")).collect();
    let refs: Vec<&SnapshotEntry> = v.iter().collect();
    let p = plan_prune(&refs, "A", now, &RetentionPolicy::default());
    let mut ficam: Vec<u32> = v
        .iter()
        .filter(|x| !p.iter().any(|y| y.file_name == x.file_name))
        .map(|x| x.created_at.with_timezone(&chrono::Local).format("%m%d").to_string().parse().unwrap())
        .collect();
    ficam.sort();
    assert_eq!(ficam, [1108, 1115, 1116, 1117, 1118, 1119, 1120, 1121, 1122, 1123, 1124, 1125]);
}

#[test]
fn outro_pc_manual_e_fixado_nunca_saem() {
    let now = local(2026, 11, 25, 23, 0, 0);
    let mut v: Vec<_> = (0..20).map(|h| e(h, SnapshotKind::Auto, local(2026, 11, 25, 1 + h as u32, 0, 0), "B")).collect();
    v.extend((20..40).map(|n| e(n, SnapshotKind::Manual, local(2026, 1, 1, 12, 0, 0), "A")));
    v.push(e(40, SnapshotKind::Pinned, local(2025, 1, 1, 12, 0, 0), "A"));
    assert!(podados(&v, now).is_empty());
}

#[test]
fn antes_de_restaurar_guarda_cinco() {
    let now = local(2026, 11, 25, 23, 0, 0);
    let v: Vec<_> =
        (0..8).map(|h| e(h, SnapshotKind::PreRestore, local(2026, 11, 25, 1 + h as u32, 0, 0), "A")).collect();
    assert_eq!(podados(&v, now), ["0.zip", "1.zip", "2.zip"]);
}
