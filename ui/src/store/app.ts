// Estado da interface (Zustand). O backend entra por injeção: o app usa o do
// Tauri; o navegador e os testes usam o de demonstração.
import { createContext, createElement, useContext, type ReactNode } from "react";
import { createStore, useStore } from "zustand";
import { ErroApp, type Backend } from "../api/backend";
import type { AddGameRequest } from "../api/gerado/AddGameRequest";
import type { AppStateView } from "../api/gerado/AppStateView";
import type { GameView } from "../api/gerado/GameView";
import type { LibraryView } from "../api/gerado/LibraryView";
import type { RestoreResultView } from "../api/gerado/RestoreResultView";
import type { SnapshotView } from "../api/gerado/SnapshotView";
import { FILTRO_INICIAL, type Filtro } from "../lib/library";
import { T } from "../texto";

export type Tela = "biblioteca" | "configuracoes";
export type Aba = "historico" | "arquivos" | "ajustes";

export interface Toast {
  id: number;
  tipo: "ok" | "erro" | "info";
  texto: string;
  /** Erros ficam até a pessoa fechar. */
  fixo: boolean;
}

export interface Painel {
  id: string;
  aba: Aba;
  /** Aberto pelo "Restaurar…": mostra a faixa "escolha a versão". */
  modoRestaurar: boolean;
}

export type Dialogo = { tipo: "adicionar" } | { tipo: "renomear"; id: string } | { tipo: "remover"; id: string };

/** Uma restauração pedida (a confirmação é desenhada a partir disto). */
export interface PedidoRestauracao {
  id: string;
  arquivo: string;
  /** Só estes arquivos (restauração seletiva). */
  soArquivos?: { root: string; relPath: string }[];
}

export interface AppState {
  backend: Backend;
  estado: AppStateView | null;
  erroInicial: string | null;
  tela: Tela;
  biblioteca: LibraryView | null;
  filtro: Filtro;
  escaneando: boolean;
  backupGeral: boolean;
  abertos: string[];
  pendentes: Record<string, "backup" | "restauracao">;
  toasts: Toast[];
  painel: Painel | null;
  /** Diálogo aberto por cima de tudo (adicionar, renomear, remover). */
  dialogo: Dialogo | null;
  /** Histórico de cada jogo já carregado (do mais novo ao mais velho). */
  historicos: Record<string, SnapshotView[]>;
  restauracao: PedidoRestauracao | null;
  /** Escolha de arquivos de uma versão (restauração seletiva). */
  selecaoArquivos: { id: string } | null;

  carregarEstado(): Promise<void>;
  carregarBiblioteca(): Promise<void>;
  irPara(tela: Tela): void;
  definirFiltro(f: Partial<Filtro>): void;
  escanear(): Promise<void>;
  fazerBackup(id: string, nome?: string | null): Promise<void>;
  backupDeVarios(ids: string[]): Promise<void>;
  backupDeTudo(): Promise<void>;
  atualizarAbertos(): Promise<void>;
  aplicarCapa(id: string, caminho: string): void;
  abrirPainel(id: string, opcoes?: { aba?: Aba; modoRestaurar?: boolean }): void;
  fecharPainel(): void;
  definirAba(aba: Aba): void;
  carregarHistorico(id: string): Promise<void>;
  iniciarRestauracao(id: string, arquivo: string, soArquivos?: { root: string; relPath: string }[]): void;
  cancelarRestauracao(): void;
  marcarPendente(id: string, acao: "backup" | "restauracao" | null): void;
  aposRestaurar(id: string, r: RestoreResultView): Promise<void>;
  abrirSelecaoArquivos(id: string): void;
  fecharSelecaoArquivos(): void;
  abrirDialogo(d: Dialogo): void;
  fecharDialogo(): void;
  /** Devolve a mensagem de erro (para o diálogo mostrar) ou null se deu certo. */
  adicionarJogo(pedido: AddGameRequest): Promise<string | null>;
  renomear(id: string, titulo: string | null): Promise<void>;
  ignorar(id: string): Promise<void>;
  remover(id: string): Promise<void>;
  trocarCapa(id: string): Promise<void>;
  definirPastaDoJogo(id: string, pasta: string | null): Promise<void>;
  esquecerDestino(id: string, template: string): Promise<void>;
  avisar(tipo: Toast["tipo"], texto: string): void;
  fecharToast(id: number): void;
}

