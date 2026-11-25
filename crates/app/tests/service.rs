//! O serviço do app: fases, backup, restauração, cofre e configurações.
mod common;

use std::collections::BTreeMap;
use std::path::PathBuf;

use common::{Mundo, ler, mover_app, mundo, pronto, servico, servico_em, write};
use savecontrol_app::view::path_str;
use savecontrol_app::config::{AppConfig, load_config, save_config};
use savecontrol_app::dto::*;
use savecontrol_app::service::{AppService, BusyFlag};
use savecontrol_app::texts;
use savecontrol_core::error::{Error, Result};
use savecontrol_core::manifest::store::{FetchResult, Fetcher};
use savecontrol_core::process::{FakeProbe, RunningProcess};
use savecontrol_core::vault::Vault;

fn nenhum() -> FakeProbe {
    FakeProbe(vec![])
}

/// Jogo à mão com um arquivo de save; devolve (id, pasta do save).
fn jogo(svc: &mut AppService, w: &Mundo, nome: &str, processos: &[&str]) -> (String, PathBuf) {
    let dir = w.sys.app_data.join(nome);
    write(&dir.join("slot.sav"), nome.as_bytes());
    let g = svc
        .add_game(AddGameRequest {
            name: nome.into(),
            save_dirs: vec![dir.display().to_string()],
            processes: processos.iter().map(|p| p.to_string()).collect(),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(g.title, nome);
    (g.id, dir)
}

fn status(svc: &mut AppService, id: &str) -> StatusView {
    svc.library().unwrap().games.into_iter().find(|g| g.id == id).unwrap().status
}

#[test]
fn primeiro_uso_sem_config() {
    let w = mundo();
    let mut svc = servico(&w, BusyFlag::default());
    let st = svc.state();
    assert_eq!(st.phase, AppPhase::PrimeiroUso);
    assert!(st.default_vault.ends_with(r"App\cofre"), "{}", st.default_vault);
    assert!(st.warnings.is_empty());
}

#[test]
fn cofre_sumido_nao_e_recriado() {
    let w = mundo();
    let sumido = w.t.path().join("HD-desligado").join("cofre");
    let cfg = AppConfig { vault_dir: Some(sumido.clone()), first_run_done: true, ..Default::default() };
    save_config(&w.exe.join("SaveControl-dados").join("config.json"), &cfg).unwrap();
    let mut svc = servico(&w, BusyFlag::default());
    assert_eq!(svc.state().phase, AppPhase::CofreSumiu);
    assert!(svc.library().is_err());
    assert!(svc.scan().is_err());
    assert!(!sumido.exists(), "o cofre sumido foi recriado");
    assert!(!w.t.path().join("HD-desligado").exists());
}

#[test]
fn concluir_primeiro_uso_abre_o_cofre_escolhido() {
    let w = mundo();
    let mut svc = servico(&w, BusyFlag::default());
    let meu = w.t.path().join("MeuCofre");
    let st = svc.finish_first_run(&meu).unwrap();
    assert_eq!(st.phase, AppPhase::Pronto);
    assert!(meu.join("cofre.json").is_file());
    let cfg = load_config(&w.exe.join("SaveControl-dados").join("config.json")).config;
    assert!(cfg.first_run_done);
    assert_eq!(cfg.vault_dir, Some(meu));
}

#[test]
fn backup_de_tudo_so_pega_save_novo_e_sem_backup() {
    let w = mundo();
    let mut svc = pronto(&w);
    let (a, _) = jogo(&mut svc, &w, "JogoA", &[]);
    let (b, dir_b) = jogo(&mut svc, &w, "JogoB", &[]);
    let (c, _) = jogo(&mut svc, &w, "JogoC", &[]);
    svc.backup_game(&a, None).unwrap();
    svc.backup_game(&b, None).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(30));
    write(&dir_b.join("slot.sav"), b"JogoB depois de jogar");
    svc.scan().unwrap();

    let mut precisa = svc.needs_backup_ids();
    precisa.sort();
    let mut esperado = vec![b.clone(), c.clone()];
    esperado.sort();
    assert_eq!(precisa, esperado);

    let r = svc.backup_many(&precisa).unwrap();
    let mut feitos = r.done.clone();
    feitos.sort();
    assert_eq!(feitos, esperado);
    assert!(r.failed.is_empty());
    assert_eq!(svc.history(&a).unwrap().len(), 1);
    assert!(svc.needs_backup_ids().is_empty());
}

#[test]
fn backup_com_nome_aparece_no_historico() {
    let w = mundo();
    let mut svc = pronto(&w);
    let (a, _) = jogo(&mut svc, &w, "JogoA", &[]);
    let r = svc.backup_game(&a, Some("antes de atualizar".into())).unwrap();
    assert_eq!(r.outcome, BackupOutcomeView::Feito);
    let h = svc.history(&a).unwrap();
    assert_eq!(h[0].label.as_deref(), Some("antes de atualizar"));
    assert_eq!(r.snapshot.unwrap().file_name, h[0].file_name);
}

#[test]
fn backup_muda_o_status_sem_reescanear_tudo() {
    let w = mundo();
    let mut svc = pronto(&w);
    let (a, _) = jogo(&mut svc, &w, "JogoA", &[]);
    assert_eq!(status(&mut svc, &a), StatusView::SemBackup);
    svc.backup_game(&a, None).unwrap();
    assert_eq!(status(&mut svc, &a), StatusView::Protegido);
}

#[test]
fn jogo_aberto_bloqueia_restauracao() {
    let w = mundo();
    let mut svc = pronto(&w);
    let (a, dir) = jogo(&mut svc, &w, "JogoA", &["JogoA.exe"]);
    let m = svc.backup_game(&a, None).unwrap().snapshot.unwrap();
    write(&dir.join("slot.sav"), b"progresso de hoje");
    let aberto = FakeProbe(vec![RunningProcess { name: "JogoA.exe".into(), exe_path: None }]);

    let p = svc.restore_preview(&a, &m.file_name, &BTreeMap::new(), &aberto).unwrap();
    assert!(p.running);
    assert!(p.single_root);
    let err = svc.restore(&a, &m.file_name, &RestoreRequest::default(), &aberto).unwrap_err();
    assert_eq!(err.kind, ErrorKind::JogoAberto);
    assert_eq!(ler(&dir.join("slot.sav")), "progresso de hoje");
}

#[test]
fn restauracao_pede_destino_e_lembra() {
    let w = mundo();
    let mut svc = pronto(&w);
    let gd = w.t.path().join("Jogos").join("Portatil");
    write(&gd.join("saves").join("s1.sav"), b"x");
    let g = svc
        .add_game(AddGameRequest {
            name: "Portátil".into(),
            save_dirs: vec![gd.join("saves").display().to_string()],
            game_dir: Some(gd.display().to_string()),
            ..Default::default()
        })
        .unwrap();
    let m = svc.backup_game(&g.id, None).unwrap().snapshot.unwrap();
    svc.set_game_dir(&g.id, None).unwrap();

    let err = svc.restore_preview(&g.id, &m.file_name, &BTreeMap::new(), &nenhum()).unwrap_err();
    assert_eq!(err.kind, ErrorKind::PrecisaDestino);
    assert_eq!(err.root.as_deref(), Some("<gameDir>/saves"));
    assert_eq!(err.alias.as_deref(), Some("r0"), "a interface precisa saber qual raiz pede destino");

    let x = w.t.path().join("Escolhido");
    let pedido = RestoreRequest {
        targets: BTreeMap::from([("r0".to_string(), x.display().to_string())]),
        remember: true,
        ..Default::default()
    };
    svc.restore(&g.id, &m.file_name, &pedido, &nenhum()).unwrap();
    assert_eq!(ler(&x.join("s1.sav")), "x");
    let v = svc.library().unwrap().games.into_iter().find(|j| j.id == g.id).unwrap();
    assert_eq!(v.remembered_targets.len(), 1);
    assert_eq!(v.remembered_targets[0].template, "<gameDir>/saves");
}

#[test]
fn biblioteca_fica_em_cache() {
    let w = mundo();
    let mut svc = pronto(&w);
    let (a, _) = jogo(&mut svc, &w, "JogoA", &[]);
    svc.scan().unwrap();
    drop(svc);

    let mut outro = servico(&w, BusyFlag::default());
    assert_eq!(outro.state().phase, AppPhase::Pronto);
    let lib = outro.library().unwrap();
    assert!(lib.games.iter().any(|g| g.id == a), "a biblioteca não veio do cache");
}

#[test]
fn trocar_cofre_para_pasta_com_cofre_existente_abre_ele() {
    let w = mundo();
    let mut svc = pronto(&w);
    let outro = w.t.path().join("OutroCofre");
    Vault::open_or_create(&outro).unwrap();
    let st = svc.change_vault(&outro, false).unwrap();
    assert_eq!(st.phase, AppPhase::Pronto);
    assert_eq!(st.vault_dir, outro.display().to_string());
}

#[test]
fn trocar_para_pasta_vazia_so_cria_quando_pedido() {
    let w = mundo();
    let mut svc = pronto(&w);
    let vazia = w.t.path().join("Vazia");
    let err = svc.change_vault(&vazia, false).unwrap_err();
    assert_eq!(err.kind, ErrorKind::NaoEncontrado);
    assert!(!vazia.join("cofre.json").exists());
    svc.change_vault(&vazia, true).unwrap();
    assert!(vazia.join("cofre.json").is_file());
}

#[test]
fn inspecionar_pasta_com_cofre_conta_versoes() {
    let w = mundo();
    let mut svc = pronto(&w);
    let (a, dir) = jogo(&mut svc, &w, "JogoA", &[]);
    svc.backup_game(&a, None).unwrap();
    write(&dir.join("slot.sav"), b"outra versao");
    svc.backup_game(&a, None).unwrap();
    let cofre = w.exe.join("cofre");
    assert_eq!(svc.inspect_vault(&cofre).unwrap(), VaultProbeView { exists: true, versions: 2, path: path_str(&cofre) });

    let vazia = w.t.path().join("Vazia");
    std::fs::create_dir_all(&vazia).unwrap();
    assert_eq!(svc.inspect_vault(&vazia).unwrap(), VaultProbeView { exists: false, versions: 0, path: path_str(&vazia) });
    assert_eq!(std::fs::read_dir(&vazia).unwrap().count(), 0, "inspecionar criou arquivos");
}

/// Um cofre novo nunca é misturado com outros arquivos: numa pasta de jogos,
/// os backups poderiam cair dentro da pasta de um jogo (e sumir com ela).
#[test]
fn cofre_novo_em_pasta_com_arquivos_vai_para_a_subpasta() {
    let w = mundo();
    let mut svc = pronto(&w);
    let jogos = w.t.path().join("Jogos");
    write(&jogos.join("Ship of Harkinian").join("soh.exe"), b"exe");
    let sub = jogos.join("cofre");
    assert_eq!(svc.inspect_vault(&jogos).unwrap(), VaultProbeView { exists: false, versions: 0, path: path_str(&sub) });

    let st = svc.change_vault(&jogos, true).unwrap();
    assert_eq!(st.vault_dir, path_str(&sub));
    assert!(sub.join("cofre.json").is_file());
    assert!(!jogos.join("cofre.json").exists(), "o cofre foi criado no meio da pasta de jogos");
}

#[test]
fn primeiro_uso_em_pasta_com_arquivos_cria_o_cofre_na_subpasta() {
    let w = mundo();
    let mut svc = servico(&w, BusyFlag::default());
    let d = w.t.path().join("Backups");
    write(&d.join("outra-coisa.txt"), b"x");
    let st = svc.finish_first_run(&d).unwrap();
    assert_eq!(st.vault_dir, path_str(&d.join("cofre")));
    assert!(d.join("cofre").join("cofre.json").is_file());
    assert!(!d.join("cofre.json").exists());
}

/// Escolher a pasta do app (que tem o cofre dentro) acha o cofre de dentro.
#[test]
fn pasta_que_tem_um_cofre_dentro_usa_ele() {
    let w = mundo();
    let svc = pronto(&w);
    let sonda = svc.inspect_vault(&w.exe).unwrap();
    assert!(sonda.exists);
    assert_eq!(sonda.path, path_str(&w.exe.join("cofre")));
}

#[test]
fn config_corrompida_abre_e_avisa() {
    let w = mundo();
    write(&w.exe.join("SaveControl-dados").join("config.json"), b"{lixo");
    let mut svc = servico(&w, BusyFlag::default());
    let st = svc.state();
    assert_eq!(st.phase, AppPhase::PrimeiroUso);
    assert!(st.warnings.iter().any(|x| x == texts::CONFIG_RESET));
}

#[test]
fn ocupado_liga_e_desliga_mesmo_com_erro() {
    let busy = BusyFlag::default();
    {
        let _g = busy.guard();
        let _h = busy.guard();
        assert!(busy.is_busy());
    }
    assert!(!busy.is_busy());

    let w = mundo();
    let mut svc = servico(&w, busy.clone());
    svc.finish_first_run(&w.exe.join("cofre")).unwrap();
    assert!(svc.backup_game("man:nao-existe", None).is_err());
    assert!(!busy.is_busy());
}

struct SemInternet;

impl Fetcher for SemInternet {
    fn fetch(&self, _url: &str, _etag: Option<&str>) -> Result<FetchResult> {
        Err(Error::Http("sem conexão".into()))
    }
}

#[test]
fn base_sem_internet_nao_e_erro() {
    let w = mundo();
    let mut svc = pronto(&w);
    let r = svc.update_manifest(&SemInternet).unwrap();
    assert_eq!(r.outcome, ManifestOutcome::Falhou);
    assert!(r.message.unwrap().contains("sem conexão"));
    assert!(!svc.settings().unwrap().manifest_loaded);
}

/// Anota se a janela estaria travada (ocupado) durante o download.
struct Espia(BusyFlag, std::cell::Cell<Option<bool>>);

impl Fetcher for Espia {
    fn fetch(&self, _url: &str, _etag: Option<&str>) -> Result<FetchResult> {
        self.1.set(Some(self.0.is_busy()));
        Err(Error::Http("sem conexão".into()))
    }
}

#[test]
fn atualizar_a_base_nao_segura_o_fechar() {
    let busy = BusyFlag::default();
    let w = mundo();
    let mut svc = servico(&w, busy.clone());
    svc.finish_first_run(&w.exe.join("cofre")).unwrap();
    let espia = Espia(busy.clone(), std::cell::Cell::new(None));
    svc.update_manifest(&espia).unwrap();
    assert_eq!(espia.1.get(), Some(false), "a janela não fecharia durante o download da base");
}

/// O HD com o app aparece com outra letra em outro PC: o cofre ao lado do
/// `.exe` tem de continuar sendo achado.
#[test]
fn cofre_padrao_acompanha_a_pasta_do_app() {
    let w = mundo();
    let mut svc = pronto(&w);
    let (a, _) = jogo(&mut svc, &w, "JogoA", &[]);
    svc.backup_game(&a, None).unwrap();
    drop(svc);

    let novo = mover_app(&w);
    let mut svc = servico_em(&w, &novo, BusyFlag::default());
    let st = svc.state();
    assert_eq!(st.phase, AppPhase::Pronto, "o cofre ao lado do app não foi achado depois de mover");
    assert_eq!(st.vault_dir, path_str(&novo.join("cofre")));
    assert_eq!(svc.history(&a).unwrap().len(), 1);
}

#[test]
fn cofre_dentro_da_pasta_do_app_acompanha_a_mudanca() {
    let w = mundo();
    let mut svc = pronto(&w);
    svc.change_vault(&w.exe.join("Cofres").join("Principal"), true).unwrap();
    drop(svc);

    let novo = mover_app(&w);
    let mut svc = servico_em(&w, &novo, BusyFlag::default());
    let st = svc.state();
    assert_eq!(st.phase, AppPhase::Pronto);
    assert_eq!(st.vault_dir, path_str(&novo.join("Cofres").join("Principal")));
}

#[test]
fn capa_propria_acompanha_a_pasta_do_app() {
    let w = mundo();
    let mut svc = pronto(&w);
    let (a, _) = jogo(&mut svc, &w, "JogoA", &[]);
    let img = w.t.path().join("minha-capa.png");
    write(&img, b"\x89PNG imagem de teste");
    svc.set_cover(&a, Some(img)).unwrap();
    drop(svc);

    let novo = mover_app(&w);
    let mut svc = servico_em(&w, &novo, BusyFlag::default());
    let v = svc.scan().unwrap().games.into_iter().find(|g| g.id == a).unwrap();
    assert_eq!(v.cover.kind, CoverKind::Propria, "a capa escolhida sumiu depois de mover o app");
    assert!(v.cover.path.unwrap().starts_with(&path_str(&novo)));
}

/// O HD do cofre desligou com o app aberto: escanear não pode mostrar todo
/// mundo como "Sem backup" nem apagar o histórico guardado.
#[test]
fn escanear_com_o_cofre_fora_nao_esvazia_o_historico() {
    let w = mundo();
    let mut svc = pronto(&w);
    let (a, _) = jogo(&mut svc, &w, "JogoA", &[]);
    svc.backup_game(&a, None).unwrap();
    svc.scan().unwrap();
    let cofre = w.exe.join("cofre");
    let fora = w.t.path().join("cofre-no-hd-desligado");
    std::fs::rename(&cofre, &fora).unwrap();

    let lib = svc.scan().unwrap();
    assert!(!lib.vault.available);
    let g = lib.games.iter().find(|g| g.id == a).unwrap();
    assert_eq!(g.status, StatusView::Protegido, "o jogo virou 'Sem backup' com o cofre fora");
    assert_eq!(g.backup_count, 1);

    std::fs::rename(&fora, &cofre).unwrap();
    let lib = svc.scan().unwrap();
    assert!(lib.vault.available);
    assert_eq!(lib.games.iter().find(|g| g.id == a).unwrap().backup_count, 1);
}

/// Com o cofre fora, "Backup de tudo" dá um aviso só (não uma falha por jogo).
#[test]
fn backup_de_varios_com_o_cofre_fora_avisa_uma_vez() {
    let w = mundo();
    let mut svc = pronto(&w);
    let (a, _) = jogo(&mut svc, &w, "JogoA", &[]);
    let (b, _) = jogo(&mut svc, &w, "JogoB", &[]);
    std::fs::rename(w.exe.join("cofre"), w.t.path().join("cofre-no-hd-desligado")).unwrap();
    let err = svc.backup_many(&[a, b]).unwrap_err();
    assert_eq!(err.kind, ErrorKind::CofreIndisponivel);
}
