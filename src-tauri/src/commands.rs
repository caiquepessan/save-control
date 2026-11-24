//! Um comando Tauri por ação do `AppService`. Cada um roda numa thread de
//! trabalho com o serviço travado: as ações nunca rodam ao mesmo tempo.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

use chrono::Utc;
use savecontrol_app::covers::UreqCoverFetcher;
use savecontrol_app::dto::*;
use savecontrol_app::service::AppService;
use savecontrol_core::manifest::store::HttpFetcher;
use savecontrol_core::process::SystemProbe;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

use crate::Shared;

type Res<T> = Result<T, ErrorView>;

fn erro(message: impl Into<String>) -> ErrorView {
    ErrorView::new(ErrorKind::Erro, message)
}

/// Roda `f` com o serviço travado, fora da thread da interface.
async fn com_servico<T, F>(state: &State<'_, Shared>, f: F) -> Res<T>
where
    T: Send + 'static,
    F: FnOnce(&mut AppService) -> Res<T> + Send + 'static,
{
    let svc = state.svc.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut s = svc.lock().unwrap_or_else(|e| e.into_inner());
        f(&mut s)
    })
    .await
    .map_err(|e| erro(e.to_string()))?
}

#[derive(Clone, Serialize)]
struct CapaPronta {
    id: String,
    caminho: String,
}

/// Baixa as capas da Steam que faltam, uma por vez, sem travar o serviço.
fn baixar_capas(app: &AppHandle, state: &State<'_, Shared>) {
    if state.covers_running.swap(true, Ordering::SeqCst) {
        return;
    }
    let (lista, store) = {
        let svc = state.svc.lock().unwrap_or_else(|e| e.into_inner());
        (svc.steam_covers_to_fetch(), svc.cover_store())
    };
    let running = state.covers_running.clone();
    if lista.is_empty() {
        running.store(false, Ordering::SeqCst);
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        let fetcher = UreqCoverFetcher::default();
        for (id, appid) in lista {
            if let Some(path) = store.ensure_steam(appid, &fetcher, Utc::now()) {
                let _ = app.emit("capa", CapaPronta { id, caminho: path.display().to_string() });
            }
        }
        running.store(false, Ordering::SeqCst);
    });
}

#[tauri::command]
pub async fn estado(state: State<'_, Shared>) -> Res<AppStateView> {
    com_servico(&state, |s| Ok(s.state())).await
}

#[tauri::command]
pub async fn biblioteca(state: State<'_, Shared>) -> Res<LibraryView> {
    com_servico(&state, |s| s.library()).await
}

#[tauri::command]
pub async fn escanear(app: AppHandle, state: State<'_, Shared>) -> Res<LibraryView> {
    let lib = com_servico(&state, |s| s.scan()).await?;
    baixar_capas(&app, &state);
    Ok(lib)
}

#[tauri::command]
pub async fn fazer_backup(state: State<'_, Shared>, id: String, nome: Option<String>) -> Res<BackupResultView> {
    com_servico(&state, move |s| s.backup_game(&id, nome)).await
}

#[tauri::command]
pub async fn backup_de_varios(state: State<'_, Shared>, ids: Vec<String>) -> Res<BackupManyView> {
    com_servico(&state, move |s| Ok(s.backup_many(&ids))).await
}

#[tauri::command]
pub async fn jogos_para_backup(state: State<'_, Shared>) -> Res<Vec<String>> {
    com_servico(&state, |s| Ok(s.needs_backup_ids())).await
}

#[tauri::command]
pub async fn historico(state: State<'_, Shared>, id: String) -> Res<Vec<SnapshotView>> {
    com_servico(&state, move |s| s.history(&id)).await
}

#[tauri::command]
pub async fn arquivos_do_backup(state: State<'_, Shared>, id: String, arquivo: String) -> Res<Vec<SnapshotFileView>> {
    com_servico(&state, move |s| s.snapshot_files(&id, &arquivo)).await
}

#[tauri::command]
pub async fn previa_restauracao(
    state: State<'_, Shared>,
    id: String,
    arquivo: String,
    destinos: BTreeMap<String, String>,
) -> Res<RestorePreviewView> {
    com_servico(&state, move |s| s.restore_preview(&id, &arquivo, &destinos, &SystemProbe)).await
}

#[tauri::command]
pub async fn restaurar(state: State<'_, Shared>, id: String, arquivo: String, pedido: RestoreRequest) -> Res<RestoreResultView> {
    com_servico(&state, move |s| s.restore(&id, &arquivo, &pedido, &SystemProbe)).await
}

/// Abre a pasta no Explorador (ou a pasta do arquivo, com ele selecionado).
#[tauri::command]
pub async fn abrir_pasta(app: AppHandle, caminho: String) -> Res<()> {
    let p = PathBuf::from(&caminho);
    let r = if p.is_dir() {
        app.opener().open_path(caminho.clone(), None::<&str>)
    } else if p.is_file() {
        app.opener().reveal_item_in_dir(&p)
    } else if let Some(pai) = p.parent().filter(|x| x.is_dir()) {
        app.opener().open_path(pai.display().to_string(), None::<&str>)
    } else {
        return Err(ErrorView::new(ErrorKind::NaoEncontrado, format!("A pasta {caminho} não existe.")));
    };
    r.map_err(|e| erro(e.to_string()))
}

