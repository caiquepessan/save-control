import "@fontsource/inter/400.css";
import "@fontsource/inter/500.css";
import "@fontsource/inter/600.css";
import "@fontsource/inter/700.css";
import "@fontsource/inter/800.css";
import "@fontsource/sora/600.css";
import "@fontsource/sora/700.css";
import "@fontsource/sora/800.css";
import "./styles/tokens.css";
import "./styles/base.css";
import "./styles/app.css";
import "./styles/biblioteca.css";
import "./styles/painel.css";

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { tauriBackend, type Backend } from "./api/backend";
import type { AppPhase } from "./api/gerado/AppPhase";
import { AppProvider, createAppStore } from "./store/app";

/** No navegador em modo dev, a demonstração (com ?fase=primeiroUso etc.); no app, o Tauri. */
async function escolherBackend(): Promise<Backend> {
  if (import.meta.env.DEV && !("__TAURI_INTERNALS__" in window)) {
    const { createDemoBackend } = await import("./api/demo");
    const fase = (new URLSearchParams(location.search).get("fase") ?? "pronto") as AppPhase;
    return createDemoBackend({ fase });
  }
  return tauriBackend;
}

const store = createAppStore(await escolherBackend());
createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <AppProvider store={store}>
      <App />
    </AppProvider>
  </StrictMode>,
);
