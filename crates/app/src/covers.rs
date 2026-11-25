//! Capas dos jogos: a arte vertical oficial da Steam (baixada uma vez), a
//! imagem escolhida pelo usuário, ou a capa gerada pela interface.

use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration as StdDuration;

use chrono::{DateTime, Duration, Utc};

use savecontrol_core::fsutil::write_atomic;
use savecontrol_core::local_state::LocalGameSettings;

/// Depois de uma falha (offline, 404, resposta estranha), só tenta de novo depois disto.
const RETRY_AFTER_DAYS: i64 = 7;
const MAX_COVER_BYTES: u64 = 10 * 1024 * 1024;
const CUSTOM_EXTENSIONS: [&str; 4] = ["png", "jpg", "jpeg", "webp"];

pub fn steam_cover_url(appid: u32) -> String {
    format!("https://cdn.cloudflare.steamstatic.com/steam/apps/{appid}/library_600x900.jpg")
}

/// Baixa uma URL. `Ok(None)` = a imagem não existe (404).
pub trait CoverFetcher {
    fn get(&self, url: &str) -> Result<Option<Vec<u8>>, String>;
}

pub struct UreqCoverFetcher {
    agent: ureq::Agent,
}

impl Default for UreqCoverFetcher {
    fn default() -> Self {
        UreqCoverFetcher { agent: ureq::AgentBuilder::new().timeout(StdDuration::from_secs(10)).build() }
    }
}

impl CoverFetcher for UreqCoverFetcher {
    fn get(&self, url: &str) -> Result<Option<Vec<u8>>, String> {
        match self.agent.get(url).call() {
            Ok(resp) => {
                let mut buf = Vec::new();
                resp.into_reader().take(MAX_COVER_BYTES).read_to_end(&mut buf).map_err(|e| e.to_string())?;
                Ok(Some(buf))
            }
            Err(ureq::Error::Status(404, _)) => Ok(None),
            Err(ureq::Error::Status(code, _)) => Err(format!("HTTP {code}")),
            Err(e) => Err(e.to_string()),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum CoverChoice {
    Custom(PathBuf),
    Steam(PathBuf),
    Generated,
}

#[derive(Clone, Debug)]
pub struct CoverStore {
    pub dir: PathBuf,
}

fn is_jpeg(bytes: &[u8]) -> bool {
    bytes.len() > 1000 && bytes.starts_with(&[0xFF, 0xD8])
}

impl CoverStore {
    pub fn steam_path(&self, appid: u32) -> PathBuf {
        self.dir.join(format!("steam-{appid}.jpg"))
    }

    fn failures_path(&self) -> PathBuf {
        self.dir.join("falhas.json")
    }

    fn load_failures(&self) -> BTreeMap<u32, DateTime<Utc>> {
        std::fs::read_to_string(self.failures_path())
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    fn save_failures(&self, failures: &BTreeMap<u32, DateTime<Utc>>) {
        if let Ok(json) = serde_json::to_vec_pretty(failures) {
            let _ = write_atomic(&self.failures_path(), &json);
        }
    }

    /// A capa da Steam deste jogo, baixando se ainda não tiver. `None` = usar a gerada.
    pub fn ensure_steam(&self, appid: u32, fetcher: &dyn CoverFetcher, now: DateTime<Utc>) -> Option<PathBuf> {
        let path = self.steam_path(appid);
        if path.is_file() {
            return Some(path);
        }
        let mut failures = self.load_failures();
        if failures.get(&appid).is_some_and(|when| now - *when < Duration::days(RETRY_AFTER_DAYS)) {
            return None;
        }
        let saved = match fetcher.get(&steam_cover_url(appid)) {
            Ok(Some(bytes)) if is_jpeg(&bytes) => write_atomic(&path, &bytes).is_ok(),
            _ => false,
        };
        if saved {
            if failures.remove(&appid).is_some() {
                self.save_failures(&failures);
            }
            Some(path)
        } else {
            failures.insert(appid, now);
            self.save_failures(&failures);
            None
        }
    }

    /// Copia a imagem escolhida para a pasta de capas (o original pode sumir depois).
    pub fn import_custom(&self, src: &Path) -> Result<PathBuf, String> {
        let ext = src
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase)
            .filter(|e| CUSTOM_EXTENSIONS.contains(&e.as_str()))
            .ok_or_else(|| "Escolha uma imagem PNG, JPG ou WEBP.".to_string())?;
        let bytes = std::fs::read(src).map_err(|e| format!("Não foi possível ler a imagem: {e}"))?;
        let hash = blake3::hash(&bytes).to_hex();
        let dest = self.dir.join(format!("custom-{}.{ext}", &hash[..8]));
        write_atomic(&dest, &bytes).map_err(|e| e.to_string())?;
        Ok(dest)
    }

    /// A própria (se o arquivo existe) > a da Steam (se já baixada) > a gerada.
    pub fn choice(&self, steam_app_id: Option<u32>, settings: &LocalGameSettings) -> CoverChoice {
        // Guardada pelo nome do arquivo (relativa à pasta de capas); caminhos completos antigos também valem.
        let custom = settings.cover.as_ref().map(|c| if c.is_relative() { self.dir.join(c) } else { c.clone() });
        if let Some(custom) = custom.filter(|c| c.is_file()) {
            return CoverChoice::Custom(custom);
        }
        if let Some(path) = steam_app_id.map(|id| self.steam_path(id)).filter(|p| p.is_file()) {
            return CoverChoice::Steam(path);
        }
        CoverChoice::Generated
    }
}
