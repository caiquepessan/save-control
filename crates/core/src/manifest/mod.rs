//! Base de locais de save de jogos de PC (ludusavi-manifest, dados do PCGamingWiki).

pub mod store;

use std::collections::BTreeMap;

use serde::de::{Deserializer, IgnoredAny};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    pub games: BTreeMap<String, ManifestGame>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ManifestGame {
    pub files: Vec<ManifestFile>,
    pub install_dirs: Vec<String>,
    pub steam_id: Option<u32>,
    pub steam_extra: Vec<u32>,
    pub gog_id: Option<u64>,
    pub alias: Option<String>,
    pub cloud: CloudFlags,
    pub has_registry: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ManifestFile {
    pub path: String,
    pub tags: Vec<String>,
    pub when: Vec<When>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct When {
    pub os: Option<String>,
    pub store: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CloudFlags {
    pub steam: bool,
    pub epic: bool,
    pub gog: bool,
    pub origin: bool,
    pub uplay: bool,
}

impl Manifest {
    /// Lê o YAML da base de forma leniente: campos e valores desconhecidos são ignorados.
    pub fn from_yaml(text: &str) -> Result<Manifest> {
        let raw: BTreeMap<String, Option<RawGame>> =
            serde_yaml::from_str(text).map_err(|e| Error::Parse(format!("base de jogos: {e}")))?;
        let games = raw
            .into_iter()
            .map(|(title, g)| (title, g.unwrap_or_default().into_game()))
            .collect();
        Ok(Manifest { games })
    }

    /// Entrada de um título, seguindo `alias` uma vez. Devolve o título real.
    pub fn entry(&self, title: &str) -> Option<(&str, &ManifestGame)> {
        let (k, g) = self.games.get_key_value(title)?;
        if let Some(target) = &g.alias {
            if let Some((k2, g2)) = self.games.get_key_value(target.as_str()) {
                return Some((k2.as_str(), g2));
            }
        }
        Some((k.as_str(), g))
    }
}

impl ManifestGame {
    /// Arquivos que valem para o Windows. Sem tag conta como save; `config` só se pedido.
    pub fn save_files_for_windows(&self, include_config: bool) -> Vec<&ManifestFile> {
        self.files
            .iter()
            .filter(|f| applies_to_windows(&f.when) && wanted(&f.tags, include_config))
            .collect()
    }
}

fn applies_to_windows(when: &[When]) -> bool {
    when.is_empty()
        || when
            .iter()
            .any(|w| w.os.as_deref().is_none_or(|os| os.eq_ignore_ascii_case("windows")))
}

fn wanted(tags: &[String], include_config: bool) -> bool {
    tags.is_empty()
        || tags.iter().any(|t| t == "save")
        || (include_config && tags.iter().any(|t| t == "config"))
}

// ---- formato "cru" do YAML, leniente ----

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct RawGame {
    files: BTreeMap<String, Option<RawFile>>,
    registry: BTreeMap<String, IgnoredAny>,
    #[serde(deserialize_with = "keys_only")]
    install_dir: Vec<String>,
    steam: Option<RawId>,
    gog: Option<RawId>,
    id: Option<RawExtraIds>,
    alias: Option<String>,
    cloud: Option<RawCloud>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct RawFile {
    tags: Vec<String>,
    when: Vec<RawWhen>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct RawWhen {
    os: Option<String>,
    store: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct RawId {
    id: Option<u64>,
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct RawExtraIds {
    steam_extra: Vec<u64>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct RawCloud {
    steam: bool,
    epic: bool,
    gog: bool,
    origin: bool,
    uplay: bool,
}

fn keys_only<'de, D: Deserializer<'de>>(d: D) -> std::result::Result<Vec<String>, D::Error> {
    let m: Option<BTreeMap<String, IgnoredAny>> = Option::deserialize(d)?;
    Ok(m.map(|m| m.into_keys().collect()).unwrap_or_default())
}

impl RawGame {
    fn into_game(self) -> ManifestGame {
        let files = self
            .files
            .into_iter()
            .map(|(path, f)| {
                let f = f.unwrap_or_default();
                ManifestFile {
                    path,
                    tags: f.tags,
                    when: f.when.into_iter().map(|w| When { os: w.os, store: w.store }).collect(),
                }
            })
            .collect();
        let cloud = self.cloud.unwrap_or_default();
        ManifestGame {
            files,
            install_dirs: self.install_dir,
            steam_id: self.steam.and_then(|s| s.id).and_then(|id| u32::try_from(id).ok()),
            steam_extra: self
                .id
                .map(|i| i.steam_extra.into_iter().filter_map(|x| u32::try_from(x).ok()).collect())
                .unwrap_or_default(),
            gog_id: self.gog.and_then(|g| g.id),
            alias: self.alias,
            cloud: CloudFlags {
                steam: cloud.steam,
                epic: cloud.epic,
                gog: cloud.gog,
                origin: cloud.origin,
                uplay: cloud.uplay,
            },
            has_registry: !self.registry.is_empty(),
        }
    }
}
