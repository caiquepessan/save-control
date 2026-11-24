import { render } from "@testing-library/react";
import { App } from "../App";
import type { Backend } from "../api/backend";
import { createDemoBackend } from "../api/demo";
import { AppProvider, createAppStore } from "../store/app";

/** Renderiza o app inteiro com um backend (o de demonstração, sem espera, por padrão). */
export function renderApp(backend: Backend = createDemoBackend({ atraso: 0 })) {
  const store = createAppStore(backend);
  const r = render(
    <AppProvider store={store}>
      <App />
    </AppProvider>,
  );
  return { ...r, store, backend };
}
