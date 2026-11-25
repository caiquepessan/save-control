import { act, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { IDS, createDemoBackend } from "../api/demo";
import type { AutoBackupView } from "../api/gerado/AutoBackupView";
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
    expect(itens).toEqual(["Biblioteca", "Atividade", "Configurações"]);
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

describe("o que o app conta à janela", () => {
  const feito = (id: string, title: string): AutoBackupView => ({
    id,
    title,
    outcome: "feito",
    fileCount: 2,
    totalSize: 2048,
    cover: null,
  });

  async function pronto() {
    const backend = createDemoBackend({ atraso: 0 });
    const r = renderApp(backend);
    await screen.findByRole("article", { name: "Academia Arcana" });
    return { ...r, backend };
  }

  it("backup automático feito vira um aviso com os títulos", async () => {
    const { backend } = await pronto();
    act(() => backend.emitirBackupAutomatico([feito(IDS.academia, "Academia Arcana"), feito(IDS.vale, "Vale das Sementes")]));
    expect(await screen.findByText("Backup automático feito · Academia Arcana, Vale das Sementes")).toBeInTheDocument();
  });

  it("clicar na notificação abre o jogo no Histórico", async () => {
    const { backend, store } = await pronto();
    act(() => store.getState().irPara("configuracoes"));
    act(() => backend.emitirAbrirJogo(IDS.academia));
    const painel = await screen.findByRole("dialog", { name: "Academia Arcana" });
    expect(within(painel).getByRole("tab", { name: /Histórico/ })).toHaveAttribute("aria-selected", "true");
    expect(store.getState().tela).toBe("biblioteca");
  });

  it("a janela aberta pela notificação já abre o jogo", async () => {
    const backend = createDemoBackend({ atraso: 0 });
    vi.spyOn(backend, "jogoParaAbrir").mockResolvedValue(IDS.vale);
    renderApp(backend);
    const painel = await screen.findByRole("dialog", { name: "Vale das Sementes" });
    expect(within(painel).getByRole("tab", { name: /Histórico/ })).toHaveAttribute("aria-selected", "true");
  });

  it("a biblioteca mudou no app: a janela recarrega", async () => {
    const backend = createDemoBackend({ atraso: 0 });
    const biblioteca = vi.spyOn(backend, "biblioteca");
    renderApp(backend);
    await screen.findByRole("article", { name: "Academia Arcana" });
    const antes = biblioteca.mock.calls.length;
    act(() => backend.emitirBibliotecaMudou());
    await waitFor(() => expect(biblioteca).toHaveBeenCalledTimes(antes + 1));
  });

  it("aviso do app vira um aviso na janela", async () => {
    const { backend } = await pronto();
    act(() => backend.emitirAviso("O cofre voltou. O backup automático continua de onde parou."));
    expect(await screen.findByText("O cofre voltou. O backup automático continua de onde parou.")).toBeInTheDocument();
  });
});
