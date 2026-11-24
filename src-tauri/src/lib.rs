//! App Tauri do Save Control: janela, plugins e a ponte entre a interface e o
//! `AppService` (toda a regra fica em `savecontrol-app` e `savecontrol-core`).

mod commands;

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use savecontrol_app::config::WindowState;
use savecontrol_app::paths::resolve_paths;
use savecontrol_app::service::{AppService, BusyFlag};
use savecontrol_core::env::SystemPaths;
use savecontrol_core::registry::{RegistryReader, WinRegistry};
use tauri::{
    Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent,
};

/// Estado compartilhado pelos comandos.
pub struct Shared {
    pub svc: Arc<Mutex<AppService>>,
    pub busy: BusyFlag,
    /// Já existe uma thread baixando capas da Steam.
    pub covers_running: Arc<AtomicBool>,
    last_window_save: Mutex<Option<Instant>>,
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

/// Grava a geometria da janela sem travar a interface (se o serviço estiver
/// ocupado, deixa para a próxima).
fn save_window(win: &WebviewWindow, shared: &Shared, throttle: bool) {
    if throttle {
        let mut last = shared.last_window_save.lock().unwrap_or_else(|e| e.into_inner());
        if last.is_some_and(|t| t.elapsed() < Duration::from_secs(2)) {
            return;
        }
        *last = Some(Instant::now());
    }
    let Ok(mut svc) = shared.svc.try_lock() else { return };
    if let Some(ws) = window_state(win, svc.window()) {
        let _ = svc.save_window(ws);
    }
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

pub fn run() {
    let sys = match SystemPaths::detect() {
        Ok(s) => s,
        Err(e) => {
            avisar(&format!("Não foi possível achar as pastas do Windows: {e}"));
            return;
        }
    };
    let paths = resolve_paths(&exe_dir(), &sys.local_app_data);
    let busy = BusyFlag::default();
    let svc = AppService::new(paths, sys, Box::new(registro), busy.clone());
    let shared = Shared {
        svc: Arc::new(Mutex::new(svc)),
        busy,
        covers_running: Arc::new(AtomicBool::new(false)),
        last_window_save: Mutex::new(None),
    };

    let result = tauri::Builder::default()
        // Primeiro: abrir o app de novo só traz a janela que já existe.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.unminimize();
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(shared)
        .setup(|app| {
            let shared = app.state::<Shared>();
            let (saved, covers, webview) = {
                let svc = shared.svc.lock().unwrap_or_else(|e| e.into_inner());
                (svc.window(), svc.paths().covers_dir(), svc.paths().webview_dir())
            };
            let _ = std::fs::create_dir_all(&covers);
            app.asset_protocol_scope().allow_directory(&covers, false)?;
            let win = WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
                .title("Save Control")
                .decorations(false)
                .inner_size(1280.0, 800.0)
                .min_inner_size(1100.0, 700.0)
                .data_directory(webview)
                .visible(false)
                .build()?;
            restore_window(&win, saved);
            win.show()?;
            Ok(())
        })
        .on_window_event(|window, event| {
            let Some(win) = window.app_handle().get_webview_window(window.label()) else { return };
            let shared = window.state::<Shared>();
            match event {
                WindowEvent::CloseRequested { api, .. } => {
                    // Nunca fechar no meio de um backup ou restauração.
                    if shared.busy.is_busy() {
                        api.prevent_close();
                        let _ = window.emit("fechar-bloqueado", ());
                    } else {
                        save_window(&win, &shared, false);
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
        ])
        .run(tauri::generate_context!());
    if let Err(e) = result {
        avisar(&format!("O Save Control não conseguiu abrir: {e}"));
    }
}
