//! O trabalhador do backup automático (spec §9). Uma thread que junta o vigia
//! de pastas, os processos abertos, a varredura leve, os escaneamentos, as
//! pausas e o cofre fora, e faz os backups pelo `AppService`.
//!
//! Relógio, processos, base de jogos e "avisos para fora" (bandeja, janela,
//! notificação) são injetáveis: tudo aqui tem teste sem Tauri e sem esperar.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{RecvTimeoutError, Receiver, Sender};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::JoinHandle;

use chrono::{DateTime, Duration, Utc};
use notify::Watcher as _;

use savecontrol_core::auto::{Due, DueReason, Scheduler};
use savecontrol_core::manifest::store::Fetcher;
use savecontrol_core::process::{ProcessProbe, RunningProcess};

use crate::dto::*;
use crate::service::AppService;
use crate::texts;

/// Ritmos (Decisão 2 do plano).
fn processos() -> Duration {
    Duration::seconds(5)
}

fn cofre() -> Duration {
    Duration::seconds(30)
}

fn varredura() -> Duration {
    Duration::minutes(10)
}

fn completo() -> Duration {
    Duration::hours(6)
}

/// O primeiro escaneamento completo espera um pouco depois de abrir: a janela
/// já escaneia ao abrir, e baixar a base logo no início seguraria o serviço.
fn primeiro_completo() -> Duration {
    Duration::minutes(2)
}

/// A interface é avisada de mudanças no máximo a cada 2 s.
fn aviso_biblioteca() -> Duration {
    Duration::seconds(2)
}

pub trait Clock: Send {
    fn now(&self) -> DateTime<Utc>;
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// O que o trabalhador conta para fora (bandeja, janela, notificação).
pub trait Sink: Send {
    fn status(&self, s: &AutoStatusView);
    fn library_changed(&self);
    /// Backups automáticos feitos num lote. `game_closed`: algum veio de "o jogo fechou".
    fn backed_up(&self, items: &[AutoBackupView], game_closed: bool);
    fn notice(&self, text: &str);
}

pub enum Command {
    /// Pausar por um tempo; `None` = até reabrir o app.
    Pause(Option<std::time::Duration>),
    Resume,
    SetEnabled(bool),
    ScanNow,
    /// Um arquivo mudou (vigia de pastas).
    Changed(PathBuf),
    /// Configurações mudaram (tempos, automático geral ou de um jogo).
    ConfigChanged,
    Quit,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Pausa {
    Nenhuma,
    Ate(DateTime<Utc>),
    AteReabrir,
}

/// Lista fixa de processos, lida uma vez fora da trava do serviço.
struct Lista(Vec<RunningProcess>);

impl ProcessProbe for Lista {
    fn running(&self) -> Vec<RunningProcess> {
        self.0.clone()
    }
}

/// Backups automáticos com prioridade baixa de disco (§14): a thread fica no
/// modo "em segundo plano" do Windows enquanto o guarda existe.
struct PrioridadeBaixa;

impl PrioridadeBaixa {
    fn nova() -> PrioridadeBaixa {
        #[cfg(windows)]
        unsafe {
            use windows_sys::Win32::System::Threading::{GetCurrentThread, SetThreadPriority, THREAD_MODE_BACKGROUND_BEGIN};
            SetThreadPriority(GetCurrentThread(), THREAD_MODE_BACKGROUND_BEGIN);
        }
        PrioridadeBaixa
    }
}

impl Drop for PrioridadeBaixa {
    fn drop(&mut self) {
        #[cfg(windows)]
        unsafe {
            use windows_sys::Win32::System::Threading::{GetCurrentThread, SetThreadPriority, THREAD_MODE_BACKGROUND_END};
            SetThreadPriority(GetCurrentThread(), THREAD_MODE_BACKGROUND_END);
        }
    }
}

pub struct AutoWorker {
    svc: Arc<Mutex<AppService>>,
    sched: Scheduler,
    probe: Box<dyn ProcessProbe + Send>,
    sink: Box<dyn Sink>,
    clock: Box<dyn Clock>,
    fetcher: Box<dyn Fetcher + Send>,
    pausa: Pausa,
    /// A Engine está aberta (fase Pronto).
    pronto: bool,
    fase: Option<AppPhase>,
    cofre_ok: bool,
    ultimo_cofre: Option<DateTime<Utc>>,
    ultimo_processo: Option<DateTime<Utc>>,
    ultima_varredura: Option<DateTime<Utc>>,
    ultimo_completo: Option<DateTime<Utc>>,
    ultimo_aviso: Option<DateTime<Utc>>,
    biblioteca_mudou: bool,
    raizes_velhas: bool,
    ultimo_backup: Option<LastBackupView>,
    vigiados: usize,
    raizes: Vec<PathBuf>,
    versao_raizes: u64,
    ocupado: bool,
    ultimo_status: Option<AutoStatusView>,
    sair: bool,
}

impl AutoWorker {
    pub fn new(
        svc: Arc<Mutex<AppService>>,
        probe: Box<dyn ProcessProbe + Send>,
        sink: Box<dyn Sink>,
        clock: Box<dyn Clock>,
        fetcher: Box<dyn Fetcher + Send>,
    ) -> AutoWorker {
        let regras = svc.lock().unwrap_or_else(|e| e.into_inner()).auto_rules();
        let agora = clock.now();
        AutoWorker {
            svc,
            sched: Scheduler::new(regras),
            probe,
            sink,
            clock,
            fetcher,
            pausa: Pausa::Nenhuma,
            pronto: false,
            fase: None,
            cofre_ok: true,
            ultimo_cofre: None,
            ultimo_processo: None,
            ultima_varredura: Some(agora),
            ultimo_completo: Some(agora - completo() + primeiro_completo()),
            ultimo_aviso: None,
            biblioteca_mudou: false,
            raizes_velhas: true,
            ultimo_backup: None,
            vigiados: 0,
            raizes: Vec::new(),
            versao_raizes: 0,
            ocupado: false,
            ultimo_status: None,
            sair: false,
        }
    }

