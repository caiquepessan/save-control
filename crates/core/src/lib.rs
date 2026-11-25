//! Núcleo do Save Control: encontrar saves, guardar no cofre e restaurar.

pub mod auto;
pub mod backup;
pub mod catalog;
pub mod custom;
pub mod engine;
pub mod env;
pub mod error;
pub mod fingerprint;
pub mod fsglob;
pub mod fsutil;
pub mod local_state;
pub mod machine;
pub mod manifest;
pub mod model;
pub mod placeholder;
pub mod process;
pub mod registry;
pub mod restore;
pub mod scan;
pub mod status;
pub mod stores;
pub mod targets;
pub mod templatize;
pub mod vault;
