//! Encontra no disco os arquivos de uma expansão de template.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};

use crate::env::SystemPaths;
use crate::placeholder::{Expansion, slash};

#[derive(Clone, Debug, PartialEq)]
pub struct FoundFile {
    pub abs_path: PathBuf,
    /// Relativo a `Found::root_dir`, com `/`.
    pub rel_path: String,
    pub size: u64,
    pub modified: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Found {
    /// Template da raiz (quando a raiz é um arquivo, é a pasta dele).
    pub root_template: String,
    pub root_dir: PathBuf,
    /// Ordenados por `rel_path`.
    pub files: Vec<FoundFile>,
}

#[derive(Debug, PartialEq)]
pub enum FindResult {
    Found(Found),
    Nothing,
    /// Só casou com pastas amplas demais (Documentos inteiro, perfil inteiro…).
    IgnoredBroad,
}

fn norm(p: &Path) -> String {
    slash(p).to_lowercase()
}

/// Raiz de disco ou uma das pastas grandes do Windows: nunca é raiz de save.
pub fn is_broad_path(path: &Path, sys: &SystemPaths) -> bool {
    let p = norm(path);
    if p.len() == 2 && p.ends_with(':') {
        return true;
    }
    [
        &sys.home,
        &sys.app_data,
        &sys.local_app_data,
        &sys.local_app_data_low,
        &sys.documents,
        &sys.saved_games,
        &sys.public,
        &sys.program_data,
        &sys.win_dir,
        &sys.program_files,
        &sys.program_files_x86,
        &sys.downloads,
        &sys.desktop,
        &sys.temp,
    ]
    .iter()
    .any(|b| norm(b) == p)
}

/// Um curinga sem nenhum trecho literal (`*`, `**`, `*.*`) logo abaixo de uma
/// pasta ampla pegaria a pasta inteira.
fn is_bare_wildcard(component: &str) -> bool {
    component.chars().all(|c| matches!(c, '*' | '?' | '.'))
}

fn file_of(abs: &Path, root_dir: &Path) -> Option<FoundFile> {
    let meta = std::fs::metadata(abs).ok()?;
    if !meta.is_file() {
        return None;
    }
    let rel = abs.strip_prefix(root_dir).ok()?;
    Some(FoundFile {
        abs_path: abs.to_path_buf(),
        rel_path: slash(rel),
        size: meta.len(),
        modified: meta.modified().map(DateTime::<Utc>::from).unwrap_or_default(),
    })
}

fn collect(path: &Path, root_dir: &Path, out: &mut Vec<FoundFile>) {
    if path.is_dir() {
        for entry in walkdir::WalkDir::new(path).into_iter().flatten() {
            if entry.file_type().is_file() {
                out.extend(file_of(entry.path(), root_dir));
            }
        }
    } else {
        out.extend(file_of(path, root_dir));
    }
}

/// Lista os arquivos que uma expansão encontra no disco.
pub fn find_files(exp: &Expansion, sys: &SystemPaths) -> FindResult {
    let mut files = Vec::new();
    let mut broad = false;
    let root_dir;
    let root_template;

    match &exp.tail_glob {
        None => {
            let p = &exp.root_path;
            if !p.exists() {
                return FindResult::Nothing;
            }
            if is_broad_path(p, sys) {
                return FindResult::IgnoredBroad;
            }
            if p.is_dir() {
                root_dir = p.clone();
                root_template = exp.root_template.clone();
            } else {
                root_dir = p.parent().map(Path::to_path_buf).unwrap_or_default();
                root_template = exp
                    .root_template
                    .rsplit_once('/')
                    .map(|(head, _)| head.to_string())
                    .unwrap_or_else(|| exp.root_template.clone());
            }
            collect(p, &root_dir, &mut files);
        }
        Some(tail) => {
            root_dir = exp.root_path.clone();
            root_template = exp.root_template.clone();
            let first = tail.split('/').next().unwrap_or_default();
            if is_broad_path(&root_dir, sys) && is_bare_wildcard(first) {
                return if root_dir.exists() { FindResult::IgnoredBroad } else { FindResult::Nothing };
            }
            let pattern = format!("{}/{}", glob::Pattern::escape(&slash(&root_dir)), tail);
            let opts = glob::MatchOptions {
                case_sensitive: false,
                require_literal_separator: true,
                require_literal_leading_dot: false,
            };
            if let Ok(paths) = glob::glob_with(&pattern, opts) {
                for m in paths.flatten() {
                    if is_broad_path(&m, sys) {
                        broad = true;
                        continue;
                    }
                    collect(&m, &root_dir, &mut files);
                }
            }
        }
    }

    if files.is_empty() {
        return if broad { FindResult::IgnoredBroad } else { FindResult::Nothing };
    }
    files.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
    files.dedup_by(|a, b| norm(&a.abs_path) == norm(&b.abs_path));
    FindResult::Found(Found { root_template, root_dir, files })
}