    fn svc(&self) -> MutexGuard<'_, AppService> {
        self.svc.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn passou(ultimo: Option<DateTime<Utc>>, agora: DateTime<Utc>, intervalo: Duration) -> bool {
        ultimo.is_none_or(|u| agora - u >= intervalo)
    }

    pub fn handle(&mut self, cmd: Command) {
        let agora = self.clock.now();
        match cmd {
            Command::Pause(Some(d)) => {
                self.pausa = Pausa::Ate(agora + Duration::from_std(d).unwrap_or_else(|_| Duration::zero()));
            }
            Command::Pause(None) => self.pausa = Pausa::AteReabrir,
            Command::Resume => self.pausa = Pausa::Nenhuma,
            Command::SetEnabled(on) => {
                let _ = self.svc().set_auto_backup(on);
                self.raizes_velhas = true;
                self.agendar_saves_novos(agora);
            }
            Command::ScanNow => self.ultimo_completo = None,
            Command::Changed(p) => self.mudou(&p, agora),
            Command::ConfigChanged => {
                let regras = self.svc().auto_rules();
                self.sched.set_rules(regras);
                self.raizes_velhas = true;
                // A janela escaneia e avisa por aqui: os "Save novo" achados entram na fila.
                self.agendar_saves_novos(agora);
            }
            Command::Quit => self.sair = true,
        }
        self.publicar();
    }

    /// Os "Save novo" achados por um escaneamento (o save mudou com o app
    /// fechado, ou antes de o vigia começar) entram na fila. Os que já estão
    /// nela ficam como estão, para não empurrar o prazo deles.
    fn agendar_saves_novos(&mut self, agora: DateTime<Utc>) {
        let na_fila: HashSet<String> = self.sched.pending().into_iter().collect();
        let novos = self.svc().new_save_ids();
        for id in novos.into_iter().filter(|id| !na_fila.contains(id)) {
            self.sched.note_change(&id, agora);
        }
    }

