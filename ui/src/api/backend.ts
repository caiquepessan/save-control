// O que a interface pede ao app. Dentro do Tauri vira `invoke`; no navegador
// (modo dev) há uma versão de demonstração em ./demo.ts.
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { ActivityView } from "./gerado/ActivityView";
import type { AddGameRequest } from "./gerado/AddGameRequest";
import type { AppStateView } from "./gerado/AppStateView";
import type { AutoBackupView } from "./gerado/AutoBackupView";
import type { AutoStatusView } from "./gerado/AutoStatusView";
import type { AutoTimesView } from "./gerado/AutoTimesView";
import type { BackupManyView } from "./gerado/BackupManyView";
import type { BackupResultView } from "./gerado/BackupResultView";
import type { ErrorKind } from "./gerado/ErrorKind";
import type { ErrorView } from "./gerado/ErrorView";
import type { GameView } from "./gerado/GameView";
import type { LibraryView } from "./gerado/LibraryView";
import type { ManifestUpdateView } from "./gerado/ManifestUpdateView";
import type { RestorePreviewView } from "./gerado/RestorePreviewView";
import type { RestoreRequest } from "./gerado/RestoreRequest";
import type { RestoreResultView } from "./gerado/RestoreResultView";
import type { RetentionView } from "./gerado/RetentionView";
import type { SettingsView } from "./gerado/SettingsView";
import type { SnapshotFileView } from "./gerado/SnapshotFileView";
import type { SnapshotView } from "./gerado/SnapshotView";
import type { VaultProbeView } from "./gerado/VaultProbeView";

export interface Janela {
  minimizar(): Promise<void>;
  alternarMaximizar(): Promise<void>;
  fechar(): Promise<void>;
  estaMaximizada(): Promise<boolean>;
}

export interface Backend {
  estado(): Promise<AppStateView>;
  biblioteca(): Promise<LibraryView>;
  escanear(): Promise<LibraryView>;
  fazerBackup(id: string, nome?: string | null): Promise<BackupResultView>;
  backupDeVarios(ids: string[]): Promise<BackupManyView>;
  jogosParaBackup(): Promise<string[]>;
  historico(id: string): Promise<SnapshotView[]>;
  arquivosDoBackup(id: string, arquivo: string): Promise<SnapshotFileView[]>;
  previaRestauracao(id: string, arquivo: string, destinos?: Record<string, string>): Promise<RestorePreviewView>;
  restaurar(id: string, arquivo: string, pedido: RestoreRequest): Promise<RestoreResultView>;
  abrirPasta(caminho: string): Promise<void>;
  escolherPasta(titulo: string): Promise<string | null>;
  escolherImagem(): Promise<string | null>;
  adicionarJogo(pedido: AddGameRequest): Promise<GameView>;
  renomearJogo(id: string, titulo: string | null): Promise<void>;
  ignorarJogo(id: string, ignorado: boolean): Promise<void>;
  definirPastaDoJogo(id: string, pasta: string | null): Promise<void>;
  definirCapa(id: string, arquivo: string | null): Promise<void>;
  esquecerDestino(id: string, template: string): Promise<void>;
  removerJogoManual(id: string): Promise<void>;
  configuracoes(): Promise<SettingsView>;
  definirPastasDeJogos(pastas: string[]): Promise<void>;
  definirIncluirConfig(ligado: boolean): Promise<void>;
  definirCapasSteam(ligado: boolean): Promise<void>;
  trocarCofre(pasta: string, criar: boolean): Promise<AppStateView>;
  inspecionarCofre(pasta: string): Promise<VaultProbeView>;
  tentarCofreDeNovo(): Promise<AppStateView>;
  atualizarBase(): Promise<ManifestUpdateView>;
  concluirPrimeiroUso(pastaDoCofre: string): Promise<AppStateView>;
  jogosAbertos(): Promise<string[]>;
  situacaoAutomatico(): Promise<AutoStatusView>;
  definirAutomatico(ligado: boolean): Promise<void>;
  /** Pausa o backup automático; `null` = até reabrir o app. */
  pausarAutomatico(minutos: number | null): Promise<void>;
  retomarAutomatico(): Promise<void>;
  definirAutomaticoDoJogo(id: string, ligado: boolean): Promise<void>;
  definirRetencao(politica: RetentionView): Promise<void>;
  /** `null` = o jogo volta a usar o padrão. */
  definirRetencaoDoJogo(id: string, politica: RetentionView | null): Promise<void>;
  definirTemposDoAutomatico(tempos: AutoTimesView): Promise<void>;
  /** `null` = o jogo segue o ajuste geral. */
  definirIncluirConfigDoJogo(id: string, valor: boolean | null): Promise<void>;
  fixarBackup(id: string, arquivo: string, fixado: boolean): Promise<SnapshotView>;
  apagarBackup(id: string, arquivo: string): Promise<void>;
  atividade(limite?: number): Promise<ActivityView[]>;
  definirFecharParaBandeja(ligado: boolean): Promise<void>;
  definirIniciarComWindows(ligado: boolean): Promise<void>;
  definirNotificacoes(ligado: boolean): Promise<void>;
  removerRegistrosDoWindows(): Promise<void>;
  /** O jogo que esta janela deve abrir (ela foi aberta pela notificação), uma vez só. */
  jogoParaAbrir(): Promise<string | null>;
  /** Capa da Steam baixada em segundo plano. Devolve a função que para de ouvir. */
  aoReceberCapa(cb: (id: string, caminho: string) => void): () => void;
  /** O "Sair" está esperando um backup ou uma restauração terminar. */
  aoBloquearFechar(cb: () => void): () => void;
  /** A situação do backup automático mudou (ligado, pausado, cofre fora…). */
  aoMudarSituacao(cb: (s: AutoStatusView) => void): () => void;
  /** O app mudou a biblioteca por conta própria (backup automático, save novo). */
  aoMudarBiblioteca(cb: () => void): () => void;
  /** Backups automáticos feitos sem notificação do Windows. */
  aoFazerBackupAutomatico(cb: (itens: AutoBackupView[]) => void): () => void;
  /** Clique na notificação ("Ver histórico"). */
  aoAbrirJogo(cb: (id: string) => void): () => void;
  /** Aviso do app (ex.: o cofre voltou). */
  aoAvisar(cb: (texto: string) => void): () => void;
  /** Endereço para mostrar uma imagem local num <img>. */
  srcDaImagem(caminho: string): string;
  janela: Janela;
}

