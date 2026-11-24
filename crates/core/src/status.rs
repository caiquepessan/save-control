//! Status de um jogo em relação ao cofre.

use serde::{Deserialize, Serialize};

use crate::fingerprint::Fingerprint;
use crate::model::Game;
use crate::vault::index::VaultIndex;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameStatus {
    /// O save atual é idêntico a algum backup (de qualquer PC).
    Protected,
    /// O save atual não bate com nenhum backup.
    NewSave,
    NoBackup,
    NoSave,
}

pub fn status_of(game: &Game, current: &Fingerprint, index: &VaultIndex) -> GameStatus {
    if game.files.is_empty() {
        GameStatus::NoSave
    } else if index.for_game(&game.id).is_empty() {
        GameStatus::NoBackup
    } else if index.find_identical(&game.id, current).is_some() {
        GameStatus::Protected
    } else {
        GameStatus::NewSave
    }
}
