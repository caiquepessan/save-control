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

  it("Sobre mostra a versão e os créditos", async () => {
    renderApp(createDemoBackend({ atraso: 0 }));
    await abrirConfiguracoes();
    expect(await screen.findByText("Save Control 0.1.0")).toBeInTheDocument();
    expect(screen.getByText("Locais de save: ludusavi-manifest, com dados do PCGamingWiki.")).toBeInTheDocument();
  });
});