/** Erro vindo do app, já com a mensagem em pt-BR. */
export class ErroApp extends Error {
  kind: ErrorKind;
  root: string | null;
  alias: string | null;
  candidates: string[];

  constructor(v: ErrorView) {
    super(v.message);
    this.name = "ErroApp";
    this.kind = v.kind;
    this.root = v.root;
    this.alias = v.alias ?? null;
    this.candidates = v.candidates;
  }

  static de(e: unknown): ErroApp {
    if (e instanceof ErroApp) return e;
    if (e && typeof e === "object" && "kind" in e && "message" in e) return new ErroApp(e as ErrorView);
    const message = e instanceof Error ? e.message : String(e);
    return new ErroApp({ kind: "erro", message, root: null, alias: null, candidates: [] });
  }
}

async function chamar<R>(comando: string, args?: Record<string, unknown>): Promise<R> {
  try {
    return await invoke<R>(comando, args);
  } catch (e) {
    throw ErroApp.de(e);
  }
}

function ouvir<P>(evento: string, cb: (p: P) => void): () => void {
  const parar = listen<P>(evento, (e) => cb(e.payload));
  return () => {
    void parar.then((f) => f());
  };
}

export const tauriBackend: Backend = {
  estado: () => chamar("estado"),
  biblioteca: () => chamar("biblioteca"),
  escanear: () => chamar("escanear"),
  fazerBackup: (id, nome) => chamar("fazer_backup", { id, nome: nome ?? null }),
  backupDeVarios: (ids) => chamar("backup_de_varios", { ids }),
  jogosParaBackup: () => chamar("jogos_para_backup"),
  historico: (id) => chamar("historico", { id }),
  arquivosDoBackup: (id, arquivo) => chamar("arquivos_do_backup", { id, arquivo }),
  previaRestauracao: (id, arquivo, destinos) => chamar("previa_restauracao", { id, arquivo, destinos: destinos ?? {} }),
  restaurar: (id, arquivo, pedido) => chamar("restaurar", { id, arquivo, pedido }),
  abrirPasta: (caminho) => chamar("abrir_pasta", { caminho }),
  escolherPasta: (titulo) => chamar("escolher_pasta", { titulo }),
  escolherImagem: () => chamar("escolher_imagem"),
  adicionarJogo: (pedido) => chamar("adicionar_jogo", { pedido }),
  renomearJogo: (id, titulo) => chamar("renomear_jogo", { id, titulo }),
  ignorarJogo: (id, ignorado) => chamar("ignorar_jogo", { id, ignorado }),
  definirPastaDoJogo: (id, pasta) => chamar("definir_pasta_do_jogo", { id, pasta }),
  definirCapa: (id, arquivo) => chamar("definir_capa", { id, arquivo }),
  esquecerDestino: (id, template) => chamar("esquecer_destino", { id, template }),
  removerJogoManual: (id) => chamar("remover_jogo_manual", { id }),
  configuracoes: () => chamar("configuracoes"),
  definirPastasDeJogos: (pastas) => chamar("definir_pastas_de_jogos", { pastas }),
  definirIncluirConfig: (ligado) => chamar("definir_incluir_config", { ligado }),
  definirCapasSteam: (ligado) => chamar("definir_capas_steam", { ligado }),
  trocarCofre: (pasta, criar) => chamar("trocar_cofre", { pasta, criar }),
  inspecionarCofre: (pasta) => chamar("inspecionar_cofre", { pasta }),
  tentarCofreDeNovo: () => chamar("tentar_cofre_de_novo"),
  atualizarBase: () => chamar("atualizar_base"),
  concluirPrimeiroUso: (pastaDoCofre) => chamar("concluir_primeiro_uso", { pastaDoCofre }),
  jogosAbertos: () => chamar("jogos_abertos"),
  situacaoAutomatico: () => chamar("situacao_automatico"),
  definirAutomatico: (ligado) => chamar("definir_automatico", { ligado }),
  pausarAutomatico: (minutos) => chamar("pausar_automatico", { minutos }),
  retomarAutomatico: () => chamar("retomar_automatico"),
  definirAutomaticoDoJogo: (id, ligado) => chamar("definir_automatico_do_jogo", { id, ligado }),
  definirRetencao: (politica) => chamar("definir_retencao", { politica }),
  definirRetencaoDoJogo: (id, politica) => chamar("definir_retencao_do_jogo", { id, politica }),
  definirTemposDoAutomatico: (tempos) => chamar("definir_tempos_do_automatico", { tempos }),
  definirIncluirConfigDoJogo: (id, valor) => chamar("definir_incluir_config_do_jogo", { id, valor }),
  fixarBackup: (id, arquivo, fixado) => chamar("fixar_backup", { id, arquivo, fixado }),
  apagarBackup: (id, arquivo) => chamar("apagar_backup", { id, arquivo }),
  atividade: (limite) => chamar("atividade", { limite: limite ?? null }),
  definirFecharParaBandeja: (ligado) => chamar("definir_fechar_para_bandeja", { ligado }),
  definirIniciarComWindows: (ligado) => chamar("definir_iniciar_com_windows", { ligado }),
  definirNotificacoes: (ligado) => chamar("definir_notificacoes", { ligado }),
  removerRegistrosDoWindows: () => chamar("remover_registros_do_windows"),
  jogoParaAbrir: () => chamar("jogo_para_abrir"),
  aoReceberCapa: (cb) => ouvir<{ id: string; caminho: string }>("capa", (p) => cb(p.id, p.caminho)),
  aoBloquearFechar: (cb) => ouvir<null>("fechar-bloqueado", () => cb()),
  aoMudarSituacao: (cb) => ouvir<AutoStatusView>("situacao", cb),
  aoMudarBiblioteca: (cb) => ouvir<null>("biblioteca-mudou", () => cb()),
  aoFazerBackupAutomatico: (cb) => ouvir<{ itens: AutoBackupView[] }>("backup-automatico", (p) => cb(p.itens)),
  aoAbrirJogo: (cb) => ouvir<{ id: string }>("abrir-jogo", (p) => cb(p.id)),
  aoAvisar: (cb) => ouvir<string>("aviso", cb),
  srcDaImagem: (caminho) => convertFileSrc(caminho),
  janela: {
    minimizar: () => getCurrentWindow().minimize(),
    alternarMaximizar: () => getCurrentWindow().toggleMaximize(),
    fechar: () => getCurrentWindow().close(),
    estaMaximizada: () => getCurrentWindow().isMaximized(),
  },
};
