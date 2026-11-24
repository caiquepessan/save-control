//! Expansão dos placeholders da base de jogos (`<winAppData>`, `<base>`…)
//! para caminhos reais e padrões de glob.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::env::SystemPaths;

/// Uma instalação possível de um jogo (Steam, Epic, GOG ou pasta do usuário).
#[derive(Clone, Debug, PartialEq)]
pub struct InstallCandidate {
    pub root: PathBuf,
    pub game: String,
    pub base: PathBuf,
    pub store_game_id: Option<String>,
}

pub struct ExpandCtx<'a> {
    pub sys: &'a SystemPaths,
    pub installs: &'a [InstallCandidate],
    pub store_user_ids: &'a [String],
    pub game_dir: Option<&'a Path>,
    pub game_name: &'a str,
}

/// Resultado de expandir um template para um caso concreto.
#[derive(Clone, Debug, PartialEq)]
pub struct Expansion {
    /// Parte do template antes do primeiro componente com glob (placeholders intactos).
    pub root_template: String,
    /// Valores escolhidos para os placeholders de vários valores (`base`, `storeUserId`…).
    pub bindings: BTreeMap<String, String>,
    /// `root_template` resolvido para um caminho real.
    pub root_path: PathBuf,
    /// Resto do padrão, relativo a `root_path`; `None` quando não há glob.
    pub tail_glob: Option<String>,
}

const INSTALL_KEYS: [&str; 4] = ["base", "root", "game", "storeGameId"];

enum Seg<'t> {
    Lit(&'t str),
    Ph(&'t str),
}

/// Caminho com `/` e sem barra no fim.
pub(crate) fn slash(p: &Path) -> String {
    let s = p.to_string_lossy().replace('\\', "/");
    s.trim_end_matches('/').to_string()
}

/// Tira `.` e resolve `..` sem tocar no disco (`C:/a/./b/../c` → `C:/a/c`).
/// Nunca sobe acima da raiz do disco.
pub(crate) fn normalize(path: &str) -> String {
    let mut out: Vec<&str> = Vec::new();
    for (i, c) in path.split('/').enumerate() {
        match c {
            "" if i > 1 => {}
            "." if i > 0 => {}
            ".." => match out.last() {
                None | Some(&"..") => out.push(".."),
                Some(l) if l.is_empty() || l.ends_with(':') => {}
                Some(_) => {
                    out.pop();
                }
            },
            _ => out.push(c),
        }
    }
    out.join("/")
}

fn system_value(name: &str, sys: &SystemPaths) -> Option<String> {
    let p = match name {
        "home" => &sys.home,
        "osUserName" => return Some(sys.os_user_name.clone()),
        "winAppData" => &sys.app_data,
        "winLocalAppData" => &sys.local_app_data,
        "winLocalAppDataLow" => &sys.local_app_data_low,
        "winDocuments" => &sys.documents,
        "winPublic" => &sys.public,
        "winProgramData" => &sys.program_data,
        "winDir" => &sys.win_dir,
        "programFiles" => &sys.program_files,
        "programFilesX86" => &sys.program_files_x86,
        _ => return None,
    };
    Some(slash(p))
}

fn is_multi(name: &str) -> bool {
    INSTALL_KEYS.contains(&name) || name == "storeUserId" || name == "gameDir"
}

/// Separa um componente em trechos literais e placeholders `<nome>`.
fn segments(comp: &str) -> Vec<Seg<'_>> {
    let mut out = Vec::new();
    let mut rest = comp;
    while let Some(start) = rest.find('<') {
        let after = &rest[start + 1..];
        let name_len = after.find(|c: char| !c.is_ascii_alphanumeric()).unwrap_or(after.len());
        if name_len > 0 && after[name_len..].starts_with('>') {
            if start > 0 {
                out.push(Seg::Lit(&rest[..start]));
            }
            out.push(Seg::Ph(&after[..name_len]));
            rest = &after[name_len + 1..];
        } else {
            out.push(Seg::Lit(&rest[..=start]));
            rest = after;
        }
    }
    if !rest.is_empty() {
        out.push(Seg::Lit(rest));
    }
    out
}

const ESCAPES: [(&str, char); 4] = [("[[]", '['), ("[]]", ']'), ("[*]", '*'), ("[?]", '?')];

/// Desfaz as escapes de glob de um trecho literal.
fn unescape(lit: &str) -> String {
    let mut out = String::with_capacity(lit.len());
    let mut rest = lit;
    'outer: while !rest.is_empty() {
        for (seq, ch) in ESCAPES {
            if let Some(r) = rest.strip_prefix(seq) {
                out.push(ch);
                rest = r;
                continue 'outer;
            }
        }
        let c = rest.chars().next().unwrap();
        out.push(c);
        rest = &rest[c.len_utf8()..];
    }
    out
}

/// O componente tem glob depois de removidas as escapes `[[]`, `[]]`, `[*]`, `[?]`?
pub fn has_glob(component: &str) -> bool {
    let mut stripped = component.to_string();
    for (seq, _) in ESCAPES {
        stripped = stripped.replace(seq, "");
    }
    stripped.contains(['*', '?', '['])
}

fn components(template: &str) -> Vec<String> {
    template.replace('\\', "/").split('/').map(str::to_string).collect()
}

