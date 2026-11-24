//! Escrita segura de um backup: lê estável, grava `.parcial`, confere e renomeia.

use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::{DateTime, Utc};
use zip::CompressionMethod;
use zip::write::SimpleFileOptions;

use crate::error::{Error, Result, io};
use crate::vault::PARTIAL_SUFFIX;
use crate::vault::format::{SnapshotFile, SnapshotMeta};

/// Pontos de espera/observação da cópia (os testes trocam por versões que não esperam).
pub trait CopyHooks {
    fn wait_before_retry(&self) {}
    fn after_read(&self, _path: &Path) {}
}

/// Espera 2 s entre tentativas.
pub struct RealHooks;

impl CopyHooks for RealHooks {
    fn wait_before_retry(&self) {
        std::thread::sleep(Duration::from_secs(2));
    }
}

/// Não espera (testes).
pub struct NoWait;

impl CopyHooks for NoWait {}

pub struct SourceFile {
    pub abs_path: PathBuf,
    pub root_alias: String,
    pub rel_path: String,
}

const ATTEMPTS: usize = 3;
pub(crate) const META_ENTRY: &str = "save-control.json";

pub(crate) fn entry_name(root_alias: &str, rel_path: &str) -> String {
    format!("arquivos/{root_alias}/{rel_path}")
}

fn zip_err(e: impl std::fmt::Display) -> Error {
    Error::Parse(format!("zip: {e}"))
}

struct Content {
    bytes: Vec<u8>,
    modified: DateTime<Utc>,
}

enum Failure {
    Locked,
    Unstable,
}

/// Lê o arquivo inteiro, garantindo que ele não mudou durante a leitura.
fn read_stable(path: &Path, hooks: &dyn CopyHooks) -> Result<Content> {
    let mut last = Failure::Unstable;
    for attempt in 0..ATTEMPTS {
        if attempt > 0 {
            hooks.wait_before_retry();
        }
        let before = std::fs::metadata(path).map_err(io(path))?;
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) if matches!(e.raw_os_error(), Some(32 | 33)) => {
                last = Failure::Locked;
                continue;
            }
            Err(e) => return Err(io(path)(e)),
        };
        hooks.after_read(path);
        let after = std::fs::metadata(path).map_err(io(path))?;
        let same = before.len() == after.len()
            && before.modified().ok() == after.modified().ok()
            && bytes.len() as u64 == after.len();
        if !same {
            last = Failure::Unstable;
            continue;
        }
        let modified = after.modified().map(DateTime::<Utc>::from).unwrap_or_default();
        return Ok(Content { bytes, modified });
    }
    Err(match last {
        Failure::Locked => Error::FileLocked(path.to_path_buf()),
        Failure::Unstable => Error::UnstableFile(path.to_path_buf()),
    })
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut s = path.as_os_str().to_owned();
    s.push(suffix);
    PathBuf::from(s)
}

/// Nome livre: em colisão, `-2`, `-3`… logo depois dos segundos (ou antes de `.zip`).
fn unique_path(dir: &Path, file_name: &str) -> PathBuf {
    let taken = |p: &Path| p.exists() || with_suffix(p, PARTIAL_SUFFIX).exists();
    let first = dir.join(file_name);
    if !taken(&first) {
        return first;
    }
    for n in 2.. {
        let name = match file_name.match_indices('_').nth(1) {
            Some((i, _)) if i == 19 => format!("{}-{n}{}", &file_name[..i], &file_name[i..]),
            _ => {
                let stem = file_name.strip_suffix(".zip").unwrap_or(file_name);
                format!("{stem}-{n}.zip")
            }
        };
        let p = dir.join(name);
        if !taken(&p) {
            return p;
        }
    }
    unreachable!()
}

fn write_zip(partial: &Path, meta: &SnapshotMeta, sources: &[SourceFile], contents: &[Content]) -> Result<()> {
    let file = File::create(partial).map_err(io(partial))?;
    let mut zw = zip::ZipWriter::new(file);
    let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let meta_json = serde_json::to_vec_pretty(meta).map_err(|e| Error::Parse(e.to_string()))?;
    zw.start_file(META_ENTRY, opts).map_err(zip_err)?;
    zw.write_all(&meta_json).map_err(io(partial))?;
    for (s, c) in sources.iter().zip(contents) {
        let large = c.bytes.len() as u64 >= u64::from(u32::MAX);
        zw.start_file(entry_name(&s.root_alias, &s.rel_path), opts.large_file(large)).map_err(zip_err)?;
        zw.write_all(&c.bytes).map_err(io(partial))?;
    }
    let file = zw.finish().map_err(zip_err)?;
    file.sync_all().map_err(io(partial))
}

/// Relê o zip: CRC de cada entrada (conferido pela leitura) e BLAKE3 contra o meta.
fn verify_zip(partial: &Path, meta: &SnapshotMeta) -> Result<()> {
    let file = File::open(partial).map_err(io(partial))?;
    let mut ar = zip::ZipArchive::new(file).map_err(zip_err)?;
    let stored: SnapshotMeta = {
        let entry = ar.by_name(META_ENTRY).map_err(zip_err)?;
        serde_json::from_reader(entry).map_err(|e| Error::Parse(e.to_string()))?
    };
    if stored.game_id != meta.game_id || stored.files.len() != meta.files.len() {
        return Err(Error::VerifyFailed(partial.to_path_buf()));
    }
    for f in &meta.files {
        let mut entry = ar.by_name(&entry_name(&f.root, &f.rel_path)).map_err(zip_err)?;
        let mut buf = Vec::with_capacity(f.size as usize);
        entry.read_to_end(&mut buf).map_err(|_| Error::VerifyFailed(partial.to_path_buf()))?;
        if blake3::hash(&buf).to_hex().as_str() != f.blake3 {
            return Err(Error::VerifyFailed(partial.to_path_buf()));
        }
    }
    Ok(())
}

/// Grava um backup no cofre. `meta.files` chega vazio e volta preenchido.
/// Em qualquer erro, nada fica para trás (nem `.zip` nem `.parcial`).
pub fn write_snapshot(
    game_dir: &Path,
    file_name: &str,
    mut meta: SnapshotMeta,
    sources: &[SourceFile],
    hooks: &dyn CopyHooks,
) -> Result<(PathBuf, SnapshotMeta)> {
    let total: u64 = sources.iter().filter_map(|s| std::fs::metadata(&s.abs_path).ok()).map(|m| m.len()).sum();
    let needed = total + total / 10 + 1_048_576;
    if let Ok(available) = fs4::available_space(game_dir) {
        if available < needed {
            return Err(Error::InsufficientSpace { needed, available });
        }
    }

    let contents = sources.iter().map(|s| read_stable(&s.abs_path, hooks)).collect::<Result<Vec<_>>>()?;
    meta.files = sources
        .iter()
        .zip(&contents)
        .map(|(s, c)| SnapshotFile {
            root: s.root_alias.clone(),
            rel_path: s.rel_path.clone(),
            size: c.bytes.len() as u64,
            modified: c.modified,
            blake3: blake3::hash(&c.bytes).to_hex().to_string(),
        })
        .collect();

    let final_path = unique_path(game_dir, file_name);
    let partial = with_suffix(&final_path, PARTIAL_SUFFIX);
    let result = write_zip(&partial, &meta, sources, &contents)
        .and_then(|()| verify_zip(&partial, &meta))
        .and_then(|()| std::fs::rename(&partial, &final_path).map_err(io(&final_path)));
    if let Err(e) = result {
        let _ = std::fs::remove_file(&partial);
        return Err(e);
    }
    Ok((final_path, meta))
}
