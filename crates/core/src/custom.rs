//! Jogos adicionados à mão pelo usuário.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::env::SystemPaths;
use crate::templatize::templatize;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CustomGame {
    pub id: String,
    pub name: String,
    pub save_templates: Vec<String>,
    pub process_names: Vec<String>,
}

impl CustomGame {
    pub fn new(name: &str, save_dirs: &[PathBuf], process_names: &[String], sys: &SystemPaths) -> CustomGame {
        CustomGame {
            id: format!("man:{}", uuid::Uuid::new_v4().simple()),
            name: name.trim().to_string(),
            save_templates: save_dirs.iter().map(|d| templatize(d, sys, None)).collect(),
            process_names: process_names.to_vec(),
        }
    }
}
