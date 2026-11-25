//! App Tauri do Save Control: janela, bandeja, notificações, o trabalhador do
//! backup automático e a ponte entre a interface e o `AppService` (toda a
//! regra fica em `savecontrol-app` e `savecontrol-core`).

mod commands;
mod toast;
mod tray;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use savecontrol_app::auto_worker::{self, AutoWorker, Command, Sink, SystemClock};
use savecontrol_app::bandeja;
use savecontrol_app::config::WindowState;
use savecontrol_app::dto::{AutoBackupView, AutoState, AutoStatusView};
use savecontrol_app::paths::resolve_paths;
use savecontrol_app::registro::{Integracao, WinRegistro, unidade_removivel};
use savecontrol_app::service::{AppService, BusyFlag};
use savecontrol_core::env::SystemPaths;
use savecontrol_core::manifest::store::HttpFetcher;
use savecontrol_core::process::SystemProbe;
use savecontrol_core::registry::{RegistryReader, WinRegistry};
use serde::Serialize;
use tauri::window::Color;
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, RunEvent, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder, WindowEvent,
};

/// "Sair" espera o backup em andamento por até 10 min.
const ESPERA_AO_SAIR: Duration = Duration::from_secs(10 * 60);

/// Estado compartilhado pelos comandos, pela bandeja e pelo trabalhador.
pub struct Shared {
    pub svc: Arc<Mutex<AppService>>,
    pub busy: BusyFlag,
    /// Já existe uma thread baixando capas da Steam.
    pub covers_running: Arc<AtomicBool>,
    last_window_save: Mutex<Option<Instant>>,
    /// Última geometria da janela, para recriá-la no mesmo lugar.
    janela: Mutex<Option<WindowState>>,
    webview_dir: PathBuf,
    criando_janela: AtomicBool,
    /// Canal do trabalhador do backup automático.
    worker: Sender<Command>,
    worker_rx: Mutex<Option<Receiver<Command>>>,
    worker_thread: Mutex<Option<JoinHandle<()>>>,
    pub integracao: Mutex<Integracao>,
    /// Última situação publicada pelo trabalhador.
    pub status: Mutex<AutoStatusView>,
    pub fechar_para_bandeja: AtomicBool,
    pub notificacoes: AtomicBool,
    saindo: AtomicBool,
    /// Jogo que a janela recém-criada abre (clique na notificação).
    pub jogo_pendente: Mutex<Option<String>>,
}

impl Shared {
    /// Manda um comando ao trabalhador do backup automático.
    pub fn mandar(&self, c: Command) {
        let _ = self.worker.send(c);
    }
}

pub(crate) fn travar<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// Mensagem nativa do Windows, para quando a interface não pode abrir.
pub fn avisar(texto: &str) {
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{MB_ICONWARNING, MB_OK, MessageBoxW};
        let largo = |s: &str| s.encode_utf16().chain(std::iter::once(0)).collect::<Vec<u16>>();
        let (texto, titulo) = (largo(texto), largo("Save Control"));
        unsafe {
            MessageBoxW(std::ptr::null_mut(), texto.as_ptr(), titulo.as_ptr(), MB_OK | MB_ICONWARNING);
        }
    }
    #[cfg(not(windows))]
    eprintln!("{texto}");
}

fn registro() -> Box<dyn RegistryReader + Send> {
    Box::new(WinRegistry)
}

/// Posição e tamanho atuais. Maximizada, guarda só o "maximizado" e mantém o
/// tamanho de antes, para voltar ao normal no lugar certo.
fn window_state(win: &WebviewWindow, previous: Option<WindowState>) -> Option<WindowState> {
    let maximized = win.is_maximized().ok()?;
    if maximized {
        return previous.map(|p| WindowState { maximized: true, ..p });
    }
    let pos = win.outer_position().ok()?;
    let size = win.inner_size().ok()?;
    if win.is_minimized().unwrap_or(false) || size.width == 0 {
        return None;
    }
    Some(WindowState { x: pos.x, y: pos.y, width: size.width, height: size.height, maximized: false })
}

