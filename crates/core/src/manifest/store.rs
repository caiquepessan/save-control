//! Cache e atualização diária da base de jogos.

use std::io::Read;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Duration, Utc};

use crate::error::{Error, Result, io};
use crate::manifest::Manifest;

pub const MANIFEST_URL: &str =
    "https://raw.githubusercontent.com/mtkennerly/ludusavi-manifest/refs/heads/data/data/manifest.yaml";

const YAML: &str = "manifest.yaml";
const ETAG: &str = "manifest.etag";
const CACHE: &str = "manifest.cache";
const LAST_CHECK: &str = "ultima-verificacao.txt";

pub enum FetchResult {
    NotModified,
    Fetched { body: Vec<u8>, etag: Option<String> },
}

pub trait Fetcher {
    fn fetch(&self, url: &str, etag: Option<&str>) -> Result<FetchResult>;
}

/// Busca pela internet, com `If-None-Match` quando já existe um ETag.
pub struct HttpFetcher;

impl Fetcher for HttpFetcher {
    fn fetch(&self, url: &str, etag: Option<&str>) -> Result<FetchResult> {
        let mut req = ureq::get(url).set("User-Agent", "save-control");
        if let Some(e) = etag {
            req = req.set("If-None-Match", e);
        }
        match req.call() {
            Ok(resp) if resp.status() == 304 => Ok(FetchResult::NotModified),
            Ok(resp) => {
                let etag = resp.header("ETag").map(str::to_string);
                let mut body = Vec::new();
                resp.into_reader()
                    .read_to_end(&mut body)
                    .map_err(|e| Error::Http(e.to_string()))?;
                Ok(FetchResult::Fetched { body, etag })
            }
            Err(ureq::Error::Status(304, _)) => Ok(FetchResult::NotModified),
            Err(e) => Err(Error::Http(e.to_string())),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpdateOutcome {
    Skipped,
    NotModified,
    Updated,
}

pub struct ManifestStore {
    dir: PathBuf,
}

impl ManifestStore {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }

    fn read_text(&self, name: &str) -> Option<String> {
        std::fs::read_to_string(self.path(name)).ok().map(|s| s.trim().to_string())
    }

    /// Base guardada: do cache binário se o ETag bater; senão relê o YAML e
    /// refaz o cache. `None` se nunca foi baixada.
    pub fn load(&self) -> Result<Option<Manifest>> {
        let yaml = self.path(YAML);
        if !yaml.exists() {
            return Ok(None);
        }
        let etag = self.read_text(ETAG).unwrap_or_default();
        if let Ok(bytes) = std::fs::read(self.path(CACHE)) {
            if let Ok((cached_etag, m)) = bincode::deserialize::<(String, Manifest)>(&bytes) {
                if cached_etag == etag {
                    return Ok(Some(m));
                }
            }
        }
        let text = std::fs::read_to_string(&yaml).map_err(io(&yaml))?;
        let m = Manifest::from_yaml(&text)?;
        self.write_cache(&etag, &m)?;
        Ok(Some(m))
    }

    /// Confere a base na internet no máximo uma vez a cada 24 h.
    pub fn update(&self, fetcher: &dyn Fetcher, now: DateTime<Utc>) -> Result<UpdateOutcome> {
        let have_yaml = self.path(YAML).exists();
        let last = self
            .read_text(LAST_CHECK)
            .and_then(|t| DateTime::parse_from_rfc3339(&t).ok())
            .map(|t| t.with_timezone(&Utc));
        if have_yaml && last.is_some_and(|l| now - l < Duration::hours(24)) {
            return Ok(UpdateOutcome::Skipped);
        }
        let etag = if have_yaml { self.read_text(ETAG).filter(|e| !e.is_empty()) } else { None };
        let outcome = match fetcher.fetch(MANIFEST_URL, etag.as_deref())? {
            FetchResult::NotModified => UpdateOutcome::NotModified,
            FetchResult::Fetched { body, etag } => {
                let text = String::from_utf8(body)
                    .map_err(|e| Error::Parse(format!("base de jogos: {e}")))?;
                let m = Manifest::from_yaml(&text)?;
                let etag = etag.unwrap_or_default();
                write_atomic(&self.path(YAML), text.as_bytes())?;
                write_atomic(&self.path(ETAG), etag.as_bytes())?;
                self.write_cache(&etag, &m)?;
                UpdateOutcome::Updated
            }
        };
        write_atomic(&self.path(LAST_CHECK), now.to_rfc3339().as_bytes())?;
        Ok(outcome)
    }

    fn write_cache(&self, etag: &str, m: &Manifest) -> Result<()> {
        let bytes = bincode::serialize(&(etag, m)).map_err(|e| Error::Parse(e.to_string()))?;
        write_atomic(&self.path(CACHE), &bytes)
    }
}

/// Grava num temporário ao lado e renomeia por cima.
pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(io(parent))?;
    }
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    std::fs::write(&tmp, bytes).map_err(io(&tmp))?;
    std::fs::rename(&tmp, path).map_err(io(path))
}
