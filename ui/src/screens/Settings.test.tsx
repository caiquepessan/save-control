import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { IDS, createDemoBackend } from "../api/demo";
import { renderApp } from "../test/render";

async function abrirConfiguracoes() {
  await screen.findByRole("heading", { name: "Seus jogos" });
  await userEvent.click(within(screen.getByRole("navigation", { name: "Menu" })).getByRole("button", { name: "Configurações" }));
  return screen.findByRole("heading", { name: "Configurações" });
}

describe("configurações", () => {
  it("as chaves chamam o comando certo", async () => {
    const backend = createDemoBackend({ atraso: 0 });
    const capas = vi.spyOn(backend, "definirCapasSteam");
    const config = vi.spyOn(backend, "definirIncluirConfig");
    renderApp(backend);
    await abrirConfiguracoes();
    const chaveCapas = await screen.findByRole("switch", { name: "Baixar capas oficiais da Steam" });
    expect(chaveCapas).toBeChecked();
    await userEvent.click(chaveCapas);
    expect(capas).toHaveBeenCalledWith(false);
    await userEvent.click(screen.getByRole("switch", { name: "Incluir arquivos de configuração, além dos saves" }));
    expect(config).toHaveBeenCalledWith(true);
    await waitFor(() => expect(screen.getByRole("switch", { name: "Baixar capas oficiais da Steam" })).not.toBeChecked());
  });

  it("adicionar e remover pasta de jogos manda a lista nova", async () => {
    const backend = createDemoBackend({ atraso: 0 });
    const spy = vi.spyOn(backend, "definirPastasDeJogos");
    renderApp(backend);
    await abrirConfiguracoes();
    await userEvent.click(await screen.findByRole("button", { name: "Adicionar pasta…" }));
    await waitFor(() => expect(spy).toHaveBeenCalledWith(["D:\\Jogos", "D:\\Jogos\\Pasta Escolhida"]));
    await userEvent.click(await screen.findByRole("button", { name: "Remover D:\\Jogos" }));
    await waitFor(() => expect(spy).toHaveBeenLastCalledWith(["D:\\Jogos\\Pasta Escolhida"]));
  });

  it("Voltar a mostrar desfaz o ignorar", async () => {
    const backend = createDemoBackend({ atraso: 0 });
    await backend.ignorarJogo(IDS.corrida, true);
    const spy = vi.spyOn(backend, "ignorarJogo");
    renderApp(backend);
    await abrirConfiguracoes();
    const item = (await screen.findByText("Corrida Pixel")).closest("li")!;
    await userEvent.click(within(item).getByRole("button", { name: "Voltar a mostrar" }));
    expect(spy).toHaveBeenCalledWith(IDS.corrida, false);
    expect(await screen.findByText("Nenhum jogo ignorado.")).toBeInTheDocument();
  });

  it("Atualizar agora mostra o resultado", async () => {
    renderApp(createDemoBackend({ atraso: 0 }));
    await abrirConfiguracoes();
    await userEvent.click(await screen.findByRole("button", { name: "Atualizar agora" }));
    expect(await screen.findByText("Base atualizada.")).toBeInTheDocument();
  });

  it("trocar a pasta do cofre só cria o cofre depois de confirmar", async () => {
    const backend = createDemoBackend({ atraso: 0 });
    const trocar = vi.spyOn(backend, "trocarCofre");
    renderApp(backend);
    await abrirConfiguracoes();
    await userEvent.click(await screen.findByRole("button", { name: "Trocar pasta…" }));
    const d = await screen.findByRole("dialog", { name: "Criar um cofre novo nesta pasta?" });
    // A pasta escolhida tem outros arquivos: o cofre novo vai para a subpasta "cofre".
    expect(within(d).getByText("D:\\Jogos\\Pasta Escolhida\\cofre")).toBeInTheDocument();
    expect(trocar).not.toHaveBeenCalled();
    await userEvent.click(within(d).getByRole("button", { name: "Criar" }));
    await waitFor(() => expect(trocar).toHaveBeenCalledWith("D:\\Jogos\\Pasta Escolhida\\cofre", true));
    const secao = screen.getByRole("region", { name: "Cofre" });
    expect(await within(secao).findByText("D:\\Jogos\\Pasta Escolhida\\cofre")).toBeInTheDocument();
  });

  it("as chaves do segundo plano chamam o comando certo", async () => {
    const backend = createDemoBackend({ atraso: 0 });
    const auto = vi.spyOn(backend, "definirAutomatico");
    const bandeja = vi.spyOn(backend, "definirFecharParaBandeja");
    const iniciar = vi.spyOn(backend, "definirIniciarComWindows");
    const notificacoes = vi.spyOn(backend, "definirNotificacoes");
    renderApp(backend);
    await abrirConfiguracoes();
    await userEvent.click(await screen.findByRole("switch", { name: "Fazer backup automático" }));
    expect(auto).toHaveBeenCalledWith(false);
    await userEvent.click(screen.getByRole("switch", { name: "Fechar a janela deixa o app na bandeja" }));
    expect(bandeja).toHaveBeenCalledWith(false);
    await userEvent.click(screen.getByRole("switch", { name: "Iniciar com o Windows (na bandeja)" }));
    expect(iniciar).toHaveBeenCalledWith(true);
    await userEvent.click(screen.getByRole("switch", { name: "Mostrar notificações do Windows" }));
    expect(notificacoes).toHaveBeenCalledWith(true);
    expect(screen.getByText("O app registra o nome e o ícone dele no seu usuário do Windows. Dá para remover.")).toBeInTheDocument();
  });

  it("Salvar da retenção manda os 3 números", async () => {
    const backend = createDemoBackend({ atraso: 0 });
    const spy = vi.spyOn(backend, "definirRetencao");
    renderApp(backend);
    await abrirConfiguracoes();
    for (const [campo, valor] of [
      ["Guardar os últimos", "5"],
      ["Dias com 1 backup por dia", "3"],
      ["Semanas com 1 backup por semana", "2"],
    ]) {
      const entrada = await screen.findByRole("spinbutton", { name: campo });
      await userEvent.clear(entrada);
      await userEvent.type(entrada, valor);
    }
    await userEvent.click(screen.getByRole("button", { name: "Salvar a retenção" }));
    expect(spy).toHaveBeenCalledWith({ autoRecent: 5, autoDailyDays: 3, autoWeeklyWeeks: 2 });
  });

  it("Salvar dos tempos manda os valores digitados, e fora da faixa mostra o erro", async () => {
    const backend = createDemoBackend({ atraso: 0 });
    const spy = vi.spyOn(backend, "definirTemposDoAutomatico");
    renderApp(backend);
    await abrirConfiguracoes();
    const preencher = async (campo: string, valor: string) => {
      const entrada = await screen.findByRole("spinbutton", { name: campo });
      await userEvent.clear(entrada);
      await userEvent.type(entrada, valor);
    };
    await preencher("Segundos depois de fechar o jogo", "20");
    await preencher("Minutos entre backups com o jogo aberto", "30");
    await preencher("Segundos sem mudanças", "90");
    await userEvent.click(screen.getByRole("button", { name: "Salvar os tempos" }));
    expect(spy).toHaveBeenCalledWith({ quietSecs: 90, afterCloseSecs: 20, whileRunningMins: 30 });
    await preencher("Minutos entre backups com o jogo aberto", "200");
    await userEvent.click(screen.getByRole("button", { name: "Salvar os tempos" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("Use um valor entre 5 e 120 minutos.");
  });

  it("Remover registros do Windows só remove depois de confirmar, e as chaves desligam", async () => {
    const backend = createDemoBackend({ atraso: 0 });
    await backend.definirIniciarComWindows(true);
    await backend.definirNotificacoes(true);
    const spy = vi.spyOn(backend, "removerRegistrosDoWindows");
    renderApp(backend);
    await abrirConfiguracoes();
    expect(await screen.findByRole("switch", { name: "Iniciar com o Windows (na bandeja)" })).toBeChecked();
    await userEvent.click(screen.getByRole("button", { name: "Remover registros do Windows" }));
    let d = await screen.findByRole("dialog", { name: "Remover a entrada de iniciar com o Windows e o registro das notificações?" });
    await userEvent.click(within(d).getByRole("button", { name: "Cancelar" }));
    expect(spy).not.toHaveBeenCalled();
    await userEvent.click(screen.getByRole("button", { name: "Remover registros do Windows" }));
    d = await screen.findByRole("dialog", { name: "Remover a entrada de iniciar com o Windows e o registro das notificações?" });
    await userEvent.click(within(d).getByRole("button", { name: "Remover" }));
    expect(spy).toHaveBeenCalledTimes(1);
    await waitFor(() => expect(screen.getByRole("switch", { name: "Iniciar com o Windows (na bandeja)" })).not.toBeChecked());
    expect(screen.getByRole("switch", { name: "Mostrar notificações do Windows" })).not.toBeChecked();
  });

  it("Sobre mostra a versão e os créditos", async () => {
    renderApp(createDemoBackend({ atraso: 0 }));
    await abrirConfiguracoes();
    expect(await screen.findByText("Save Control 0.1.0")).toBeInTheDocument();
    expect(screen.getByText("Locais de save: ludusavi-manifest, com dados do PCGamingWiki.")).toBeInTheDocument();
  });
});
