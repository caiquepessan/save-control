//! Saber se um jogo está aberto.

use std::path::{Path, PathBuf};

use crate::model::Game;
use crate::placeholder::slash;

#[derive(Clone, Debug)]
pub struct RunningProcess {
    pub name: String,
    pub exe_path: Option<PathBuf>,
}

pub trait ProcessProbe {
    fn running(&self) -> Vec<RunningProcess>;
}

/// Processos de verdade do Windows.
pub struct SystemProbe;

impl ProcessProbe for SystemProbe {
    fn running(&self) -> Vec<RunningProcess> {
        let Ok(mut sys) = sysinfo::System::new() else { return Vec::new() };
        sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
        sys.processes()
            .values()
            .map(|p| RunningProcess {
                name: p.name().to_string_lossy().into_owned(),
                exe_path: p.exe().map(Path::to_path_buf),
            })
            .collect()
    }
}

/// Lista fixa de processos, para testes.
pub struct FakeProbe(pub Vec<RunningProcess>);

impl ProcessProbe for FakeProbe {
    fn running(&self) -> Vec<RunningProcess> {
        self.0.clone()
    }
}

fn is_under(path: &Path, dir: &Path) -> bool {
    let (p, d) = (slash(path).to_lowercase(), slash(dir).to_lowercase());
    !d.is_empty() && (p == d || p.starts_with(&format!("{d}/")))
}

/// Dá para saber se o jogo está aberto: ele tem nome de processo (catálogo ou
/// à mão), pasta de instalação ou pasta do jogo conhecida.
pub fn can_tell_running(game: &Game) -> bool {
    !game.process_names.is_empty() || !game.install_dirs.is_empty() || game.game_dir.is_some()
}

/// Aberto = algum processo com o nome do executável do jogo, ou rodando de
/// dentro da pasta de instalação dele.
pub fn is_game_running(game: &Game, probe: &dyn ProcessProbe) -> bool {
    let dirs: Vec<&PathBuf> = game.install_dirs.iter().chain(game.game_dir.iter()).collect();
    probe.running().iter().any(|p| {
        game.process_names.iter().any(|n| n.eq_ignore_ascii_case(&p.name))
            || p.exe_path.as_deref().is_some_and(|e| dirs.iter().any(|d| is_under(e, d)))
    })
}