    /// Um save mudou: o jogo vira "Save novo" na hora e o backup é agendado.
    fn mudou(&mut self, path: &Path, agora: DateTime<Utc>) {
        let ids = {
            let mut s = self.svc();
            let ids = s.game_ids_for_path(path);
            for id in &ids {
                s.mark_changed(id);
            }
            ids
        };
        for id in &ids {
            self.sched.note_change(id, agora);
        }
        if !ids.is_empty() {
            self.biblioteca_mudou = true;
        }
    }

    pub fn tick(&mut self) {
        let agora = self.clock.now();
        if let Pausa::Ate(t) = self.pausa {
            if agora >= t {
                self.pausa = Pausa::Nenhuma;
            }
        }

        // 1. Sem a Engine (primeiro uso, cofre sumido, falhou) não há o que vigiar.
        if !self.pronto {
            if Self::passou(self.ultimo_cofre, agora, cofre()) {
                self.ultimo_cofre = Some(agora);
                let fase = self.svc().state().phase;
                if fase == AppPhase::Pronto {
                    if self.fase == Some(AppPhase::CofreSumiu) {
                        self.sink.notice(texts::COFRE_VOLTOU);
                        self.biblioteca_mudou = true;
                        self.ultimo_completo = None;
                    }
                    self.pronto = true;
                    self.cofre_ok = true;
                    self.raizes_velhas = true;
                }
                self.fase = Some(fase);
            }
            if !self.pronto {
                self.publicar();
                return;
            }
        }

        // 2. Cofre: fora, o automático para; de volta, escaneia.
        if Self::passou(self.ultimo_cofre, agora, cofre()) {
            self.ultimo_cofre = Some(agora);
            let ok = self.svc().vault_available();
            if ok != self.cofre_ok {
                self.cofre_ok = ok;
                self.biblioteca_mudou = true;
                if ok {
                    self.ultimo_completo = None;
                    self.svc().log_info("o cofre voltou: o backup automático continua");
                } else {
                    self.svc().log_info("cofre indisponível: o backup automático pausou");
                }
            }
        }

        // 3. Escaneamento completo e base de jogos (§7.1, §7.5).
        if self.cofre_ok && Self::passou(self.ultimo_completo, agora, completo()) {
            self.ultimo_completo = Some(agora);
            self.ultima_varredura = Some(agora);
            {
                let mut s = self.svc();
                match s.update_manifest(&*self.fetcher) {
                    Ok(v) if v.outcome == ManifestOutcome::Falhou => {
                        s.log_error(&format!("base de jogos: {}", v.message.unwrap_or_default()))
                    }
                    Ok(_) => {}
                    Err(e) => s.log_error(&format!("base de jogos: {}", e.message)),
                }
                if let Err(e) = s.scan_with_covers() {
                    s.log_error(&format!("escaneamento: {}", e.message));
                }
            }
            self.raizes_velhas = true;
            self.biblioteca_mudou = true;
            self.agendar_saves_novos(agora);
        }
        if self.raizes_velhas {
            self.raizes_velhas = false;
            let lista = self.svc().auto_watch_list();
            self.atualizar_raizes(lista);
        }

        // 4. Processos abertos (lidos fora da trava do serviço).
        if Self::passou(self.ultimo_processo, agora, processos()) {
            self.ultimo_processo = Some(agora);
            let abertos = Lista(self.probe.running());
            let estados = self.svc().running_states(&abertos);
            for (id, r) in estados {
                self.sched.note_running(&id, r, agora);
            }
        }

        // 5. Varredura leve: cobre eventos de arquivo que o vigia perdeu.
        if self.cofre_ok && Self::passou(self.ultima_varredura, agora, varredura()) {
            self.ultima_varredura = Some(agora);
            let ids = {
                let mut s = self.svc();
                let ids = s.changed_game_ids();
                for id in &ids {
                    s.mark_changed(id);
                }
                ids
            };
            for id in &ids {
                self.sched.note_change(id, agora);
            }
            if !ids.is_empty() {
                self.biblioteca_mudou = true;
            }
        }

        // 6. Os backups que venceram.
        if self.estado() == AutoState::Ativo {
            let vencidos = self.sched.due(agora);
            if !vencidos.is_empty() {
                self.fazer_backups(vencidos, agora);
            }
        }

        // 7. Avisar a interface e a bandeja.
        if self.biblioteca_mudou && Self::passou(self.ultimo_aviso, agora, aviso_biblioteca()) {
            self.ultimo_aviso = Some(agora);
            self.biblioteca_mudou = false;
            self.sink.library_changed();
        }
        self.publicar();
    }

