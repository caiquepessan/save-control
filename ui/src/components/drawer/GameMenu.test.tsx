import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { IDS, createDemoBackend } from "../../api/demo";
import { renderApp } from "../../test/render";

async function abrirPainel(titulo: string) {
  await screen.findByRole("heading", { name: "Seus jogos" });
  const cartao = screen.getByRole("article", { name: titulo });
  await userEvent.click(within(cartao).getByRole("button", { name: `Abrir ${titulo}` }));
  return screen.findByRole("dialog", { name: titulo });
}

async function abrirMenu(painel: HTMLElement) {
  await userEvent.click(within(painel).getByRole("button", { name: "Mais ações" }));
  return within(painel).findByRole("menu");
}

describe("menu e ajustes do jogo", () => {
  it("renomear e voltar ao nome original", async () => {
    const backend = createDemoBackend({ atraso: 0 });
    const spy = vi.spyOn(backend, "renomearJogo");
    renderApp(backend);
    let painel = await abrirPainel("Academia Arcana");
    await userEvent.click(within(await abrirMenu(painel)).getByRole("menuitem", { name: "Renomear" }));
    const d = await screen.findByRole("dialog", { name: "Renomear jogo" });
    expect(within(d).queryByRole("button", { name: "Voltar ao nome original" })).toBeNull();
    const campo = within(d).getByLabelText("Nome");
    await userEvent.clear(campo);
    await userEvent.type(campo, "Novo nome");
    await userEvent.click(within(d).getByRole("button", { name: "Salvar" }));
    expect(spy).toHaveBeenCalledWith(IDS.academia, "Novo nome");
    painel = await screen.findByRole("dialog", { name: "Novo nome" });
    await userEvent.click(within(await abrirMenu(painel)).getByRole("menuitem", { name: "Renomear" }));
    const d2 = await screen.findByRole("dialog", { name: "Renomear jogo" });
    await userEvent.click(within(d2).getByRole("button", { name: "Voltar ao nome original" }));
    expect(spy).toHaveBeenLastCalledWith(IDS.academia, null);
  });

  it("ignorar some com o jogo e fecha o painel", async () => {
    const backend = createDemoBackend({ atraso: 0 });
    const spy = vi.spyOn(backend, "ignorarJogo");
    renderApp(backend);
    const painel = await abrirPainel("Corrida Pixel");
    await userEvent.click(within(await abrirMenu(painel)).getByRole("menuitem", { name: "Ignorar jogo" }));
    expect(spy).toHaveBeenCalledWith(IDS.corrida, true);
    await waitFor(() => expect(screen.queryByRole("dialog", { name: "Corrida Pixel" })).toBeNull());
    await waitFor(() => expect(screen.queryByRole("article", { name: "Corrida Pixel" })).toBeNull());
  });

  it("Remover jogo só aparece em jogos à mão e pede confirmação", async () => {
    const backend = createDemoBackend({ atraso: 0 });
    const spy = vi.spyOn(backend, "removerJogoManual");
    renderApp(backend);
    let painel = await abrirPainel("Academia Arcana");
    expect(within(await abrirMenu(painel)).queryByRole("menuitem", { name: "Remover jogo" })).toBeNull();
    await userEvent.keyboard("{Escape}");
    await userEvent.click(within(painel).getByRole("button", { name: "Fechar painel" }));

    const titulo = "Ação & Reação: Edição Especial do Colecionador com Todos os Extras Inclusos";
    painel = await abrirPainel(titulo);
    await userEvent.click(within(await abrirMenu(painel)).getByRole("menuitem", { name: "Remover jogo" }));
    const d = await screen.findByRole("dialog", { name: `Remover ${titulo}?` });
    expect(d).toHaveTextContent("Os backups continuam no cofre.");
    await userEvent.click(within(d).getByRole("button", { name: "Remover" }));
    expect(spy).toHaveBeenCalledWith(IDS.acao);
    await waitFor(() => expect(screen.queryByRole("article", { name: titulo })).toBeNull());
  });

  it("Trocar capa escolhe a imagem e grava", async () => {
    const backend = createDemoBackend({ atraso: 0 });
    const escolher = vi.spyOn(backend, "escolherImagem");
    const spy = vi.spyOn(backend, "definirCapa");
    renderApp(backend);
    const painel = await abrirPainel("Academia Arcana");
    await userEvent.click(within(await abrirMenu(painel)).getByRole("menuitem", { name: "Trocar capa" }));
    expect(escolher).toHaveBeenCalled();
    await waitFor(() => expect(spy).toHaveBeenCalledWith(IDS.academia, "C:\\Imagens\\capa.png"));
  });

  it("aba Ajustes: pasta do jogo e destinos lembrados", async () => {
    const backend = createDemoBackend({ atraso: 0 });
    const pasta = vi.spyOn(backend, "definirPastaDoJogo");
    const esquecer = vi.spyOn(backend, "esquecerDestino");
    renderApp(backend);
    const painel = await abrirPainel("Urso & Pássaro");
    await userEvent.click(within(painel).getByRole("tab", { name: "Ajustes" }));
    await userEvent.click(within(painel).getByRole("button", { name: "Escolher…" }));
    await waitFor(() => expect(pasta).toHaveBeenCalledWith(IDS.urso, "D:\\Jogos\\Pasta Escolhida"));
    await userEvent.click(within(painel).getByRole("button", { name: "Limpar" }));
    await waitFor(() => expect(pasta).toHaveBeenLastCalledWith(IDS.urso, null));
    expect(within(painel).getByText("<gameDir>/saves")).toBeInTheDocument();
    await userEvent.click(within(painel).getByRole("button", { name: "Esquecer" }));
    await waitFor(() => expect(esquecer).toHaveBeenCalledWith(IDS.urso, "<gameDir>/saves"));
  });

  it("o menu abre pelo teclado e as setas andam pelos itens", async () => {
    renderApp();
    const painel = await abrirPainel("Academia Arcana");
    within(painel).getByRole("button", { name: "Mais ações" }).focus();
    await userEvent.keyboard("{Enter}");
    const menu = await within(painel).findByRole("menu");
    const itens = within(menu).getAllByRole("menuitem");
    expect(itens[0]).toHaveFocus();
    await userEvent.keyboard("{ArrowDown}");
    expect(itens[1]).toHaveFocus();
    await userEvent.keyboard("{Escape}");
    expect(within(painel).queryByRole("menu")).toBeNull();
    expect(screen.getByRole("dialog", { name: "Academia Arcana" })).toBeInTheDocument();
  });
});
