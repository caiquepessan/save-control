//! Ícone da bandeja (spec §9): ícone, dica e menu pela situação do backup
//! automático. Os textos e as escolhas ficam em `savecontrol_app::bandeja`.

use std::time::Duration;

use chrono::Local;
use savecontrol_app::auto_worker::Command;
use savecontrol_app::bandeja::{self, Bandeja, IconeBandeja, Pausa};
use savecontrol_app::dto::AutoStatusView;
use tauri::image::Image;
use tauri::menu::{IsMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Wry};
use tauri_plugin_opener::OpenerExt;

use crate::{Shared, travar};

const ID: &str = "principal";

fn imagem(i: IconeBandeja) -> tauri::Result<Image<'static>> {
    let png: &[u8] = match i {
        IconeBandeja::Normal => include_bytes!("../icons/bandeja-normal.png"),
        IconeBandeja::Backup => include_bytes!("../icons/bandeja-backup.png"),
        IconeBandeja::Pausado => include_bytes!("../icons/bandeja-pausado.png"),
        IconeBandeja::CofreFora => include_bytes!("../icons/bandeja-cofre-fora.png"),
    };
    Image::from_bytes(png)
}

fn item(app: &AppHandle, id: &str, texto: &str, ligado: bool) -> tauri::Result<MenuItem<Wry>> {
    MenuItem::with_id(app, id, texto, ligado, None::<&str>)
}

/// O menu do mockup, refeito a cada mudança de situação.
fn menu(app: &AppHandle, b: &Bandeja) -> tauri::Result<Menu<Wry>> {
    let titulo = item(app, "titulo", &b.titulo, false)?;
    let abrir = item(app, "abrir", bandeja::ABRIR, true)?;
    let tudo = item(app, "backup-tudo", bandeja::BACKUP_DE_TUDO, b.backup_ok)?;
    let cofre = item(app, "pasta-cofre", bandeja::ABRIR_COFRE, b.backup_ok)?;
    let separador = PredefinedMenuItem::separator(app)?;
    let sair = item(app, "sair", bandeja::SAIR, true)?;
    let retomar;
    let pausar;
    let pausa: &dyn IsMenuItem<Wry> = if b.pausa == Pausa::Retomar {
        retomar = item(app, "retomar", bandeja::RETOMAR, true)?;
        &retomar
    } else {
        let meia_hora = item(app, "pausar-30", bandeja::PAUSAR_30_MIN, true)?;
        let duas_horas = item(app, "pausar-120", bandeja::PAUSAR_2_H, true)?;
        let ate_reabrir = item(app, "pausar-reabrir", bandeja::PAUSAR_ATE_REABRIR, true)?;
        pausar = Submenu::with_id_and_items(
            app,
            "pausar",
            bandeja::PAUSAR,
            b.pausa == Pausa::Pausar,
            &[&meia_hora, &duas_horas, &ate_reabrir],
        )?;
        &pausar
    };
    Menu::with_items(app, &[&titulo, &abrir, &tudo, pausa, &cofre, &separador, &sair])
}

fn desenho(s: &AutoStatusView) -> Bandeja {
    bandeja::bandeja(s, *Local::now().offset())
}

pub fn criar(app: &AppHandle, s: &AutoStatusView) -> tauri::Result<()> {
    let b = desenho(s);
    TrayIconBuilder::with_id(ID)
        .icon(imagem(b.icone)?)
        .tooltip(&b.dica)
        .menu(&menu(app, &b)?)
        .show_menu_on_left_click(false)
        .on_menu_event(no_menu)
        .on_tray_icon_event(|tray, ev| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = ev {
                crate::abrir_janela(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

/// Ícone, dica e menu pela situação.
pub fn atualizar(app: &AppHandle, s: &AutoStatusView) {
    let Some(tray) = app.tray_by_id(ID) else { return };
    let b = desenho(s);
    if let Ok(i) = imagem(b.icone) {
        let _ = tray.set_icon(Some(i));
    }
    let _ = tray.set_tooltip(Some(&b.dica));
    if let Ok(m) = menu(app, &b) {
        let _ = tray.set_menu(Some(m));
    }
}

/// Troca só a dica (ex.: "Terminando o backup…" ao sair).
pub fn dica(app: &AppHandle, texto: &str) {
    if let Some(tray) = app.tray_by_id(ID) {
        let _ = tray.set_tooltip(Some(texto));
    }
}

fn no_menu(app: &AppHandle, ev: MenuEvent) {
    let shared = app.state::<Shared>();
    match ev.id().as_ref() {
        "abrir" => crate::abrir_janela(app),
        "backup-tudo" => backup_de_tudo(app),
        "pausar-30" => shared.mandar(Command::Pause(Some(Duration::from_secs(30 * 60)))),
        "pausar-120" => shared.mandar(Command::Pause(Some(Duration::from_secs(2 * 60 * 60)))),
        "pausar-reabrir" => shared.mandar(Command::Pause(None)),
        "retomar" => shared.mandar(Command::Resume),
        "pasta-cofre" => abrir_cofre(app),
        "sair" => crate::sair(app),
        _ => {}
    }
}

/// "Backup de tudo agora": os jogos com save novo, como o botão da Biblioteca.
/// Roda fora da thread principal (o serviço pode estar ocupado).
fn backup_de_tudo(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        let shared = app.state::<Shared>();
        let agora = travar(&shared.status).clone();
        atualizar(&app, &AutoStatusView { busy: true, ..agora });
        {
            let mut svc = travar(&shared.svc);
            let ids = svc.needs_backup_ids();
            if !ids.is_empty() {
                let _ = svc.backup_many(&ids);
            }
        }
        let depois = travar(&shared.status).clone();
        atualizar(&app, &depois);
        let _ = app.emit("biblioteca-mudou", ());
    });
}

fn abrir_cofre(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        let pasta = travar(&app.state::<Shared>().svc).state().vault_dir;
        let _ = app.opener().open_path(pasta, None::<&str>);
    });
}
