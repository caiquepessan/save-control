import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { createDemoBackend } from "../api/demo";
import { renderApp } from "../test/render";

describe("estrutura da janela", () => {
  it("barra de título com o nome e os botões da janela", async () => {
    const backend = createDemoBackend({ atraso: 0 });
    const min = vi.spyOn(backend.janela, "minimizar");
    const max = vi.spyOn(backend.janela, "alternarMaximizar");
    const fechar = vi.spyOn(backend.janela, "fechar");
    renderApp(backend);
    const barra = await screen.findByTestId("barra-titulo");
    expect(within(barra).getByText("Save Control")).toBeInTheDocument();
    await userEvent.click(within(barra).getByRole("button", { name: "Minimizar" }));
    await userEvent.click(within(barra).getByRole("button", { name: "Maximizar" }));
    await userEvent.click(within(barra).getByRole("button", { name: "Fechar" }));
    expect(min).toHaveBeenCalledTimes(1);
    expect(max).toHaveBeenCalledTimes(1);
    expect(fechar).toHaveBeenCalledTimes(1);
  });

  it("o menu lateral só tem o que já funciona", async () => {
    renderApp(createDemoBackend({ atraso: 0 }));
    const menu = await screen.findByRole("navigation", { name: "Menu" });
    const itens = within(menu).getAllByRole("button").map((b) => b.getAttribute("aria-label"));
    expect(itens).toEqual(["Biblioteca", "Configurações"]);
  });

  it("os avisos do app aparecem ao abrir e ficam até fechar", async () => {
    const backend = createDemoBackend({ atraso: 0 });
    const original = backend.estado;
    const aviso = "As configurações estavam danificadas e voltaram ao padrão.";
    vi.spyOn(backend, "estado").mockImplementation(async () => ({ ...(await original()), warnings: [aviso] }));
    const { store } = renderApp(backend);
    const toast = (await screen.findByText(aviso)).closest(".toast")!;
    expect(store.getState().toasts.filter((t) => t.texto === aviso)).toEqual([expect.objectContaining({ fixo: true })]);
    await userEvent.click(within(toast as HTMLElement).getByRole("button", { name: "Fechar aviso" }));
    expect(screen.queryByText(aviso)).not.toBeInTheDocument();
  });

  it("o cartão do cofre mostra a pasta e as versões guardadas", async () => {
    renderApp(createDemoBackend({ atraso: 0 }));
    const cartao = await screen.findByTestId("cartao-cofre");
    expect(await within(cartao).findByText(/versões guardadas/)).toHaveTextContent("1,4 GB · 212 versões guardadas");
    expect(within(cartao).getByTitle("D:\\Save Control\\cofre")).toBeInTheDocument();
  });
});
