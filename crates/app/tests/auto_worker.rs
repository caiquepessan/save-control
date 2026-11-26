//! O trabalhador do backup automático, com relógio, processos e saída falsos e
//! o serviço de verdade num mundo temporário.
mod common;

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Duration, TimeZone, Utc};
use common::{Mundo, mundo, pronto, servico, write};
use savecontrol_app::activity::ActivityKind;
use savecontrol_app::auto_worker::{AutoWorker, Clock, Command, Sink, Watcher};
use savecontrol_app::dto::*;
use savecontrol_app::service::{AppService, BusyFlag};
use savecontrol_core::error::{Error, Result};
use savecontrol_core::manifest::store::{FetchResult, Fetcher};
use savecontrol_core::process::{ProcessProbe, RunningProcess};

fn t0() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 25, 20, 0, 0).unwrap()
}

#[derive(Clone)]
struct RelogioFalso(Arc<Mutex<DateTime<Utc>>>);

impl RelogioFalso {
    fn avancar(&self, d: Duration) {
        let mut t = self.0.lock().unwrap();
        *t += d;
    }
}

impl Clock for RelogioFalso {
    fn now(&self) -> DateTime<Utc> {
        *self.0.lock().unwrap()
    }
}

#[derive(Clone, Default)]
struct ProcessosFalsos(Arc<Mutex<Vec<String>>>);

impl ProcessosFalsos {
    fn abertos(&self, nomes: &[&str]) {
        *self.0.lock().unwrap() = nomes.iter().map(|n| n.to_string()).collect();
    }
}