    fn fazer_backups(&mut self, vencidos: Vec<Due>, agora: DateTime<Utc>) {
        // O lote inteiro conta como "ocupado": o "Sair" e o fim da sessão do
        // Windows esperam o lote todo, não só o backup da vez.
        let _lote = self.svc().busy().guard();
        let mut feitos = Vec::new();
        let mut fechou = false;
        self.ocupado = true;
        self.publicar();
        for d in vencidos {
            if !self.svc().can_back_up(&d.game_id) {
                self.sched.forget(&d.game_id);
                continue;
            }
            let r = {
                let _prioridade = PrioridadeBaixa::nova();
                self.svc().auto_backup(&d.game_id)
            };
            match r {
                Ok(v) => {
                    self.sched.backed_up(&d.game_id, agora);
                    self.biblioteca_mudou = true;
                    if v.outcome == BackupOutcomeView::Feito {
                        fechou |= d.reason == DueReason::Closed;
                        self.ultimo_backup = Some(LastBackupView { title: v.title.clone(), at: agora.to_rfc3339() });
                        feitos.push(v);
                    }
                }
                Err(e) if e.kind == ErrorKind::CofreIndisponivel => {
                    self.cofre_ok = false;
                    self.biblioteca_mudou = true;
                    break;
                }
                // Arquivo travado, save que não estabilizou…: já foi para a Atividade.
                Err(_) => self.sched.failed(&d.game_id, agora),
            }
        }
        self.ocupado = false;
        if !feitos.is_empty() {
            self.sink.backed_up(&feitos, fechou);
        }
    }

    fn atualizar_raizes(&mut self, lista: Vec<(String, Vec<PathBuf>)>) {
        self.vigiados = lista.len();
        let mut raizes: Vec<PathBuf> = lista.into_iter().flat_map(|(_, r)| r).collect();
        raizes.sort();
        raizes.dedup();
        if raizes != self.raizes {
            self.raizes = raizes;
            self.versao_raizes += 1;
        }
    }

    fn estado(&self) -> AutoState {
        if !self.pronto {
            AutoState::Parado
        } else if !self.svc().auto_backup_enabled() {
            AutoState::Desligado
        } else if !self.cofre_ok {
            AutoState::CofreFora
        } else if self.pausa != Pausa::Nenhuma {
            AutoState::Pausado
        } else {
            AutoState::Ativo
        }
    }

    pub fn status(&self) -> AutoStatusView {
        AutoStatusView {
            state: self.estado(),
            paused_until: match self.pausa {
                Pausa::Ate(t) => Some(t.to_rfc3339()),
                _ => None,
            },
            watching: self.vigiados,
            busy: self.ocupado,
            last_backup: self.ultimo_backup.clone(),
        }
    }