/// Guarda a geometria atual na memória.
fn lembrar_janela(win: &WebviewWindow, shared: &Shared) -> Option<WindowState> {
    let mut guardada = travar(&shared.janela);
    let ws = window_state(win, *guardada)?;
    *guardada = Some(ws);
    Some(ws)
}

/// Grava a geometria da janela sem travar a interface (se o serviço estiver
/// ocupado, deixa para a próxima).
fn save_window(win: &WebviewWindow, shared: &Shared, throttle: bool) {
    let Some(ws) = lembrar_janela(win, shared) else { return };
    if throttle {
        let mut last = travar(&shared.last_window_save);
        if last.is_some_and(|t| t.elapsed() < Duration::from_secs(2)) {
            return;
        }
        *last = Some(Instant::now());
    }
    let Ok(mut svc) = shared.svc.try_lock() else { return };
    let _ = svc.save_window(ws);
}

/// Grava a geometria guardada numa thread própria (o serviço pode estar ocupado).
fn gravar_janela_depois(shared: &Shared) {
    let Some(ws) = *travar(&shared.janela) else { return };
    let svc = shared.svc.clone();
    std::thread::spawn(move || {
        let _ = travar(&svc).save_window(ws);
    });
}

/// Aplica a geometria salva se ela cair dentro de algum monitor; senão, centraliza.
fn restore_window(win: &WebviewWindow, saved: Option<WindowState>) {
    let monitors = win.available_monitors().unwrap_or_default();
    let visible = saved.filter(|w| {
        monitors.iter().any(|m| {
            let (p, s) = (m.position(), m.size());
            let (cx, cy) = (w.x + 80, w.y + 20);
            cx >= p.x && cy >= p.y && cx < p.x + s.width as i32 && cy < p.y + s.height as i32
        })
    });
    match visible {
        Some(w) => {
            let _ = win.set_size(PhysicalSize::new(w.width.max(1100), w.height.max(700)));
            let _ = win.set_position(PhysicalPosition::new(w.x, w.y));
            if w.maximized {
                let _ = win.maximize();
            }
        }
        None => {
            let _ = win.center();
        }
    }
}

fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Cria a janela principal, no lugar onde ela estava.
fn criar_janela(app: &AppHandle) -> tauri::Result<()> {
    let shared = app.state::<Shared>();
    let salva = *travar(&shared.janela);
    let win = WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
        .title("Save Control")
        .decorations(false)
        .inner_size(1280.0, 800.0)
        .min_inner_size(1100.0, 700.0)
        .data_directory(shared.webview_dir.clone())
        // Fundo escuro desde o primeiro quadro: sem clarão branco ao abrir.
        .background_color(Color(10, 13, 19, 255))
        .visible(false)
        .build()?;
    restore_window(&win, salva);
    win.show()?;
    let _ = win.set_focus();
    Ok(())
}

/// Mostra a janela. Fechada para a bandeja, ela é criada de novo, numa thread
/// própria: criar janela num evento da thread principal trava no Windows.
pub fn abrir_janela(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
        return;
    }
    if app.state::<Shared>().criando_janela.swap(true, Ordering::SeqCst) {
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        if let Err(e) = criar_janela(&app) {
            avisar(&format!("O Save Control não conseguiu abrir a janela: {e}"));
        }
        app.state::<Shared>().criando_janela.store(false, Ordering::SeqCst);
    });
}

#[derive(Clone, Serialize)]
struct AbrirJogo {
    id: String,
}

/// Clique na notificação: abre a janela no histórico do jogo. A janela
/// recém-criada pergunta o jogo (`jogo_para_abrir`) quando fica pronta.
pub fn abrir_jogo(app: &AppHandle, id: &str) {
    if app.get_webview_window("main").is_some() {
        abrir_janela(app);
        let _ = app.emit("abrir-jogo", AbrirJogo { id: id.to_string() });
    } else {
        *travar(&app.state::<Shared>().jogo_pendente) = Some(id.to_string());
        abrir_janela(app);
    }
}

