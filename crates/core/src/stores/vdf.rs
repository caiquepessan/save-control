//! Leitor do formato KeyValues da Steam (`.vdf`, `.acf`).

use crate::error::{Error, Result};

#[derive(Clone, Debug, PartialEq)]
pub enum Vdf {
    Str(String),
    Obj(Vec<(String, Vdf)>),
}

impl Vdf {
    /// Primeiro filho com a chave (sem diferenciar maiúsculas).
    pub fn get(&self, key: &str) -> Option<&Vdf> {
        self.entries().iter().find(|(k, _)| k.eq_ignore_ascii_case(key)).map(|(_, v)| v)
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Vdf::Str(s) => Some(s),
            Vdf::Obj(_) => None,
        }
    }

    pub fn entries(&self) -> &[(String, Vdf)] {
        match self {
            Vdf::Obj(items) => items,
            Vdf::Str(_) => &[],
        }
    }
}

enum Tok {
    Str(String),
    Open,
    Close,
}

fn err(msg: &str) -> Error {
    Error::Parse(format!("vdf: {msg}"))
}

fn tokenize(text: &str) -> Result<Vec<Tok>> {
    let mut toks = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            c if c.is_whitespace() => {}
            '/' if chars.peek() == Some(&'/') => {
                for c in chars.by_ref() {
                    if c == '\n' {
                        break;
                    }
                }
            }
            '{' => toks.push(Tok::Open),
            '}' => toks.push(Tok::Close),
            '"' => {
                let mut s = String::new();
                loop {
                    match chars.next() {
                        None => return Err(err("texto sem aspas de fechamento")),
                        Some('"') => break,
                        Some('\\') => match chars.next() {
                            Some('n') => s.push('\n'),
                            Some('t') => s.push('\t'),
                            Some(c @ ('\\' | '"')) => s.push(c),
                            Some(c) => {
                                s.push('\\');
                                s.push(c);
                            }
                            None => return Err(err("escape no fim do arquivo")),
                        },
                        Some(c) => s.push(c),
                    }
                }
                toks.push(Tok::Str(s));
            }
            c => {
                let mut s = String::from(c);
                while let Some(&n) = chars.peek() {
                    if n.is_whitespace() || matches!(n, '{' | '}' | '"') {
                        break;
                    }
                    s.push(n);
                    chars.next();
                }
                toks.push(Tok::Str(s));
            }
        }
    }
    Ok(toks)
}

fn items(toks: &[Tok], pos: &mut usize, nested: bool) -> Result<Vec<(String, Vdf)>> {
    let mut out = Vec::new();
    loop {
        match toks.get(*pos) {
            None if nested => return Err(err("faltou '}'")),
            None => return Ok(out),
            Some(Tok::Close) if nested => {
                *pos += 1;
                return Ok(out);
            }
            Some(Tok::Close) | Some(Tok::Open) => return Err(err("chave esperada")),
            Some(Tok::Str(key)) => {
                *pos += 1;
                match toks.get(*pos) {
                    Some(Tok::Str(v)) => {
                        out.push((key.clone(), Vdf::Str(v.clone())));
                        *pos += 1;
                    }
                    Some(Tok::Open) => {
                        *pos += 1;
                        let sub = items(toks, pos, true)?;
                        out.push((key.clone(), Vdf::Obj(sub)));
                    }
                    _ => return Err(err("valor esperado")),
                }
            }
        }
    }
}

pub fn parse(text: &str) -> Result<Vdf> {
    let toks = tokenize(text)?;
    let mut pos = 0;
    Ok(Vdf::Obj(items(&toks, &mut pos, false)?))
}
