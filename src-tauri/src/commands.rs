//! Um comando Tauri por ação do `AppService`. Cada um roda numa thread de
//! trabalho com o serviço travado: as ações nunca rodam ao mesmo tempo.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::Duration;

use chrono::Utc;
use savecontrol_app::activity::ActivityEntry;
use savecontrol_app::auto_worker::Command;
use savecontrol_app::covers::{CoverStore, UreqCoverFetcher};
use savecontrol_app::dto::*;
use savecontrol_app::service::AppService;
use savecontrol_core::manifest::store::HttpFetcher;
use savecontrol_core::process::SystemProbe;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

use crate::{Shared, travar};

/// Ícone das notificações do Windows (copiado para a pasta de dados ao ligar).
const ICONE_NOTIFICACAO: &[u8] = include_bytes!("../icons/128x128.png");

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
fn baixar_capas(app: &AppHandle, state: &State<'_, Shared>, lista: Vec<(String, u32)>, store: CoverStore) {
    if lista.is_empty() || state.covers_running.swap(true, Ordering::SeqCst) {
        return;
    }
    let running = state.covers_running.clone();
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

/// Avisa o trabalhador do automático que jogos ou ajustes mudaram (pastas
/// vigiadas, tempos, chaves).
fn mudou(state: &State<'_, Shared>) {
    state.mandar(Command::ConfigChanged);
}

fn erro_do_registro(e: std::io::Error) -> ErrorView {
    erro(format!("Não foi possível mudar o registro do Windows: {e}"))
}

#[tauri::command]
pub async fn estado(state: State<'_, Shared>) -> Res<AppStateView> {
    com_servico(&state, |s| Ok(s.opening_state())).await
}

#[tauri::command]
pub async fn biblioteca(state: State<'_, Shared>) -> Res<LibraryView> {
    com_servico(&state, |s| s.library()).await
}

#[tauri::command]
pub async fn escanear(app: AppHandle, state: State<'_, Shared>) -> Res<LibraryView> {
    let (lib, capas, store) = com_servico(&state, |s| {
        let (lib, capas) = s.scan_with_covers()?;
        Ok((lib, capas, s.cover_store()))
    })
    .await?;
    baixar_capas(&app, &state, capas, store);
    mudou(&state);
    Ok(lib)
}

#[tauri::command]
pub async fn fazer_backup(state: State<'_, Shared>, id: String, nome: Option<String>) -> Res<BackupResultView> {
    com_servico(&state, move |s| s.backup_game(&id, nome)).await
}

#[tauri::command]
pub async fn backup_de_varios(state: State<'_, Shared>, ids: Vec<String>) -> Res<BackupManyView> {
    com_servico(&state, move |s| s.backup_many(&ids)).await
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
    let (jogo, capas, store) = com_servico(&state, move |s| {
        let jogo = s.add_game(pedido)?;
        Ok((jogo, s.steam_covers_to_fetch(), s.cover_store()))
    })
    .await?;
    baixar_capas(&app, &state, capas, store);
    mudou(&state);
    Ok(jogo)
}

#[tauri::command]
pub async fn renomear_jogo(state: State<'_, Shared>, id: String, titulo: Option<String>) -> Res<()> {
    com_servico(&state, move |s| s.rename_game(&id, titulo)).await
}

#[tauri::command]
pub async fn ignorar_jogo(state: State<'_, Shared>, id: String, ignorado: bool) -> Res<()> {
    com_servico(&state, move |s| s.set_ignored(&id, ignorado)).await?;
    mudou(&state);
    Ok(())
}

#[tauri::command]
pub async fn definir_pasta_do_jogo(state: State<'_, Shared>, id: String, pasta: Option<String>) -> Res<()> {
    com_servico(&state, move |s| s.set_game_dir(&id, pasta.map(PathBuf::from))).await?;
    mudou(&state);
    Ok(())
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
    com_servico(&state, move |s| s.remove_custom_game(&id)).await?;
    mudou(&state);
    Ok(())
}

#[tauri::command]
pub async fn configuracoes(state: State<'_, Shared>) -> Res<SettingsView> {
    let mut ajustes = com_servico(&state, |s| s.settings()).await?;
    // A fonte da verdade do "Iniciar com o Windows" é a entrada no registro.
    ajustes.start_with_windows = travar(&state.integracao).iniciar_com_windows();
    Ok(ajustes)
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
    let estado = com_servico(&state, move |s| s.change_vault(Path::new(&pasta), criar)).await?;
    mudou(&state);
    Ok(estado)
}

#[tauri::command]
pub async fn inspecionar_cofre(state: State<'_, Shared>, pasta: String) -> Res<VaultProbeView> {
    com_servico(&state, move |s| s.inspect_vault(Path::new(&pasta))).await
}

#[tauri::command]
pub async fn tentar_cofre_de_novo(state: State<'_, Shared>) -> Res<AppStateView> {
    com_servico(&state, |s| Ok(s.retry_vault())).await
}

#[tauri::command]
pub async fn atualizar_base(state: State<'_, Shared>) -> Res<ManifestUpdateView> {
    com_servico(&state, |s| s.update_manifest(&HttpFetcher::default())).await
}

#[tauri::command]
pub async fn concluir_primeiro_uso(state: State<'_, Shared>, pasta_do_cofre: String) -> Res<AppStateView> {
    com_servico(&state, move |s| s.finish_first_run(Path::new(&pasta_do_cofre))).await
}

#[tauri::command]
pub async fn jogos_abertos(state: State<'_, Shared>) -> Res<Vec<String>> {
    com_servico(&state, |s| Ok(s.running_ids(&SystemProbe))).await
}

#[tauri::command]
pub async fn situacao_automatico(state: State<'_, Shared>) -> Res<AutoStatusView> {
    Ok(travar(&state.status).clone())
}

#[tauri::command]
pub async fn definir_automatico(state: State<'_, Shared>, ligado: bool) -> Res<()> {
    com_servico(&state, move |s| s.set_auto_backup(ligado)).await?;
    mudou(&state);
    Ok(())
}

/// Pausa o automático por `minutos` (sem valor: até reabrir o app).
#[tauri::command]
pub async fn pausar_automatico(state: State<'_, Shared>, minutos: Option<u32>) -> Res<()> {
    state.mandar(Command::Pause(minutos.map(|m| Duration::from_secs(u64::from(m) * 60))));
    Ok(())
}

#[tauri::command]
pub async fn retomar_automatico(state: State<'_, Shared>) -> Res<()> {
    state.mandar(Command::Resume);
    Ok(())
}

#[tauri::command]
pub async fn definir_automatico_do_jogo(state: State<'_, Shared>, id: String, ligado: bool) -> Res<()> {
    com_servico(&state, move |s| s.set_game_auto(&id, ligado)).await?;
    mudou(&state);
    Ok(())
}

#[tauri::command]
pub async fn definir_retencao(state: State<'_, Shared>, politica: RetentionView) -> Res<()> {
    com_servico(&state, move |s| s.set_retention(politica)).await
}

/// Sem política: o jogo volta a usar o padrão.
#[tauri::command]
pub async fn definir_retencao_do_jogo(state: State<'_, Shared>, id: String, politica: Option<RetentionView>) -> Res<()> {
    com_servico(&state, move |s| s.set_game_retention(&id, politica)).await
}

#[tauri::command]
pub async fn definir_tempos_do_automatico(state: State<'_, Shared>, tempos: AutoTimesView) -> Res<()> {
    com_servico(&state, move |s| s.set_auto_times(tempos)).await?;
    mudou(&state);
    Ok(())
}

/// Sem valor: o jogo segue o ajuste geral.
#[tauri::command]
pub async fn definir_incluir_config_do_jogo(state: State<'_, Shared>, id: String, valor: Option<bool>) -> Res<()> {
    com_servico(&state, move |s| s.set_game_include_config(&id, valor)).await?;
    mudou(&state);
    Ok(())
}

#[tauri::command]
pub async fn fixar_backup(state: State<'_, Shared>, id: String, arquivo: String, fixado: bool) -> Res<SnapshotView> {
    com_servico(&state, move |s| s.pin_backup(&id, &arquivo, fixado)).await
}

#[tauri::command]
pub async fn apagar_backup(state: State<'_, Shared>, id: String, arquivo: String) -> Res<()> {
    com_servico(&state, move |s| s.delete_backup(&id, &arquivo)).await
}

/// Os eventos mais novos da Atividade (padrão: 200).
#[tauri::command]
pub async fn atividade(state: State<'_, Shared>, limite: Option<usize>) -> Res<Vec<ActivityEntry>> {
    com_servico(&state, move |s| Ok(s.activity(limite.unwrap_or(200)))).await
}

#[tauri::command]
pub async fn definir_fechar_para_bandeja(state: State<'_, Shared>, ligado: bool) -> Res<()> {
    com_servico(&state, move |s| s.set_close_to_tray(ligado)).await?;
    state.fechar_para_bandeja.store(ligado, Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
pub async fn definir_iniciar_com_windows(state: State<'_, Shared>, ligado: bool) -> Res<()> {
    let r = travar(&state.integracao).definir_iniciar_com_windows(ligado);
    r.map_err(erro_do_registro)
}

/// Ligar registra o nome e o ícone do app para as notificações do Windows;
/// desligar apaga esse registro.
#[tauri::command]
pub async fn definir_notificacoes(state: State<'_, Shared>, ligado: bool) -> Res<()> {
    let r = {
        let mut integracao = travar(&state.integracao);
        if ligado { integracao.registrar_notificacoes(ICONE_NOTIFICACAO) } else { integracao.remover_notificacoes() }
    };
    r.map_err(erro_do_registro)?;
    com_servico(&state, move |s| s.set_notifications(ligado)).await?;
    state.notificacoes.store(ligado, Ordering::SeqCst);
    Ok(())
}

/// "Remover registros do Windows": apaga as duas entradas (início com o
/// Windows e notificações) e desliga as notificações.
#[tauri::command]
pub async fn remover_registros_do_windows(state: State<'_, Shared>) -> Res<()> {
    let r = travar(&state.integracao).remover_tudo();
    r.map_err(erro_do_registro)?;
    com_servico(&state, |s| s.set_notifications(false)).await?;
    state.notificacoes.store(false, Ordering::SeqCst);
    Ok(())
}

/// O jogo que a janela recém-criada deve abrir (clique na notificação), uma vez só.
#[tauri::command]
pub async fn jogo_para_abrir(state: State<'_, Shared>) -> Res<Option<String>> {
    Ok(travar(&state.jogo_pendente).take())
}
