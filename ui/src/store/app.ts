// Estado da interface (Zustand). O backend entra por injeção: o app usa o do
// Tauri; o navegador e os testes usam o de demonstração.
import { createContext, createElement, useContext, type ReactNode } from "react";
import { createStore, useStore } from "zustand";
import { ErroApp, type Backend } from "../api/backend";
import type { AppStateView } from "../api/gerado/AppStateView";
import type { LibraryView } from "../api/gerado/LibraryView";

export type Tela = "biblioteca" | "configuracoes";

export interface AppState {
  backend: Backend;
  estado: AppStateView | null;
  /** Erro ao falar com o app logo na abertura. */
  erroInicial: string | null;
  tela: Tela;
  biblioteca: LibraryView | null;
  carregarEstado(): Promise<void>;
  carregarBiblioteca(): Promise<void>;
  irPara(tela: Tela): void;
}

export function createAppStore(backend: Backend) {
  return createStore<AppState>()((set, get) => ({
    backend,
    estado: null,
    erroInicial: null,
    tela: "biblioteca",
    biblioteca: null,
    async carregarEstado() {
      try {
        const estado = await backend.estado();
        set({ estado, erroInicial: null });
        if (estado.phase === "pronto") await get().carregarBiblioteca();
      } catch (e) {
        set({ erroInicial: ErroApp.de(e).message });
      }
    },
    async carregarBiblioteca() {
      const biblioteca = await backend.biblioteca();
      set({ biblioteca });
    },
    irPara(tela) {
      set({ tela });
    },
  }));
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