    /// Conta a situação para fora só quando ela muda.
    fn publicar(&mut self) {
        let s = self.status();
        if self.ultimo_status.as_ref() != Some(&s) {
            self.sink.status(&s);
            self.ultimo_status = Some(s);
        }
    }

    /// Quanto dormir até o próximo passo (entre 1 s e 5 s).
    pub fn next_wake(&self) -> std::time::Duration {
        let agora = self.clock.now();
        let mut ate = processos();
        if self.estado() == AutoState::Ativo {
            if let Some(t) = self.sched.next_due(agora) {
                ate = ate.min(t - agora);
            }
        }
        if let Pausa::Ate(t) = self.pausa {
            ate = ate.min(t - agora);
        }
        ate.max(Duration::seconds(1)).to_std().unwrap_or(std::time::Duration::from_secs(1))
    }

    pub fn watched_roots(&self) -> Vec<PathBuf> {
        self.raizes.clone()
    }

    /// Muda quando as pastas vigiadas mudam (o vigia é refeito).
    pub fn roots_version(&self) -> u64 {
        self.versao_raizes
    }

    pub fn quitting(&self) -> bool {
        self.sair
    }
}

/// Roda o trabalhador numa thread própria até chegar `Command::Quit` (ou o
/// canal fechar). `tx` é o mesmo canal, para o vigia de pastas mandar as mudanças.
pub fn spawn(mut worker: AutoWorker, rx: Receiver<Command>, tx: Sender<Command>) -> JoinHandle<()> {
    std::thread::spawn(move || {
        let mut vigia = match Watcher::new(tx) {
            Ok(v) => Some(v),
            Err(e) => {
                worker.svc().log_error(&format!("vigia de pastas não começou (fica só a varredura de 10 min): {e}"));
                None
            }
        };
        let mut versao = u64::MAX;
        loop {
            match rx.recv_timeout(worker.next_wake()) {
                Ok(cmd) => {
                    worker.handle(cmd);
                    // Uma enxurrada de eventos vira um passo só.
                    while let Ok(c) = rx.try_recv() {
                        worker.handle(c);
                    }
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => break,
            }
            if worker.quitting() {
                break;
            }
            worker.tick();
            if worker.roots_version() != versao {
                versao = worker.roots_version();
                if let Some(v) = vigia.as_mut() {
                    for (pasta, e) in v.rewatch(&worker.watched_roots()) {
                        worker.svc().log_error(&format!("vigia de pastas: {}: {e}", pasta.display()));
                    }
                }
            }
        }
    })
}

/// Vigia de pastas (eventos do sistema de arquivos): cada mudança vira
/// `Command::Changed(caminho)` no canal do trabalhador.
pub struct Watcher {
    inner: notify::RecommendedWatcher,
    vigiando: Vec<PathBuf>,
}

impl Watcher {
    pub fn new(tx: Sender<Command>) -> notify::Result<Watcher> {
        let inner = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            let Ok(ev) = res else { return };
            // Abrir e ler arquivos não muda save.
            if matches!(ev.kind, notify::EventKind::Access(_)) {
                return;
            }
            for p in ev.paths {
                let _ = tx.send(Command::Changed(p));
            }
        })?;
        Ok(Watcher { inner, vigiando: Vec::new() })
    }

    /// Troca as pastas vigiadas. Pasta que ainda não existe fica para a varredura
    /// leve; devolve as que existem mas não deu para vigiar (e o erro).
    pub fn rewatch(&mut self, roots: &[PathBuf]) -> Vec<(PathBuf, notify::Error)> {
        for p in self.vigiando.drain(..) {
            let _ = self.inner.unwatch(&p);
        }
        let mut falhas = Vec::new();
        for r in roots.iter().filter(|r| r.is_dir()) {
            match self.inner.watch(r, notify::RecursiveMode::Recursive) {
                Ok(()) => self.vigiando.push(r.clone()),
                Err(e) => falhas.push((r.clone(), e)),
            }
        }
        falhas
    }
}
