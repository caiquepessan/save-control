import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { IDS, createDemoBackend } from "../../api/demo";
import { renderApp } from "../../test/render";

async function abrirPainel(titulo: string) {
  await screen.findByRole("heading", { name: "Seus jogos" }, { timeout: 3000 });
  const cartao = screen.getByRole("article", { name: titulo });
  await userEvent.click(within(cartao).getByRole("button", { name: `Abrir ${titulo}` }));
  return screen.findByRole("dialog", { name: titulo });
}

describe("fazer backup", () => {
  it("o popover manda o nome", async () => {
    const backend = createDemoBackend({ atraso: 0 });
    const spy = vi.spyOn(backend, "fazerBackup");
    renderApp(backend);
    const painel = await abrirPainel("Academia Arcana");
    await userEvent.click(within(painel).getByRole("button", { name: "Fazer backup" }));
    const pop = await screen.findByRole("dialog", { name: "Fazer backup" });
    await userEvent.type(within(pop).getByLabelText("Nome (opcional)"), "antes de atualizar");
    await userEvent.click(within(pop).getByRole("button", { name: "Fazer backup" }));
    expect(spy).toHaveBeenCalledWith(IDS.academia, "antes de atualizar");
    expect(await screen.findByText("Backup feito · Academia Arcana")).toBeInTheDocument();
  });

  it("Enter confirma, e sem nome vai vazio", async () => {
    const backend = createDemoBackend({ atraso: 0 });
    const spy = vi.spyOn(backend, "fazerBackup");
    renderApp(backend);
    const painel = await abrirPainel("Academia Arcana");
    await userEvent.click(within(painel).getByRole("button", { name: "Fazer backup" }));
    const pop = await screen.findByRole("dialog", { name: "Fazer backup" });
    await userEvent.type(within(pop).getByLabelText("Nome (opcional)"), "{Enter}");
    expect(spy).toHaveBeenCalledWith(IDS.academia, null);
  });

  it("clique duplo faz um backup só", async () => {
    const backend = createDemoBackend({ atraso: 40 });
    const spy = vi.spyOn(backend, "fazerBackup");
    renderApp(backend);
    await screen.findByRole("heading", { name: "Seus jogos" }, { timeout: 3000 });
    const cartao = screen.getByRole("article", { name: "Academia Arcana" });
    await userEvent.dblClick(within(cartao).getByRole("button", { name: "Fazer backup" }));
    await screen.findByText("Backup feito · Academia Arcana", undefined, { timeout: 3000 });
    expect(spy).toHaveBeenCalledTimes(1);
  });

  it("o botão fica travado enquanto o backup roda", async () => {
    const backend = createDemoBackend({ atraso: 40 });
    renderApp(backend);
    const painel = await abrirPainel("Academia Arcana");
    const botao = within(painel).getByRole("button", { name: "Fazer backup" });
    await userEvent.click(botao);
    const pop = await screen.findByRole("dialog", { name: "Fazer backup" });
    await userEvent.click(within(pop).getByRole("button", { name: "Fazer backup" }));
    await waitFor(() => expect(botao).toBeDisabled());
    await waitFor(() => expect(botao).toBeEnabled(), { timeout: 3000 });
  });
});