#[tauri::command]
pub async fn escolher_pasta(app: AppHandle, titulo: String) -> Res<Option<String>> {
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut d = handle.dialog().file().set_title(titulo);
        if let Some(w) = handle.get_webview_window("main") {
            d = d.set_parent(&w);
        }
        d.blocking_pick_folder().and_then(|p| p.into_path().ok()).map(|p| p.display().to_string())
    })
    .await
    .map_err(|e| erro(e.to_string()))
}

#[tauri::command]
pub async fn escolher_imagem(app: AppHandle) -> Res<Option<String>> {
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut d = handle.dialog().file().set_title("Escolher capa").add_filter("Imagens", &["png", "jpg", "jpeg", "webp"]);
        if let Some(w) = handle.get_webview_window("main") {
            d = d.set_parent(&w);
        }
        d.blocking_pick_file().and_then(|p| p.into_path().ok()).map(|p| p.display().to_string())
    })
    .await
    .map_err(|e| erro(e.to_string()))
}

#[tauri::command]
pub async fn adicionar_jogo(app: AppHandle, state: State<'_, Shared>, pedido: AddGameRequest) -> Res<GameView> {
    let jogo = com_servico(&state, move |s| s.add_game(pedido)).await?;
    baixar_capas(&app, &state);
    Ok(jogo)
}

#[tauri::command]
pub async fn renomear_jogo(state: State<'_, Shared>, id: String, titulo: Option<String>) -> Res<()> {
    com_servico(&state, move |s| s.rename_game(&id, titulo)).await
}

#[tauri::command]
pub async fn ignorar_jogo(state: State<'_, Shared>, id: String, ignorado: bool) -> Res<()> {
    com_servico(&state, move |s| s.set_ignored(&id, ignorado)).await
}

#[tauri::command]
pub async fn definir_pasta_do_jogo(state: State<'_, Shared>, id: String, pasta: Option<String>) -> Res<()> {
    com_servico(&state, move |s| s.set_game_dir(&id, pasta.map(PathBuf::from))).await
}

#[tauri::command]
pub async fn definir_capa(state: State<'_, Shared>, id: String, arquivo: Option<String>) -> Res<()> {
    com_servico(&state, move |s| s.set_cover(&id, arquivo.map(PathBuf::from))).await
}

#[tauri::command]
pub async fn esquecer_destino(state: State<'_, Shared>, id: String, template: String) -> Res<()> {
    com_servico(&state, move |s| s.forget_target(&id, &template)).await
}

#[tauri::command]
pub async fn remover_jogo_manual(state: State<'_, Shared>, id: String) -> Res<()> {
    com_servico(&state, move |s| s.remove_custom_game(&id)).await
}

#[tauri::command]
pub async fn configuracoes(state: State<'_, Shared>) -> Res<SettingsView> {
    com_servico(&state, |s| s.settings()).await
}

#[tauri::command]
pub async fn definir_pastas_de_jogos(state: State<'_, Shared>, pastas: Vec<String>) -> Res<()> {
    com_servico(&state, move |s| s.set_user_roots(pastas.into_iter().map(PathBuf::from).collect())).await
}

#[tauri::command]
pub async fn definir_incluir_config(state: State<'_, Shared>, ligado: bool) -> Res<()> {
    com_servico(&state, move |s| s.set_include_config(ligado)).await
}

#[tauri::command]
pub async fn definir_capas_steam(state: State<'_, Shared>, ligado: bool) -> Res<()> {
    com_servico(&state, move |s| s.set_steam_covers(ligado)).await
}

#[tauri::command]
pub async fn trocar_cofre(state: State<'_, Shared>, pasta: String, criar: bool) -> Res<AppStateView> {
    com_servico(&state, move |s| s.change_vault(Path::new(&pasta), criar)).await
}

#[tauri::command]
pub async fn inspecionar_cofre(state: State<'_, Shared>, pasta: String) -> Res<VaultProbeView> {
    com_servico(&state, move |s| Ok(s.inspect_vault(Path::new(&pasta)))).await
}

#[tauri::command]
pub async fn tentar_cofre_de_novo(state: State<'_, Shared>) -> Res<AppStateView> {
    com_servico(&state, |s| Ok(s.retry_vault())).await
}

#[tauri::command]
pub async fn atualizar_base(state: State<'_, Shared>) -> Res<ManifestUpdateView> {
    com_servico(&state, |s| s.update_manifest(&HttpFetcher)).await
}

#[tauri::command]
pub async fn concluir_primeiro_uso(state: State<'_, Shared>, pasta_do_cofre: String) -> Res<AppStateView> {
    com_servico(&state, move |s| s.finish_first_run(Path::new(&pasta_do_cofre))).await
}

#[tauri::command]
pub async fn jogos_abertos(state: State<'_, Shared>) -> Res<Vec<String>> {
    com_servico(&state, |s| Ok(s.running_ids(&SystemProbe))).await
}
