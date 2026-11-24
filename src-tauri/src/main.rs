// Sem janela de console na versão final (no `tauri dev` o console mostra os logs).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if tauri::webview_version().is_err() {
        save_control_lib::avisar(savecontrol_app::texts::WEBVIEW2_MISSING);
        return;
    }
    save_control_lib::run();
}
