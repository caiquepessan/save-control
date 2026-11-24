//! Erros do núcleo e do app no formato que a interface entende.

use savecontrol_core::error::Error;

use crate::dto::{ErrorKind, ErrorView};
use crate::view::path_str;

impl ErrorView {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> ErrorView {
        ErrorView { kind, message: message.into(), root: None, alias: None, candidates: Vec::new() }
    }
}

impl std::fmt::Display for ErrorView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ErrorView {}

impl From<Error> for ErrorView {
    fn from(e: Error) -> ErrorView {
        let message = e.to_string();
        match e {
            Error::GameRunning(_) => ErrorView::new(ErrorKind::JogoAberto, message),
            Error::NeedsTarget { root, candidates } => ErrorView {
                kind: ErrorKind::PrecisaDestino,
                message,
                root: Some(root),
                alias: None,
                candidates: candidates.iter().map(|c| path_str(c)).collect(),
            },
            Error::VaultUnavailable(_) => ErrorView::new(ErrorKind::CofreIndisponivel, message),
            Error::UnsafeRestore(_) => ErrorView::new(ErrorKind::RestauracaoRecusada, message),
            Error::RollbackFailed { .. } => ErrorView::new(ErrorKind::DesfazerFalhou, message),
            Error::FormatTooNew { .. } => ErrorView::new(ErrorKind::VersaoNova, message),
            Error::NotFound(_) => ErrorView::new(ErrorKind::NaoEncontrado, message),
            _ => ErrorView::new(ErrorKind::Erro, message),
        }
    }
}