impl ProcessProbe for ProcessosFalsos {
    fn running(&self) -> Vec<RunningProcess> {
        self.0.lock().unwrap().iter().map(|n| RunningProcess { name: n.clone(), exe_path: None }).collect()
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Evento {
    Status(AutoState),
    Biblioteca,
    Backup(Vec<String>, bool),
    Aviso(String),
}

#[derive(Clone, Default)]
struct SaidaFalsa(Arc<Mutex<Vec<Evento>>>);

impl SaidaFalsa {
    fn eventos(&self) -> Vec<Evento> {
        self.0.lock().unwrap().clone()
    }

    fn backups(&self) -> Vec<(Vec<String>, bool)> {
        self.eventos()
            .into_iter()
            .filter_map(|e| match e {
                Evento::Backup(ids, fechou) => Some((ids, fechou)),
                _ => None,
            })
            .collect()
    }

    fn ultimo_estado(&self) -> Option<AutoState> {
        self.eventos().into_iter().rev().find_map(|e| match e {
            Evento::Status(s) => Some(s),
            _ => None,
        })
    }
}

impl Sink for SaidaFalsa {
    fn status(&self, s: &AutoStatusView) {
        self.0.lock().unwrap().push(Evento::Status(s.state));
    }

    fn library_changed(&self) {
        self.0.lock().unwrap().push(Evento::Biblioteca);
    }

    fn backed_up(&self, items: &[AutoBackupView], game_closed: bool) {
        self.0.lock().unwrap().push(Evento::Backup(items.iter().map(|i| i.id.clone()).collect(), game_closed));
    }

    fn notice(&self, text: &str) {
        self.0.lock().unwrap().push(Evento::Aviso(text.to_string()));
    }
}

/// Sem internet: a base de jogos de PC nunca baixa nos testes.
struct SemRede;

impl Fetcher for SemRede {
    fn fetch(&self, _url: &str, _etag: Option<&str>) -> Result<FetchResult> {
        Err(Error::Http("sem rede nos testes".into()))
    }
}

struct Cena {
    w: Mundo,
    svc: Arc<Mutex<AppService>>,
    relogio: RelogioFalso,
    processos: ProcessosFalsos,
    saida: SaidaFalsa,
}

fn cena() -> Cena {
    let w = mundo();
    let svc = Arc::new(Mutex::new(pronto(&w)));
    Cena {
        w,
        svc,
        relogio: RelogioFalso(Arc::new(Mutex::new(t0()))),
        processos: ProcessosFalsos::default(),
        saida: SaidaFalsa::default(),
    }
}

impl Cena {
    /// Jogo à mão com um save; devolve (id, arquivo do save).
    fn jogo(&self, nome: &str, processos: &[&str]) -> (String, PathBuf) {
        let dir = self.w.sys.app_data.join(nome);
        write(&dir.join("slot.sav"), nome.as_bytes());
        let g = self
            .svc
            .lock()
            .unwrap()
            .add_game(AddGameRequest {
                name: nome.into(),
                save_dirs: vec![dir.display().to_string()],
                processes: processos.iter().map(|p| p.to_string()).collect(),
                ..Default::default()
            })
            .unwrap();
        (g.id, dir.join("slot.sav"))
    }

    fn trabalhador(&self) -> AutoWorker {
        AutoWorker::new(
            self.svc.clone(),
            Box::new(self.processos.clone()),
            Box::new(self.saida.clone()),
            Box::new(self.relogio.clone()),
            Box::new(SemRede),
        )
    }

    fn automaticos(&self, id: &str) -> usize {
        let h = self.svc.lock().unwrap().history(id).unwrap();
        h.iter().filter(|s| s.kind == SnapshotKindView::Auto).count()
    }

    /// Avança o relógio e dá um passo no trabalhador.
    fn passo(&self, t: &mut AutoWorker, d: Duration) {
        self.relogio.avancar(d);
        t.tick();
    }
}

/// Grava um save novo e avisa o trabalhador, como o vigia faria.
fn mudar(t: &mut AutoWorker, save: &Path, conteudo: &str) {
    write(save, conteudo.as_bytes());
    t.handle(Command::Changed(save.to_path_buf()));
}

#[test]
fn fecha_o_jogo_e_o_backup_sai_10s_depois() {
    let c = cena();
    let (id, save) = c.jogo("JogoA", &["JogoA.exe"]);
    c.processos.abertos(&["JogoA.exe"]);
    let mut t = c.trabalhador();
    t.tick();
    mudar(&mut t, &save, "progresso da partida");
    c.passo(&mut t, Duration::minutes(2));
    assert_eq!(c.automaticos(&id), 0, "fez backup com o jogo aberto");

    c.processos.abertos(&[]);
    c.passo(&mut t, Duration::seconds(5));
    c.passo(&mut t, Duration::seconds(9));
    assert_eq!(c.automaticos(&id), 0, "não esperou os 10 s depois de fechar");
    c.passo(&mut t, Duration::seconds(1));
    assert_eq!(c.automaticos(&id), 1);
    assert_eq!(c.saida.backups(), [(vec![id], true)]);
}

#[test]
fn sessao_longa_um_backup_a_cada_15_min() {
    let c = cena();
    let (id, save) = c.jogo("JogoA", &["JogoA.exe"]);
    c.processos.abertos(&["JogoA.exe"]);
    let mut t = c.trabalhador();
    t.tick();
    for m in 0..=31 {
        mudar(&mut t, &save, &format!("autosave {m}"));
        c.passo(&mut t, Duration::minutes(1));
    }
    assert_eq!(c.automaticos(&id), 2);
    assert!(c.saida.backups().iter().all(|(_, fechou)| !fechou), "sessão longa não é \"fechou o jogo\"");
}

#[test]
fn sem_processo_conhecido_espera_60s() {
    let c = cena();
    let (id, save) = c.jogo("JogoSemProcesso", &[]);
    let mut t = c.trabalhador();
    t.tick();
    mudar(&mut t, &save, "mudou");
    c.passo(&mut t, Duration::seconds(59));
    assert_eq!(c.automaticos(&id), 0);
    c.passo(&mut t, Duration::seconds(1));
    assert_eq!(c.automaticos(&id), 1);
    assert_eq!(c.saida.backups(), [(vec![id], false)]);
}

#[test]
fn pausado_nao_faz_backup_e_retoma() {
    let c = cena();
    let (id, save) = c.jogo("JogoA", &[]);
    let mut t = c.trabalhador();
    t.tick();
    t.handle(Command::Pause(Some(std::time::Duration::from_secs(30 * 60))));
    mudar(&mut t, &save, "mudou");
    c.passo(&mut t, Duration::seconds(61));
    assert_eq!(c.automaticos(&id), 0);
    assert_eq!(c.saida.ultimo_estado(), Some(AutoState::Pausado));
    c.passo(&mut t, Duration::minutes(30));
    assert_eq!(c.automaticos(&id), 1, "não retomou depois da pausa");
    assert_eq!(c.saida.ultimo_estado(), Some(AutoState::Ativo));
}

#[test]
fn pausa_ate_reabrir_nao_acaba_sozinha() {
    let c = cena();
    let (id, save) = c.jogo("JogoA", &[]);
    let mut t = c.trabalhador();
    t.tick();
    t.handle(Command::Pause(None));
    mudar(&mut t, &save, "mudou");
    for _ in 0..36 {
        c.passo(&mut t, Duration::minutes(5));
    }
    assert_eq!(c.automaticos(&id), 0);
    t.handle(Command::Resume);
    c.passo(&mut t, Duration::seconds(1));
    assert_eq!(c.automaticos(&id), 1);
}

/// Os `.zip` gravados fora de `cofre` (que não deveriam existir).
fn zips_fora(raiz: &Path, cofre: &Path) -> Vec<PathBuf> {
    let mut achados = Vec::new();
    let mut pilha = vec![raiz.to_path_buf()];
    while let Some(d) = pilha.pop() {
        for e in std::fs::read_dir(&d).unwrap().flatten() {
            let p = e.path();
            if p.starts_with(cofre) {
                continue;
            }
            if p.is_dir() {
                pilha.push(p);
            } else if p.extension().is_some_and(|x| x == "zip" || x == "parcial") {
                achados.push(p);
            }
        }
    }
    achados
}

#[test]
fn cofre_fora_pausa_e_volta_sozinho() {
    let c = cena();
    let (id, save) = c.jogo("JogoA", &[]);
    let mut t = c.trabalhador();
    t.tick();
    let cofre = c.w.exe.join("cofre");
    let fora = c.w.t.path().join("hd-desligado");
    std::fs::rename(&cofre, &fora).unwrap();
    c.passo(&mut t, Duration::seconds(30));
    assert_eq!(c.saida.ultimo_estado(), Some(AutoState::CofreFora));

    mudar(&mut t, &save, "jogou com o HD desligado");
    for _ in 0..10 {
        c.passo(&mut t, Duration::seconds(30));
    }
    assert!(!cofre.exists(), "o cofre foi recriado no lugar do HD desligado");
    assert!(zips_fora(c.w.t.path(), &fora).is_empty(), "gravou backup fora do cofre");

    std::fs::rename(&fora, &cofre).unwrap();
    c.passo(&mut t, Duration::seconds(30));
    c.passo(&mut t, Duration::seconds(1));
    assert_eq!(c.saida.ultimo_estado(), Some(AutoState::Ativo));
    assert_eq!(c.automaticos(&id), 1, "o backup pendente não aconteceu quando o cofre voltou");
}

#[test]
fn automatico_desligado_no_jogo_e_ignorado() {
    let c = cena();
    let (id, save) = c.jogo("JogoA", &[]);
    c.svc.lock().unwrap().set_game_auto(&id, false).unwrap();
    let mut t = c.trabalhador();
    t.tick();
    mudar(&mut t, &save, "mudou");
    c.passo(&mut t, Duration::minutes(5));
    assert_eq!(c.automaticos(&id), 0);
}

#[test]
fn automatico_desligado_geral_nao_faz_nada() {
    let c = cena();
    let (id, save) = c.jogo("JogoA", &[]);
    let mut t = c.trabalhador();
    t.tick();
    c.svc.lock().unwrap().set_auto_backup(false).unwrap();
    t.handle(Command::ConfigChanged);
    mudar(&mut t, &save, "mudou");
    c.passo(&mut t, Duration::minutes(5));
    assert_eq!(c.automaticos(&id), 0);
    assert_eq!(c.saida.ultimo_estado(), Some(AutoState::Desligado));
}

#[test]
fn varios_jogos_numa_notificacao_so() {
    let c = cena();
    let (a, save_a) = c.jogo("JogoA", &["A.exe"]);
    let (b, save_b) = c.jogo("JogoB", &["B.exe"]);
    c.processos.abertos(&["A.exe", "B.exe"]);
    let mut t = c.trabalhador();
    t.tick();
    mudar(&mut t, &save_a, "a mudou");
    mudar(&mut t, &save_b, "b mudou");
    c.passo(&mut t, Duration::minutes(1));
    c.processos.abertos(&[]);
    c.passo(&mut t, Duration::seconds(5));
    c.passo(&mut t, Duration::seconds(10));
    let lotes = c.saida.backups();
    assert_eq!(lotes.len(), 1, "{lotes:?}");
    let (mut ids, fechou) = lotes[0].clone();
    ids.sort();
    let mut esperado = vec![a, b];
    esperado.sort();
    assert_eq!((ids, fechou), (esperado, true));
}

#[cfg(windows)]
#[test]
fn arquivo_travado_registra_e_tenta_de_novo() {
    use std::os::windows::fs::OpenOptionsExt;
    let c = cena();
    let (id, save) = c.jogo("JogoA", &[]);
    let mut t = c.trabalhador();
    t.tick();
    mudar(&mut t, &save, "salvando…");
    let trava = std::fs::OpenOptions::new().read(true).write(true).share_mode(0).open(&save).unwrap();
    c.passo(&mut t, Duration::seconds(60));
    assert_eq!(c.automaticos(&id), 0);
    let atividade = c.svc.lock().unwrap().activity(10);
    assert_eq!(atividade[0].kind, ActivityKind::Error, "{atividade:?}");

    drop(trava);
    c.passo(&mut t, Duration::minutes(4));
    assert_eq!(c.automaticos(&id), 0, "tentou de novo antes dos 5 min");
    c.passo(&mut t, Duration::minutes(1));
    assert_eq!(c.automaticos(&id), 1);
}

#[test]
fn mudanca_vira_save_novo_na_hora() {
    let c = cena();
    let (id, save) = c.jogo("JogoA", &["JogoA.exe"]);
    c.svc.lock().unwrap().backup_game(&id, None).unwrap();
    let mut t = c.trabalhador();
    t.tick();
    let status = |c: &Cena| c.svc.lock().unwrap().library().unwrap().games.into_iter().find(|g| g.id == id).unwrap().status;
    assert_eq!(status(&c), StatusView::Protegido);
    mudar(&mut t, &save, "progresso novo");
    assert_eq!(status(&c), StatusView::SaveNovo);
    c.passo(&mut t, Duration::seconds(3));
    assert!(c.saida.eventos().contains(&Evento::Biblioteca), "a interface não foi avisada");
}

#[test]
fn tempos_novos_valem_depois_de_config_changed() {
    let c = cena();
    let (id, save) = c.jogo("JogoA", &[]);
    let mut t = c.trabalhador();
    t.tick();
    c.svc
        .lock()
        .unwrap()
        .set_auto_times(AutoTimesView { quiet_secs: 20, after_close_secs: 10, while_running_mins: 15 })
        .unwrap();
    t.handle(Command::ConfigChanged);
    mudar(&mut t, &save, "mudou");
    c.passo(&mut t, Duration::seconds(19));
    assert_eq!(c.automaticos(&id), 0);
    c.passo(&mut t, Duration::seconds(1));
    assert_eq!(c.automaticos(&id), 1);
}

#[test]
fn vigia_avisa_quando_o_save_muda() {
    let t = tempfile::tempdir().unwrap();
    let pasta = t.path().join("saves");
    std::fs::create_dir_all(&pasta).unwrap();
    let (tx, rx) = mpsc::channel();
    let mut vigia = Watcher::new(tx).unwrap();
    vigia.rewatch(std::slice::from_ref(&pasta));
    write(&pasta.join("slot.sav"), b"novo");
    let mut chegou = false;
    while let Ok(cmd) = rx.recv_timeout(std::time::Duration::from_secs(5)) {
        if let Command::Changed(p) = cmd {
            if p.starts_with(&pasta) {
                chegou = true;
                break;
            }
        }
    }
    assert!(chegou, "o vigia não avisou a mudança no save");
}

impl Cena {
    /// Outra sessão do app no mesmo PC e no mesmo cofre (o app fechou e abriu de novo).
    fn nova_sessao(&self) -> Arc<Mutex<AppService>> {
        Arc::new(Mutex::new(servico(&self.w, BusyFlag::default())))
    }

    fn trabalhador_de(&self, svc: &Arc<Mutex<AppService>>) -> AutoWorker {
        AutoWorker::new(
            svc.clone(),
            Box::new(self.processos.clone()),
            Box::new(self.saida.clone()),
            Box::new(self.relogio.clone()),
            Box::new(SemRede),
        )
    }
}

fn automaticos_de(svc: &Arc<Mutex<AppService>>, id: &str) -> usize {
    let h = svc.lock().unwrap().history(id).unwrap();
    h.iter().filter(|s| s.kind == SnapshotKindView::Auto).count()
}

/// A janela escaneia ao abrir e acha um save que mudou com o app fechado: ele
/// entra na fila do automático (o status "Save novo" promete isso).
#[test]
fn save_novo_achado_no_escaneamento_entra_na_fila() {
    let c = cena();
    let (id, save) = c.jogo("JogoA", &[]);
    c.svc.lock().unwrap().backup_game(&id, None).unwrap();
    let mut t = c.trabalhador();
    t.tick();
    write(&save, b"jogou com o app fechado");
    c.svc.lock().unwrap().scan().unwrap();
    t.handle(Command::ConfigChanged);
    c.passo(&mut t, Duration::seconds(59));
    assert_eq!(c.automaticos(&id), 0, "não esperou os 60 s sem mudanças");
    c.passo(&mut t, Duration::seconds(1));
    assert_eq!(c.automaticos(&id), 1, "o save novo achado no escaneamento ficou sem backup");
}

/// Aberto pela bandeja, ninguém escaneia ao abrir: o app pede um escaneamento
/// (ScanNow) e o save que mudou com o PC desligado entra na fila.
#[test]
fn inicio_pela_bandeja_escaneia_e_agenda_o_save_novo() {
    let c = cena();
    let (id, save) = c.jogo("JogoA", &[]);
    c.svc.lock().unwrap().backup_game(&id, None).unwrap();
    write(&save, b"jogou antes de desligar o PC");
    let svc = c.nova_sessao();
    let mut t = c.trabalhador_de(&svc);
    t.handle(Command::ScanNow);
    t.tick();
    assert_eq!(t.status().watching, 1, "não vigia nada depois de escanear");
    c.passo(&mut t, Duration::seconds(60));
    assert_eq!(automaticos_de(&svc, &id), 1, "o save novo de antes de abrir ficou sem backup");
}

/// O app abriu com o HD do cofre desligado; a pessoa jogou e religou o HD: o
/// save novo entra na fila quando o cofre volta ("continua de onde parou").
#[test]
fn cofre_que_volta_depois_de_abrir_agenda_os_saves_novos() {
    let c = cena();
    let (id, save) = c.jogo("JogoA", &[]);
    c.svc.lock().unwrap().backup_game(&id, None).unwrap();
    let cofre = c.w.exe.join("cofre");
    let fora = c.w.t.path().join("hd-desligado");
    std::fs::rename(&cofre, &fora).unwrap();
    let svc = c.nova_sessao();
    let mut t = c.trabalhador_de(&svc);
    t.tick();
    assert_eq!(c.saida.ultimo_estado(), Some(AutoState::Parado));
    write(&save, b"jogou com o HD desligado");
    std::fs::rename(&fora, &cofre).unwrap();
    c.passo(&mut t, Duration::seconds(30));
    assert_eq!(c.saida.ultimo_estado(), Some(AutoState::Ativo));
    c.passo(&mut t, Duration::seconds(60));
    assert_eq!(automaticos_de(&svc, &id), 1, "o save novo não entrou na fila quando o cofre voltou");
}

/// Um lote de backups conta como "ocupado" do começo ao fim: o "Sair" e o
/// desligamento do Windows esperam o lote inteiro, não só o backup da vez.
#[test]
fn o_lote_inteiro_conta_como_ocupado() {
    #[derive(Clone)]
    struct OlhaOcupado {
        ocupado: BusyFlag,
        vistos: Arc<Mutex<Vec<(&'static str, bool)>>>,
    }
    impl Sink for OlhaOcupado {
        fn status(&self, s: &AutoStatusView) {
            if s.busy {
                self.vistos.lock().unwrap().push(("começo", self.ocupado.is_busy()));
            }
        }
        fn library_changed(&self) {}
        fn backed_up(&self, _items: &[AutoBackupView], _game_closed: bool) {
            self.vistos.lock().unwrap().push(("fim", self.ocupado.is_busy()));
        }
        fn notice(&self, _text: &str) {}
    }

    let c = cena();
    let (a, save_a) = c.jogo("JogoA", &[]);
    let (b, save_b) = c.jogo("JogoB", &[]);
    let olho = OlhaOcupado { ocupado: c.svc.lock().unwrap().busy(), vistos: Arc::default() };
    let mut t = AutoWorker::new(
        c.svc.clone(),
        Box::new(c.processos.clone()),
        Box::new(olho.clone()),
        Box::new(c.relogio.clone()),
        Box::new(SemRede),
    );
    t.tick();
    mudar(&mut t, &save_a, "a");
    mudar(&mut t, &save_b, "b");
    c.passo(&mut t, Duration::seconds(60));
    assert_eq!(c.automaticos(&a) + c.automaticos(&b), 2);
    assert_eq!(*olho.vistos.lock().unwrap(), [("começo", true), ("fim", true)]);
}

/// O trabalhador não engole falhas: a base de jogos que não baixou vai para o log.
#[test]
fn falha_da_base_de_jogos_vai_para_o_log() {
    let c = cena();
    let mut t = c.trabalhador();
    t.handle(Command::ScanNow);
    t.tick();
    let arquivo = c.svc.lock().unwrap().paths().logs_dir().join("save-control.log");
    let log = common::ler(&arquivo);
    assert!(log.contains("ERRO") && log.contains("base de jogos"), "{log}");
}