fn placeholder_names(comps: &[String]) -> Vec<String> {
    let mut names = Vec::new();
    for c in comps {
        for seg in segments(c) {
            if let Seg::Ph(n) = seg {
                if !names.iter().any(|x: &String| x == n) {
                    names.push(n.to_string());
                }
            }
        }
    }
    names
}

/// Nomes dos placeholders que aparecem num template (sem repetir).
pub(crate) fn names_in(template: &str) -> Vec<String> {
    placeholder_names(&components(template))
}

/// Expande um template da base para cada combinação possível neste PC.
pub fn expand(template: &str, ctx: &ExpandCtx) -> Vec<Expansion> {
    let comps = components(template);
    let names = placeholder_names(&comps);
    if names.iter().any(|n| system_value(n, ctx.sys).is_none() && !is_multi(n)) {
        return Vec::new();
    }

    let uses_install = names.iter().any(|n| INSTALL_KEYS.contains(&n.as_str()));
    let uses_user = names.iter().any(|n| n == "storeUserId");
    let uses_game_dir = names.iter().any(|n| n == "gameDir");
    let uses_store_id = names.iter().any(|n| n == "storeGameId");

    let installs: Vec<Option<&InstallCandidate>> = if uses_install {
        ctx.installs
            .iter()
            .filter(|i| !uses_store_id || i.store_game_id.is_some())
            .map(Some)
            .collect()
    } else {
        vec![None]
    };
    // `None` = id desconhecido, vira o glob `*`.
    let users: Vec<Option<&str>> = if uses_user && !ctx.store_user_ids.is_empty() {
        ctx.store_user_ids.iter().map(|s| Some(s.as_str())).collect()
    } else {
        vec![None]
    };
    let game_dir = ctx.game_dir.map(slash);
    if uses_game_dir && game_dir.is_none() {
        return Vec::new();
    }

    let mut out: Vec<Expansion> = Vec::new();
    for install in &installs {
        for user in &users {
            let mut bindings = BTreeMap::new();
            if let Some(i) = install {
                bindings.insert("base".to_string(), slash(&i.base));
                bindings.insert("root".to_string(), slash(&i.root));
                bindings.insert("game".to_string(), i.game.clone());
                if let Some(id) = &i.store_game_id {
                    bindings.insert("storeGameId".to_string(), id.clone());
                }
            }
            if let (true, Some(u)) = (uses_user, user) {
                bindings.insert("storeUserId".to_string(), u.to_string());
            }
            if let (true, Some(gd)) = (uses_game_dir, &game_dir) {
                bindings.insert("gameDir".to_string(), gd.clone());
            }
            let value = |n: &str| system_value(n, ctx.sys).or_else(|| bindings.get(n).cloned());

            let mut root_tmpl = Vec::new();
            let mut root_real = Vec::new();
            let mut tail = Vec::new();
            for comp in &comps {
                let unknown_user = comp.contains("<storeUserId>") && user.is_none();
                if tail.is_empty() && !has_glob(comp) && !unknown_user {
                    let mut real = String::new();
                    for seg in segments(comp) {
                        match seg {
                            Seg::Lit(l) => real.push_str(&unescape(l)),
                            Seg::Ph(n) => real.push_str(&value(n).unwrap_or_default()),
                        }
                    }
                    root_tmpl.push(comp.as_str());
                    root_real.push(real);
                } else {
                    let mut glob = String::new();
                    for seg in segments(comp) {
                        match seg {
                            Seg::Lit(l) => glob.push_str(l),
                            Seg::Ph("storeUserId") if user.is_none() => glob.push('*'),
                            Seg::Ph(n) => {
                                glob.push_str(&glob::Pattern::escape(&value(n).unwrap_or_default()))
                            }
                        }
                    }
                    tail.push(glob);
                }
            }
            if root_real.is_empty() {
                continue;
            }
            // Bindings registram só o que vale para este caso concreto.
            bindings.retain(|k, _| names.iter().any(|n| n == k));
            let exp = Expansion {
                root_template: root_tmpl.join("/"),
                bindings,
                root_path: PathBuf::from(normalize(&root_real.join("/"))),
                tail_glob: if tail.is_empty() { None } else { Some(tail.join("/")) },
            };
            let key = |e: &Expansion| (slash(&e.root_path).to_lowercase(), e.tail_glob.clone());
            if !out.iter().any(|e| key(e) == key(&exp)) {
                out.push(exp);
            }
        }
    }
    out
}

/// Resolve um template de raiz (sem glob) com as pastas deste PC e os bindings dados.
pub fn resolve_root(
    root_template: &str,
    sys: &SystemPaths,
    bindings: &BTreeMap<String, String>,
) -> Option<PathBuf> {
    let mut parts = Vec::new();
    for comp in components(root_template) {
        let mut real = String::new();
        for seg in segments(&comp) {
            match seg {
                Seg::Lit(l) => real.push_str(&unescape(l)),
                Seg::Ph(n) => {
                    let v = system_value(n, sys).or_else(|| {
                        if is_multi(n) { bindings.get(n).cloned() } else { None }
                    })?;
                    real.push_str(&v);
                }
            }
        }
        parts.push(real);
    }
    Some(PathBuf::from(normalize(&parts.join("/"))))
}
