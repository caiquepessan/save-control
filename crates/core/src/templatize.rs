//! Transforma uma pasta real num template portátil (com placeholders).

use std::path::Path;

use crate::env::SystemPaths;
use crate::placeholder::slash;

/// Escapa os caracteres especiais de glob de um trecho literal.
fn escape_literal(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '[' => out.push_str("[[]"),
            ']' => out.push_str("[]]"),
            '*' => out.push_str("[*]"),
            '?' => out.push_str("[?]"),
            c => out.push(c),
        }
    }
    out
}

fn is_prefix(base: &str, path: &str) -> bool {
    let (b, p) = (base.to_lowercase(), path.to_lowercase());
    p == b || p.starts_with(&format!("{b}/"))
}

/// O prefixo conhecido mais longo vira placeholder; o resto fica literal (escapado).
pub fn templatize(path: &Path, sys: &SystemPaths, game_dir: Option<&Path>) -> String {
    let p = slash(path);
    let mut candidates: Vec<(String, &str)> = Vec::new();
    if let Some(gd) = game_dir {
        candidates.push((slash(gd), "<gameDir>"));
    }
    candidates.extend([
        (slash(&sys.local_app_data_low), "<winLocalAppDataLow>"),
        (slash(&sys.local_app_data), "<winLocalAppData>"),
        (slash(&sys.app_data), "<winAppData>"),
        (slash(&sys.documents), "<winDocuments>"),
        (slash(&sys.public), "<winPublic>"),
        (slash(&sys.program_data), "<winProgramData>"),
        (slash(&sys.home), "<home>"),
    ]);
    let best = candidates
        .iter()
        .filter(|(base, _)| !base.is_empty() && is_prefix(base, &p))
        .max_by_key(|(base, _)| base.len());
    match best {
        Some((base, ph)) => {
            let rest = p[base.len()..].trim_start_matches('/');
            if rest.is_empty() { ph.to_string() } else { format!("{ph}/{}", escape_literal(rest)) }
        }
        None => escape_literal(&p),
    }
}
