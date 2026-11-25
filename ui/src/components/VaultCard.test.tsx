import { act, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { createDemoBackend } from "../api/demo";
import type { AutoStatusView } from "../api/gerado/AutoStatusView";
import { renderApp } from "../test/render";

const situacao = (s: Partial<AutoStatusView>): AutoStatusView => ({
  state: "ativo",
  pausedUntil: null,
  watching: 11,
  busy: false,
  lastBackup: null,
  ...s,
});

async function cartao() {
  const c = await screen.findByTestId("cartao-cofre");
  const chave = await within(c).findByRole("switch", { name: "Backup automático" });
  return { c, chave };
}

describe("cartão do cofre: backup automático", () => {
  it("mostra quantos jogos vigia, e a chave desliga o automático", async () => {
    const backend = createDemoBackend({ atraso: 0 });
    const definir = vi.spyOn(backend, "definirAutomatico");
    renderApp(backend);
    const { c, chave } = await cartao();
    expect(chave).toBeChecked();
    expect(within(c).getByText("Vigiando 11 jogos")).toBeInTheDocument();
    await userEvent.click(chave);
    expect(definir).toHaveBeenCalledWith(false);
    await waitFor(() => expect(chave).not.toBeChecked());
    expect(within(c).getByText("Desligado")).toBeInTheDocument();
  });

  it("pausado mostra até quando, e o Retomar chama o app", async () => {
    const backend = createDemoBackend({ atraso: 0 });
    const retomar = vi.spyOn(backend, "retomarAutomatico");
    renderApp(backend);
    const { c, chave } = await cartao();
    act(() => backend.emitirSituacao(situacao({ state: "pausado", pausedUntil: new Date(2026, 10, 25, 14, 30).toISOString() })));
    expect(await within(c).findByText("Pausado até 14:30")).toBeInTheDocument();
    expect(chave).toBeChecked();
    await userEvent.click(within(c).getByRole("button", { name: "Retomar" }));
    expect(retomar).toHaveBeenCalledTimes(1);
    expect(await within(c).findByText("Vigiando 11 jogos")).toBeInTheDocument();
  });

  it("pausado até reabrir o app", async () => {
    const backend = createDemoBackend({ atraso: 0 });
    renderApp(backend);
    const { c } = await cartao();
    act(() => backend.emitirSituacao(situacao({ state: "pausado" })));
    expect(await within(c).findByText("Pausado até reabrir")).toBeInTheDocument();
  });

  it("com o cofre indisponível, a chave fica desabilitada", async () => {
    const backend = createDemoBackend({ atraso: 0 });
    renderApp(backend);
    const { c, chave } = await cartao();
    act(() => backend.emitirSituacao(situacao({ state: "cofreFora" })));
    expect(await within(c).findByText("Pausado até o cofre voltar")).toBeInTheDocument();
    expect(chave).toBeDisabled();
  });
});