/// "Sair" (bandeja, ou fechar a janela com "fechar para a bandeja" desligado):
/// para o trabalhador, espera o backup em andamento terminar (até 10 min),
/// grava a geometria da janela e fecha o app.
pub fn sair(app: &AppHandle) {
    let shared = app.state::<Shared>();
    if shared.saindo.swap(true, Ordering::SeqCst) {
        return;
    }
    shared.mandar(Command::Quit);
    if shared.busy.is_busy() {
        tray::dica(app, bandeja::TERMINANDO);
        let _ = app.emit("fechar-bloqueado", ());
    }
    let app = app.clone();
    std::thread::spawn(move || {
        let shared = app.state::<Shared>();
        let limite = Instant::now() + ESPERA_AO_SAIR;
        let trabalhador = travar(&shared.worker_thread).take();
        if let Some(t) = trabalhador {
            while !t.is_finished() && Instant::now() < limite {
                std::thread::sleep(Duration::from_millis(50));
            }
        }
        shared.busy.wait_idle(limite.saturating_duration_since(Instant::now()));
        if let Some(ws) = *travar(&shared.janela) {
            if let Ok(mut svc) = shared.svc.try_lock() {
                let _ = svc.save_window(ws);
            }
        }
        app.exit(0);
    });
}

#[derive(Clone, Serialize)]
struct BackupAutomatico {
    itens: Vec<AutoBackupView>,
}

/// O que o trabalhador conta para fora: bandeja, janela e notificação.
struct TauriSink(AppHandle);

impl Sink for TauriSink {
    fn status(&self, s: &AutoStatusView) {
        let shared = self.0.state::<Shared>();
        *travar(&shared.status) = s.clone();
        if !shared.saindo.load(Ordering::SeqCst) {
            tray::atualizar(&self.0, s);
        }
        let _ = self.0.emit("situacao", s);
    }

    fn library_changed(&self) {
        let _ = self.0.emit("biblioteca-mudou", ());
    }

    fn backed_up(&self, items: &[AutoBackupView], game_closed: bool) {
        let shared = self.0.state::<Shared>();
        let notificou = game_closed && shared.notificacoes.load(Ordering::SeqCst) && toast::mostrar(&self.0, items);
        if !notificou {
            let _ = self.0.emit("backup-automatico", BackupAutomatico { itens: items.to_vec() });
        }
    }

    fn notice(&self, text: &str) {
        let _ = self.0.emit("aviso", text);
    }
}

