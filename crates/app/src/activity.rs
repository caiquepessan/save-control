//! Atividade (`atividade.jsonl`): backups, restaurações, erros e avisos, uma
//! linha JSON por evento. A tela Atividade lê daqui.

use std::io::Write;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use savecontrol_core::fsutil::write_atomic;

/// Passando disso, o arquivo é regravado só com a metade mais nova.
const MAX_BYTES: u64 = 2 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ActivityKind {
    BackupAuto,
    BackupManual,
    Restore,
    Error,
    Notice,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "ActivityView")]
pub struct ActivityEntry {
    #[ts(type = "string")]
    pub at: DateTime<Utc>,
    pub kind: ActivityKind,
    pub game_id: Option<String>,
    pub title: Option<String>,
    pub detail: String,
}

pub struct ActivityLog {
    path: PathBuf,
    max_bytes: u64,
}

impl ActivityLog {
    pub fn new(path: PathBuf) -> ActivityLog {
        ActivityLog::with_limit(path, MAX_BYTES)
    }

    pub fn with_limit(path: PathBuf, max_bytes: u64) -> ActivityLog {
        ActivityLog { path, max_bytes }
    }

    pub fn append(&self, e: &ActivityEntry) -> std::io::Result<()> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let mut line = serde_json::to_vec(e).map_err(std::io::Error::other)?;
        line.push(b'\n');
        std::fs::OpenOptions::new().create(true).append(true).open(&self.path)?.write_all(&line)?;
        if std::fs::metadata(&self.path)?.len() > self.max_bytes {
            self.trim()?;
        }
        Ok(())
    }

    /// Regrava só as linhas mais novas que cabem em metade do limite.
    fn trim(&self) -> std::io::Result<()> {
        let text = std::fs::read_to_string(&self.path)?;
        let mut kept: Vec<&str> = Vec::new();
        let mut size = 0u64;
        for line in text.lines().rev().filter(|l| !l.trim().is_empty()) {
            size += line.len() as u64 + 1;
            if size > self.max_bytes / 2 {
                break;
            }
            kept.push(line);
        }
        kept.reverse();
        let mut out = kept.join("\n");
        out.push('\n');
        write_atomic(&self.path, out.as_bytes()).map_err(std::io::Error::other)
    }

    /// Os `limit` eventos mais novos, do mais novo ao mais velho. Linha que não
    /// dá para ler (ex.: o PC desligou no meio da gravação) é ignorada.
    pub fn recent(&self, limit: usize) -> Vec<ActivityEntry> {
        let Ok(text) = std::fs::read_to_string(&self.path) else { return Vec::new() };
        text.lines().rev().filter_map(|l| serde_json::from_str(l).ok()).take(limit).collect()
    }
}
