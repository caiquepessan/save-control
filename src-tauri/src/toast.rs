//! Notificação do Windows de um lote de backups automáticos (spec §9, §13).
//! O texto vem de `savecontrol_app::bandeja::notificacao`.

use savecontrol_app::dto::AutoBackupView;
use tauri::AppHandle;

/// Mostra a notificação. Devolve falso se o Windows não aceitou (aí o aviso
/// vai para dentro da janela).
#[cfg(windows)]
pub fn mostrar(app: &AppHandle, itens: &[AutoBackupView]) -> bool {
    use std::path::Path;

    use savecontrol_app::bandeja::{self, notificacao};
    use savecontrol_app::registro::AUMID;
    use tauri_winrt_notification::{IconCrop, Toast};

    let Some(n) = notificacao(itens) else { return false };
    let mut toast = Toast::new(AUMID).title(&n.titulo).text1(&n.texto).sound(None);
    if let (Some(capa), Some(primeiro)) = (n.capa.as_deref(), itens.first()) {
        if Path::new(capa).is_file() {
            toast = toast.icon(Path::new(capa), IconCrop::Square, &primeiro.title);
        }
    }
    let app = app.clone();
    let jogo = n.jogo.clone();
    toast
        .add_button(bandeja::VER_HISTORICO, &format!("historico:{}", n.jogo))
        .add_button(bandeja::DISPENSAR, "dispensar")
        .on_activated(move |acao| {
            match acao.as_deref() {
                Some("dispensar") => {}
                Some(a) => {
                    if let Some(id) = a.strip_prefix("historico:") {
                        crate::abrir_jogo(&app, id);
                    }
                }
                // Clique no corpo do aviso: abre o painel do jogo (§13).
                None => crate::abrir_jogo(&app, &jogo),
            }
            Ok(())
        })
        .show()
        .is_ok()
}

#[cfg(not(windows))]
pub fn mostrar(_app: &AppHandle, _itens: &[AutoBackupView]) -> bool {
    false
}
