import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { createDemoBackend } from "../../api/demo";
import { renderApp } from "../../test/render";

async function abrir(titulo: string) {
  await screen.findByRole("heading", { name: "Seus jogos" });
  const cartao = screen.getByRole("article", { name: titulo });
  const botao = within(cartao).getByRole("button", { name: `Abrir ${titulo}` });
  await userEvent.click(botao);
  const painel = await screen.findByRole("dialog", { name: titulo });
  return { cartao, botao, painel };
}

describe("painel lateral", () => {
  it("abre com o título, a origem e os chips", async () => {
    renderApp();
    const { painel } = await abrir("Reino Esmeralda: A Flauta do Tempo");
    expect(within(painel).getByRole("heading", { name: "Reino Esmeralda: A Flauta do Tempo" })).toBeInTheDocument();
    expect(within(painel).getByText("Recomp · Reino64: Recompiled")).toBeInTheDocument();
    expect(await within(painel).findByText("Jogando agora")).toBeInTheDocument();
    expect(within(painel).getByText("7 arquivos")).toBeInTheDocument();
    expect(within(painel).getByText("1,2 MB")).toBeInTheDocument();
  });

  it("Esc fecha e devolve o foco ao cartão", async () => {
    renderApp();
    const { botao } = await abrir("Academia Arcana");
    await userEvent.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(botao).toHaveFocus();
  });

  it("clicar fora fecha", async () => {
    renderApp();
    await abrir("Academia Arcana");
    await userEvent.click(screen.getByTestId("fundo-painel"));
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  });

  it("Restaurar… do cartão abre no Histórico com a faixa de escolher", async () => {
    renderApp();
    await screen.findByRole("heading", { name: "Seus jogos" });
    const cartao = screen.getByRole("article", { name: "Academia Arcana" });
    await userEvent.click(within(cartao).getByRole("button", { name: "Restaurar…" }));
    const painel = await screen.findByRole("dialog", { name: "Academia Arcana" });
    expect(within(painel).getByRole("tab", { name: /Histórico/ })).toHaveAttribute("aria-selected", "true");
    expect(within(painel).getByText("Escolha a versão que você quer restaurar.")).toBeInTheDocument();
  });

  it("histórico: save atual, versão com nome (estrela) e o primeiro backup", async () => {
    renderApp();
    const { painel } = await abrir("Reino Esmeralda: A Flauta do Tempo");
    const itens = await within(painel).findAllByRole("listitem");
    expect(itens).toHaveLength(6);
    expect(itens[0]).toHaveTextContent("Save atual");
    expect(itens[0]).toHaveTextContent("ainda não está no cofre");
    expect(itens[0]).toHaveTextContent("Novo");
    expect(itens[1]).toHaveTextContent("Antes de atualizar o recomp");
    expect(itens[1]).toHaveClass("pin");
    expect(itens[1]).toHaveTextContent(/^.*manual · .*/);
    expect(itens[2]).toHaveTextContent("Automático");
    expect(itens[5]).toHaveTextContent("Primeiro backup");
    expect(within(itens[1]).getByRole("button", { name: "Restaurar" })).toBeInTheDocument();
  });

  it("backup de outro PC mostra de qual PC veio", async () => {
    renderApp();
    const { painel } = await abrir("Academia Arcana");
    const itens = await within(painel).findAllByRole("listitem");
    expect(itens[2]).toHaveTextContent("de NOTEBOOK-SALA");
    expect(itens[1]).not.toHaveTextContent("de ");
  });

  it("aviso de pasta arriscada com o texto exato", async () => {
    renderApp();
    const { painel } = await abrir("Ação & Reação: Edição Especial do Colecionador com Todos os Extras Inclusos");
    expect(within(painel).getByRole("note")).toHaveTextContent(
      "Este save fica dentro de Downloads. Se essa pasta for limpa, o progresso some. O cofre já guarda 2 cópias.",
    );
  });

  it("Abrir pasta abre a pasta do save", async () => {
    const backend = createDemoBackend({ atraso: 0 });
    const spy = vi.spyOn(backend, "abrirPasta");
    renderApp(backend);
    const { painel } = await abrir("Academia Arcana");
    await userEvent.click(within(painel).getByRole("button", { name: "Abrir pasta" }));
    expect(spy).toHaveBeenCalledWith("C:\\Users\\voce\\AppData\\Local\\AcademiaArcana\\Saved\\SaveGames\\7656");
  });

  it("as abas trocam pelo clique e pelas setas", async () => {
    renderApp();
    const { painel } = await abrir("Cavaleiro das Cavernas");
    const historico = within(painel).getByRole("tab", { name: /Histórico/ });
    await userEvent.click(within(painel).getByRole("tab", { name: /Arquivos/ }));
    expect(within(painel).getByRole("tab", { name: /Arquivos/ })).toHaveAttribute("aria-selected", "true");
    expect(within(painel).getByText("user1.dat")).toBeInTheDocument();
    historico.focus();
    await userEvent.click(historico);
    await userEvent.keyboard("{ArrowRight}");
    expect(within(painel).getByRole("tab", { name: /Arquivos/ })).toHaveAttribute("aria-selected", "true");
    await userEvent.keyboard("{ArrowRight}");
    expect(within(painel).getByRole("tab", { name: "Ajustes" })).toHaveAttribute("aria-selected", "true");
  });
});
