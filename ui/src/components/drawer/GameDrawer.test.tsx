import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { IDS, createDemoBackend } from "../../api/demo";
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

describe("painel: automático do jogo, fixar e apagar", () => {
  async function ajustes(backend = createDemoBackend({ atraso: 0 })) {
    renderApp(backend);
    const { painel } = await abrir("Academia Arcana");
    await userEvent.click(within(painel).getByRole("tab", { name: "Ajustes" }));
    return painel;
  }

  it("a chave do jogo desliga o automático só dele", async () => {
    const backend = createDemoBackend({ atraso: 0 });
    const spy = vi.spyOn(backend, "definirAutomaticoDoJogo");
    const painel = await ajustes(backend);
    const chave = await within(painel).findByRole("switch", { name: "Backup automático deste jogo" });
    expect(chave).toBeChecked();
    await userEvent.click(chave);
    expect(spy).toHaveBeenCalledWith(IDS.academia, false);
    await waitFor(() => expect(chave).not.toBeChecked());
  });

  it("retenção personalizada: 3 últimos, 0 dias, 0 semanas", async () => {
    const backend = createDemoBackend({ atraso: 0 });
    const spy = vi.spyOn(backend, "definirRetencaoDoJogo");
    const painel = await ajustes(backend);
    expect(await within(painel).findByRole("radio", { name: "Usar o padrão (10 últimos · 7 dias · 4 semanas)" })).toBeChecked();
    await userEvent.click(within(painel).getByRole("radio", { name: "Personalizar" }));
    for (const [campo, valor] of [
      ["Últimos", "3"],
      ["Dias", "0"],
      ["Semanas", "0"],
    ]) {
      const entrada = within(painel).getByRole("spinbutton", { name: campo });
      await userEvent.clear(entrada);
      await userEvent.type(entrada, valor);
    }
    await userEvent.click(within(painel).getByRole("button", { name: "Salvar" }));
    expect(spy).toHaveBeenCalledWith(IDS.academia, { autoRecent: 3, autoDailyDays: 0, autoWeeklyWeeks: 0 });
  });

  it("retenção sem nenhum backup recente não salva", async () => {
    const backend = createDemoBackend({ atraso: 0 });
    const spy = vi.spyOn(backend, "definirRetencaoDoJogo");
    const painel = await ajustes(backend);
    await userEvent.click(await within(painel).findByRole("radio", { name: "Personalizar" }));
    const ultimos = within(painel).getByRole("spinbutton", { name: "Últimos" });
    await userEvent.clear(ultimos);
    await userEvent.type(ultimos, "0");
    expect(within(painel).getByRole("button", { name: "Salvar" })).toBeDisabled();
    expect(spy).not.toHaveBeenCalled();
  });

  it("voltar ao padrão limpa a retenção do jogo", async () => {
    const backend = createDemoBackend({ atraso: 0 });
    await backend.definirRetencaoDoJogo(IDS.academia, { autoRecent: 3, autoDailyDays: 0, autoWeeklyWeeks: 0 });
    const spy = vi.spyOn(backend, "definirRetencaoDoJogo");
    const painel = await ajustes(backend);
    expect(await within(painel).findByRole("radio", { name: "Personalizar" })).toBeChecked();
    await userEvent.click(within(painel).getByRole("radio", { name: /^Usar o padrão/ }));
    expect(spy).toHaveBeenCalledWith(IDS.academia, null);
  });

  it("arquivos de configuração: Incluir", async () => {
    const backend = createDemoBackend({ atraso: 0 });
    const spy = vi.spyOn(backend, "definirIncluirConfigDoJogo");
    const painel = await ajustes(backend);
    expect(await within(painel).findByRole("radio", { name: "Igual às Configurações" })).toBeChecked();
    await userEvent.click(within(painel).getByRole("radio", { name: "Incluir" }));
    expect(spy).toHaveBeenCalledWith(IDS.academia, true);
  });

  it("Fixar num automático chama o app e a estrela aparece", async () => {
    const backend = createDemoBackend({ atraso: 0 });
    const arquivo = (await backend.historico(IDS.academia))[1].fileName;
    const spy = vi.spyOn(backend, "fixarBackup");
    renderApp(backend);
    const { painel } = await abrir("Academia Arcana");
    const itens = await within(painel).findAllByRole("listitem");
    expect(within(itens[1]).queryByRole("button", { name: "Fixar este backup" })).toBeNull();
    expect(itens[2]).not.toHaveClass("pin");
    await userEvent.click(within(itens[2]).getByRole("button", { name: "Fixar este backup" }));
    expect(spy).toHaveBeenCalledWith(IDS.academia, arquivo, true);
    await waitFor(() => expect(within(painel).getAllByRole("listitem")[2]).toHaveClass("pin"));
    expect(within(within(painel).getAllByRole("listitem")[2]).getByRole("button", { name: "Desafixar este backup" })).toBeInTheDocument();
  });

  it("Apagar só apaga depois de confirmar, e Cancelar não apaga", async () => {
    const backend = createDemoBackend({ atraso: 0 });
    const arquivo = (await backend.historico(IDS.academia))[1].fileName;
    const spy = vi.spyOn(backend, "apagarBackup");
    renderApp(backend);
    const { painel } = await abrir("Academia Arcana");
    const itens = await within(painel).findAllByRole("listitem");
    await userEvent.click(within(itens[2]).getByRole("button", { name: "Apagar este backup" }));
    let dialogo = await screen.findByRole("dialog", { name: "Apagar este backup?" });
    expect(dialogo).toHaveTextContent("sai do cofre e não dá para desfazer.");
    await userEvent.click(within(dialogo).getByRole("button", { name: "Cancelar" }));
    await waitFor(() => expect(screen.queryByRole("dialog", { name: "Apagar este backup?" })).toBeNull());
    expect(spy).not.toHaveBeenCalled();

    await userEvent.click(within(itens[2]).getByRole("button", { name: "Apagar este backup" }));
    dialogo = await screen.findByRole("dialog", { name: "Apagar este backup?" });
    await userEvent.click(within(dialogo).getByRole("button", { name: "Apagar" }));
    expect(spy).toHaveBeenCalledWith(IDS.academia, arquivo);
    await waitFor(() => expect(within(painel).getAllByRole("listitem")).toHaveLength(itens.length - 1));
  });
});
