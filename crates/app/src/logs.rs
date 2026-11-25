//! `logs\save-control.log`: o que acontece em segundo plano, para entender um
//! problema depois. Gira em 1 MB e guarda 3 arquivos antigos. Nunca falha: um
//! log que não dá para gravar não pode derrubar o app.

use std::io::Write;
use std::path::PathBuf;

use chrono::Utc;

const MAX_BYTES: u64 = 1024 * 1024;
const OLD_FILES: u32 = 3;

pub struct Logger {
    dir: PathBuf,
    max_bytes: u64,
}

impl Logger {
    pub fn new(dir: PathBuf) -> Logger {
        Logger::with_limit(dir, MAX_BYTES)
    }

    pub fn with_limit(dir: PathBuf, max_bytes: u64) -> Logger {
        Logger { dir, max_bytes }
    }

    pub fn info(&self, msg: &str) {
        self.write("INFO", msg);
    }

    pub fn error(&self, msg: &str) {
        self.write("ERRO", msg);
    }

    fn file(&self, n: u32) -> PathBuf {
        if n == 0 { self.dir.join("save-control.log") } else { self.dir.join(format!("save-control.{n}.log")) }
    }

    /// `save-control.log` → `.1` → `.2` → `.3` (o mais velho sai).
    fn rotate(&self) {
        let _ = std::fs::remove_file(self.file(OLD_FILES));
        for n in (0..OLD_FILES).rev() {
            let _ = std::fs::rename(self.file(n), self.file(n + 1));
        }
    }

    fn write(&self, level: &str, msg: &str) {
        let line = format!("{} {level} {msg}\n", Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
        if std::fs::create_dir_all(&self.dir).is_err() {
            return;
        }
        let size = std::fs::metadata(self.file(0)).map(|m| m.len()).unwrap_or(0);
        if size > 0 && size + line.len() as u64 > self.max_bytes {
            self.rotate();
        }
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(self.file(0)) {
            let _ = f.write_all(line.as_bytes());
        }
    }
}
