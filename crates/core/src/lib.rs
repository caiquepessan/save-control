//! Núcleo do Save Control: encontrar saves, guardar no cofre e restaurar.

pub mod catalog;
pub mod custom;
pub mod env;
pub mod error;
pub mod fsglob;
pub mod fsutil;
pub mod local_state;
pub mod machine;
pub mod manifest;
pub mod model;
pub mod placeholder;
pub mod registry;
pub mod scan;
pub mod stores;
pub mod templatize;
pub mod vault;
