//! Formato do cofre: tipos de backup, metadados e nomes de arquivo.

use std::collections::BTreeMap;
use std::path::PathBuf;

use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Serialize};

use crate::custom::CustomGame;
use crate::machine::MachineIdentity;
use crate::model::GameSource;

pub const FORMAT: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SnapshotKind {
    Manual,
    Auto,
    PreRestore,
    /// Automático que o usuário fixou: nunca é apagado pela limpeza.
    Pinned,
}

impl SnapshotKind {
    pub fn suffix(self) -> &'static str {
        match self {
            SnapshotKind::Manual => "manual",
            SnapshotKind::Auto => "auto",
            SnapshotKind::PreRestore => "antes-de-restaurar",
            SnapshotKind::Pinned => "fixado",
        }
    }

    /// Tipo pelo nome `AAAA-MM-DD_HH-MM-SS[-N]_<PC>_<tipo>.zip`; nomes fora do
    /// padrão (cópias de conflito, temporários) dão `None`.
    pub fn from_file_name(name: &str) -> Option<SnapshotKind> {
        let stem = name.strip_suffix(".zip")?;
        let mut parts = stem.splitn(4, '_');
        let (date, time, label, kind) = (parts.next()?, parts.next()?, parts.next()?, parts.next()?);
        let digits_and_dashes = |s: &str, dashes: &[usize]| {
            s.char_indices().all(|(i, c)| if dashes.contains(&i) { c == '-' } else { c.is_ascii_digit() })
        };
        if date.len() != 10 || !digits_and_dashes(date, &[4, 7]) {
            return None;
        }
        let (hms, counter) = match time.get(8..) {
            Some("") => (time, None),
            Some(rest) => (&time[..8], rest.strip_prefix('-')),
            None => return None,
        };
        if !digits_and_dashes(hms, &[2, 5]) || counter.is_some_and(|c| c.is_empty() || !c.chars().all(|d| d.is_ascii_digit())) {
            return None;
        }
        if label.is_empty() || !label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return None;
        }
        [SnapshotKind::Manual, SnapshotKind::Auto, SnapshotKind::PreRestore, SnapshotKind::Pinned]
            .into_iter()
            .find(|k| k.suffix() == kind)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SnapshotRoot {
    pub alias: String,
    pub template: String,
    pub bindings: BTreeMap<String, String>,
    pub original_path: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SnapshotFile {
    pub root: String,
    pub rel_path: String,
    pub size: u64,
    pub modified: DateTime<Utc>,
    pub blake3: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SnapshotMeta {
    pub format: u32,
    pub game_id: String,
    pub title: String,
    pub kind: SnapshotKind,
    pub label: Option<String>,
    pub created_at: DateTime<Utc>,
    pub app_version: String,
    pub machine: MachineIdentity,
    pub roots: Vec<SnapshotRoot>,
    pub files: Vec<SnapshotFile>,
}

/// `jogo.json`: identifica o jogo dono de uma pasta do cofre.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GameDescriptor {
    pub format: u32,
    pub game_id: String,
    pub title: String,
    pub source: GameSource,
    pub custom: Option<CustomGame>,
    pub steam_app_id: Option<u32>,
    /// Executáveis do jogo, para saber se ele está aberto mesmo quando só existe no cofre.
    #[serde(default)]
    pub process_names: Vec<String>,
}

const RESERVED: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9", "LPT1",
    "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Nome de pasta válido no Windows a partir do título do jogo.
pub fn sanitize_folder_name(title: &str) -> String {
    let mut s = String::with_capacity(title.len());
    for c in title.chars() {
        match c {
            ':' => s.push_str(" -"),
            '<' | '>' | '"' | '/' | '\\' | '|' | '?' | '*' => s.push('_'),
            c if c.is_control() => s.push('_'),
            c => s.push(c),
        }
    }
    let collapsed = s.split_whitespace().collect::<Vec<_>>().join(" ");
    let limited: String = collapsed.chars().take(80).collect();
    let mut name = limited.trim_end_matches(['.', ' ']).to_string();
    if name.is_empty() {
        return "jogo".into();
    }
    if RESERVED.iter().any(|r| r.eq_ignore_ascii_case(&name)) {
        name.push('_');
    }
    name
}

pub fn snapshot_file_name(now: DateTime<Local>, machine: &MachineIdentity, kind: SnapshotKind) -> String {
    format!("{}_{}_{}.zip", now.format("%Y-%m-%d_%H-%M-%S"), machine.label(), kind.suffix())
}