pub fn run() {
    let sys = match SystemPaths::detect() {
        Ok(s) => s,
        Err(e) => {
            avisar(&format!("Não foi possível achar as pastas do Windows: {e}"));
            return;
        }
    };
    let paths = resolve_paths(&exe_dir(), &sys.local_app_data);
    let data_dir = paths.data_dir.clone();
    let busy = BusyFlag::default();
    let svc = AppService::new(paths, sys, Box::new(registro), busy.clone());
    let exe = std::env::current_exe().unwrap_or_else(|_| exe_dir().join("SaveControl.exe"));
    let (tx, rx) = std::sync::mpsc::channel();
    let shared = Shared {
        janela: Mutex::new(svc.window()),
        webview_dir: svc.paths().webview_dir(),
        fechar_para_bandeja: AtomicBool::new(svc.close_to_tray()),
        notificacoes: AtomicBool::new(svc.notifications_enabled()),
        svc: Arc::new(Mutex::new(svc)),
        busy,
        covers_running: Arc::new(AtomicBool::new(false)),
        last_window_save: Mutex::new(None),
        criando_janela: AtomicBool::new(false),
        worker: tx,
        worker_rx: Mutex::new(Some(rx)),
        worker_thread: Mutex::new(None),
        integracao: Mutex::new(Integracao::new(Box::new(WinRegistro), exe, data_dir, Box::new(unidade_removivel))),
        status: Mutex::new(AutoStatusView {
            state: AutoState::Parado,
            paused_until: None,
            watching: 0,
            busy: false,
            last_backup: None,
        }),
        saindo: AtomicBool::new(false),
        jogo_pendente: Mutex::new(None),
    };

    let app = tauri::Builder::default()
        // Primeiro: abrir o app de novo só traz a janela (o "--bandeja" do
        // início com o Windows não abre nada).
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            if !args.iter().any(|a| a == "--bandeja") {
                abrir_janela(app);
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(shared)
        .setup(|app| {
            let handle = app.handle().clone();
            let shared = app.state::<Shared>();
            let covers = travar(&shared.svc).paths().covers_dir();
            let _ = std::fs::create_dir_all(&covers);
            app.asset_protocol_scope().allow_directory(&covers, false)?;

            let status = travar(&shared.status).clone();
            tray::criar(&handle, &status)?;

            // "Iniciar com o Windows" apontando para outra cópia do app.
            let aviso = travar(&shared.integracao).corrigir_inicio();
            if let Some(aviso) = aviso {
                travar(&shared.svc).add_warning(aviso);
            }

            let rx = travar(&shared.worker_rx).take();
            if let Some(rx) = rx {
                let worker = AutoWorker::new(
                    shared.svc.clone(),
                    Box::new(SystemProbe),
                    Box::new(TauriSink(handle.clone())),
                    Box::new(SystemClock),
                    Box::new(HttpFetcher::default()),
                );
                *travar(&shared.worker_thread) = Some(auto_worker::spawn(worker, rx, shared.worker.clone()));
            }

            if !std::env::args().any(|a| a == "--bandeja") {
                criar_janela(&handle)?;
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            let Some(win) = window.app_handle().get_webview_window(window.label()) else { return };
            let shared = window.state::<Shared>();
            match event {
                WindowEvent::CloseRequested { api, .. } => {
                    lembrar_janela(&win, &shared);
                    if shared.fechar_para_bandeja.load(Ordering::SeqCst) {
                        // A janela (e a WebView) vai embora; o app fica na bandeja.
                        gravar_janela_depois(&shared);
                    } else {
                        api.prevent_close();
                        sair(window.app_handle());
                    }
                }
                WindowEvent::Moved(_) | WindowEvent::Resized(_) => save_window(&win, &shared, true),
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::estado,
            commands::biblioteca,
            commands::escanear,
            commands::fazer_backup,
            commands::backup_de_varios,
            commands::jogos_para_backup,
            commands::historico,
            commands::arquivos_do_backup,
            commands::previa_restauracao,
            commands::restaurar,
            commands::abrir_pasta,
            commands::escolher_pasta,
            commands::escolher_imagem,
            commands::adicionar_jogo,
            commands::renomear_jogo,
            commands::ignorar_jogo,
            commands::definir_pasta_do_jogo,
            commands::definir_capa,
            commands::esquecer_destino,
            commands::remover_jogo_manual,
            commands::configuracoes,
            commands::definir_pastas_de_jogos,
            commands::definir_incluir_config,
            commands::definir_capas_steam,
            commands::trocar_cofre,
            commands::inspecionar_cofre,
            commands::tentar_cofre_de_novo,
            commands::atualizar_base,
            commands::concluir_primeiro_uso,
            commands::jogos_abertos,
            commands::situacao_automatico,
            commands::definir_automatico,
            commands::pausar_automatico,
            commands::retomar_automatico,
            commands::definir_automatico_do_jogo,
            commands::definir_retencao,
            commands::definir_retencao_do_jogo,
            commands::definir_tempos_do_automatico,
            commands::definir_incluir_config_do_jogo,
            commands::fixar_backup,
            commands::apagar_backup,
            commands::atividade,
            commands::definir_fechar_para_bandeja,
            commands::definir_iniciar_com_windows,
            commands::definir_notificacoes,
            commands::remover_registros_do_windows,
            commands::jogo_para_abrir,
        ])
        .build(tauri::generate_context!());
    match app {
        Ok(app) => app.run(|app, event| {
            // Fechar a última janela não fecha o app: só o "Sair" (app.exit) fecha.
            if let RunEvent::ExitRequested { code: None, api, .. } = &event {
                api.prevent_exit();
                if !app.state::<Shared>().fechar_para_bandeja.load(Ordering::SeqCst) {
                    sair(app);
                }
            }
        }),
        Err(e) => avisar(&format!("O Save Control não conseguiu abrir: {e}")),
    }
}