let proximoToast = 1;

export function createAppStore(backend: Backend) {
  return createStore<AppState>()((set, get) => {
    /** Backup manual de vários jogos, um lote por vez (o botão fica travado). */
    async function emLote(quais: () => Promise<string[]>) {
      if (get().backupGeral) return;
      set({ backupGeral: true });
      try {
        const ids = await quais();
        if (ids.length === 0) {
          get().avisar("info", T.backup.nadaParaFazer);
          return;
        }
        const r = await backend.backupDeVarios(ids);
        if (r.failed.length) get().avisar("erro", T.backup.falhas(r.done.length, r.failed.map((f) => f.title)));
        else if (r.done.length) get().avisar("ok", T.backup.varios(r.done.length));
        else get().avisar("info", T.backup.nadaParaFazer);
        await get().carregarBiblioteca();
      } catch (e) {
        get().avisar("erro", ErroApp.de(e).message);
      } finally {
        set({ backupGeral: false });
      }
    }

    /** Ajuste de um jogo: chama o app e recarrega a biblioteca; erro vira aviso. */
    async function acaoNoJogo(f: () => Promise<void>): Promise<boolean> {
      try {
        await f();
        await get().carregarBiblioteca();
        return true;
      } catch (e) {
        get().avisar("erro", ErroApp.de(e).message);
        return false;
      }
    }

    return {
    backend,
    estado: null,
    erroInicial: null,
    tela: "biblioteca",
    biblioteca: null,
    filtro: FILTRO_INICIAL,
    escaneando: false,
    backupGeral: false,
    abertos: [],
    pendentes: {},
    toasts: [],
    painel: null,
    dialogo: null,
    historicos: {},
    restauracao: null,
    selecaoArquivos: null,

    async carregarEstado() {
      try {
        const estado = await backend.estado();
        set({ estado, erroInicial: null });
        if (estado.phase === "pronto") {
          await get().carregarBiblioteca();
          // Abre com o cache na hora e escaneia em seguida.
          void get().escanear();
        }
      } catch (e) {
        set({ erroInicial: ErroApp.de(e).message });
      }
    },

    async carregarBiblioteca() {
      try {
        set({ biblioteca: await backend.biblioteca() });
      } catch (e) {
        get().avisar("erro", ErroApp.de(e).message);
      }
    },

    irPara(tela) {
      set({ tela });
    },

    definirFiltro(f) {
      set({ filtro: { ...get().filtro, ...f } });
    },

    async escanear() {
      if (get().escaneando) return;
      set({ escaneando: true });
      try {
        set({ biblioteca: await backend.escanear() });
      } catch (e) {
        get().avisar("erro", ErroApp.de(e).message);
      } finally {
        set({ escaneando: false });
      }
    },

    async fazerBackup(id, nome) {
      // Um backup por vez para cada jogo: clique duplo não faz dois.
      if (get().pendentes[id]) return;
      set({ pendentes: { ...get().pendentes, [id]: "backup" } });
      const titulo = get().biblioteca?.games.find((g) => g.id === id)?.title ?? id;
      try {
        const r = await backend.fazerBackup(id, nome ?? null);
        if (r.outcome === "feito") get().avisar("ok", T.backup.feito(titulo));
        else if (r.outcome === "semMudanca") get().avisar("info", T.backup.semMudanca(titulo));
        else get().avisar("info", T.backup.semArquivos(titulo));
        await get().carregarBiblioteca();
        if (get().painel?.id === id) await get().carregarHistorico(id);
      } catch (e) {
        get().avisar("erro", ErroApp.de(e).message);
      } finally {
        const resto = { ...get().pendentes };
        delete resto[id];
        set({ pendentes: resto });
      }
    },

    async backupDeVarios(ids) {
      if (ids.length === 0) return;
      await emLote(async () => ids);
    },

    async backupDeTudo() {
      await emLote(() => backend.jogosParaBackup());
    },

    async atualizarAbertos() {
      try {
        const abertos = await backend.jogosAbertos();
        const atuais = get().abertos;
        if (abertos.length !== atuais.length || abertos.some((id, i) => id !== atuais[i])) set({ abertos });
      } catch {
        // Sem informação de processos: fica como estava.
      }
    },

    aplicarCapa(id, caminho) {
      const b = get().biblioteca;
      if (!b) return;
      const games = b.games.map((g: GameView) => (g.id === id ? { ...g, cover: { kind: "steam" as const, path: caminho } } : g));
      set({ biblioteca: { ...b, games } });
    },

    abrirPainel(id, opcoes = {}) {
      set({ painel: { id, aba: opcoes.aba ?? "historico", modoRestaurar: opcoes.modoRestaurar ?? false } });
    },

    fecharPainel() {
      set({ painel: null });
    },

    definirAba(aba) {
      const p = get().painel;
      if (p) set({ painel: { ...p, aba, modoRestaurar: aba === "historico" ? p.modoRestaurar : false } });
    },

    async carregarHistorico(id) {
      try {
        const lista = await backend.historico(id);
        set({ historicos: { ...get().historicos, [id]: lista } });
      } catch (e) {
        get().avisar("erro", ErroApp.de(e).message);
      }
    },

    iniciarRestauracao(id, arquivo, soArquivos) {
      set({ restauracao: { id, arquivo, soArquivos } });
    },

    cancelarRestauracao() {
      set({ restauracao: null });
    },

    marcarPendente(id, acao) {
      const pendentes = { ...get().pendentes };
      if (acao) pendentes[id] = acao;
      else delete pendentes[id];
      set({ pendentes });
    },

    async aposRestaurar(id, r) {
      get().avisar("ok", r.safety ? T.restaurar.feitoComSeguranca : T.restaurar.feito);
      await get().carregarBiblioteca();
      if (get().painel?.id === id) await get().carregarHistorico(id);
    },

    abrirSelecaoArquivos(id) {
      set({ selecaoArquivos: { id } });
    },

    fecharSelecaoArquivos() {
      set({ selecaoArquivos: null });
    },

    abrirDialogo(d) {
      set({ dialogo: d });
    },

    fecharDialogo() {
      set({ dialogo: null });
    },

    async adicionarJogo(pedido) {
      try {
        const novo = await backend.adicionarJogo(pedido);
        get().fecharDialogo();
        get().avisar("ok", T.adicionar.adicionado(novo.title));
        await get().carregarBiblioteca();
        return null;
      } catch (e) {
        return ErroApp.de(e).message;
      }
    },

    async renomear(id, titulo) {
      await acaoNoJogo(() => backend.renomearJogo(id, titulo));
    },

    async ignorar(id) {
      const titulo = get().biblioteca?.games.find((g) => g.id === id)?.title ?? id;
      const ok = await acaoNoJogo(() => backend.ignorarJogo(id, true));
      if (!ok) return;
      if (get().painel?.id === id) get().fecharPainel();
      get().avisar("info", T.ajustesJogo.ignorado(titulo));
    },

    async remover(id) {
      if (get().painel?.id === id) get().fecharPainel();
      await acaoNoJogo(() => backend.removerJogoManual(id));
    },

    async trocarCapa(id) {
      const arquivo = await backend.escolherImagem();
      if (arquivo) await acaoNoJogo(() => backend.definirCapa(id, arquivo));
    },

    async definirPastaDoJogo(id, pasta) {
      await acaoNoJogo(() => backend.definirPastaDoJogo(id, pasta));
    },

    async esquecerDestino(id, template) {
      await acaoNoJogo(() => backend.esquecerDestino(id, template));
    },

    avisar(tipo, texto) {
      const toast: Toast = { id: proximoToast++, tipo, texto, fixo: tipo === "erro" };
      set({ toasts: [...get().toasts, toast] });
    },

    fecharToast(id) {
      set({ toasts: get().toasts.filter((t) => t.id !== id) });
    },
    };
  });
}

export type AppStore = ReturnType<typeof createAppStore>;

const Contexto = createContext<AppStore | null>(null);

export function AppProvider({ store, children }: { store: AppStore; children: ReactNode }) {
  return createElement(Contexto.Provider, { value: store }, children);
}

export function useApp<T>(seletor: (s: AppState) => T): T {
  const store = useContext(Contexto);
  if (!store) throw new Error("useApp fora do AppProvider");
  return useStore(store, seletor);
}
