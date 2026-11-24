use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, Error>;

/// Erros do núcleo. As mensagens aparecem para o usuário, por isso em pt-BR.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Erro ao acessar {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Arquivo inválido ({0})")]
    Parse(String),
    #[error("Falha de rede: {0}")]
    Http(String),
    #[error("O arquivo {0} continuou mudando durante a cópia")]
    UnstableFile(PathBuf),
    #[error("O arquivo {0} está em uso por outro programa")]
    FileLocked(PathBuf),
    #[error("O cofre em {0} não está acessível")]
    VaultUnavailable(PathBuf),
    #[error("Espaço insuficiente: precisa de {needed} bytes, há {available}")]
    InsufficientSpace { needed: u64, available: u64 },
    #[error("Feche {0} antes de restaurar")]
    GameRunning(String),
    #[error("Escolha onde restaurar {root}")]
    NeedsTarget { root: String, candidates: Vec<PathBuf> },
    #[error("A verificação falhou em {0}")]
    VerifyFailed(PathBuf),
    #[error("Não encontrado: {0}")]
    NotFound(String),
}

/// Adaptador para `map_err`: `fs::read(&p).map_err(io(&p))?`.
pub fn io(path: impl Into<PathBuf>) -> impl FnOnce(std::io::Error) -> Error {
    let path = path.into();
    move |source| Error::Io { path, source }
}
