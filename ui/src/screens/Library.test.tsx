import { act, fireEvent, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { IDS, createDemoBackend } from "../api/demo";
import { renderApp } from "../test/render";

async function cartoes() {
  await screen.findByRole("heading", { name: "Seus jogos" });
  return screen.getAllByRole("article");
}

describe("biblioteca", () => {
  it("um cartão por jogo, com o selo certo; os sem save ficam esmaecidos", async () => {
    renderApp();
    const lista = await cartoes();
    expect(lista).toHaveLength(14);
    const reino = screen.getByRole("article", { name: "Reino Esmeralda: A Flauta do Tempo" });
    expect(within(reino).getByText("Save novo")).toBeInTheDocument();
    const urso = screen.getByRole("article", { name: "Urso & Pássaro" });
    expect(within(urso).getByText("Sem backup")).toBeInTheDocument();
    const laminas = screen.getByRole("article", { name: "Lâminas do Olimpo III" });
    expect(within(laminas).getByText("Sem save")).toBeInTheDocument();
    expect(laminas).toHaveClass("dim");
    const fortaleza = screen.getByRole("article", { name: "Fortaleza Esquecida" });
    expect(within(fortaleza).getByText("Só no cofre")).toBeInTheDocument();
    expect(fortaleza).not.toHaveClass("dim");
    expect(screen.getByText("14 encontrados · 7 protegidos · ordenados pelo último save")).toBeInTheDocument();
  });

  it("o chip Emuladores deixa só os emuladores", async () => {
    renderApp();
    await cartoes();
    const filtros = screen.getByRole("group", { name: "Filtros" });
    await userEvent.click(within(filtros).getByRole("button", { name: /Emuladores/ }));
    const nomes = screen.getAllByRole("article").map((a) => a.getAttribute("aria-label"));
    expect(nomes.sort()).toEqual(["Estrela Saltitante 64", "O Escudo do Passado"]);
  });

  it("Ctrl+K vai para a busca, e a busca filtra", async () => {
    renderApp();
    await cartoes();
    fireEvent.keyDown(window, { key: "k", ctrlKey: true });
    const busca = screen.getByRole("searchbox");
    expect(busca).toHaveFocus();
    await userEvent.type(busca, "passaro");
    expect(screen.getAllByRole("article").map((a) => a.getAttribute("aria-label"))).toEqual(["Urso & Pássaro"]);
  });

  it("o aviso de jogo sem backup faz o backup dele", async () => {
    const backend = createDemoBackend({ atraso: 0 });
    const spy = vi.spyOn(backend, "backupDeVarios");
    renderApp(backend);
    const aviso = await screen.findByRole("status", { name: "Aviso" });
    expect(aviso).toHaveTextContent("Urso & Pássaro ainda não tem nenhum backup");
    expect(aviso).toHaveTextContent("D:\\Jogos\\Urso Recompiled\\saves (modo portátil)");
    await userEvent.click(within(aviso).getByRole("button", { name: "Fazer backup agora" }));
    expect(spy).toHaveBeenCalledWith([IDS.urso]);
  });

  it("Backup de tudo roda uma vez só e fica travado enquanto roda", async () => {
    const backend = createDemoBackend({ atraso: 30 });
    const spy = vi.spyOn(backend, "backupDeVarios");
    renderApp(backend);
    await screen.findByRole("heading", { name: "Seus jogos" }, { timeout: 3000 });
    const botao = screen.getByRole("button", { name: "Backup de tudo" });
    await userEvent.click(botao);
    await userEvent.click(botao);
    await waitFor(() => expect(botao).toBeDisabled());
    await waitFor(() => expect(spy).toHaveBeenCalledTimes(1));
    expect(spy.mock.calls[0][0].sort()).toEqual([IDS.reino, IDS.urso, IDS.estrela, IDS.acao].sort());
    await waitFor(() => expect(botao).toBeEnabled(), { timeout: 3000 });
    expect(await screen.findByText(/4 backups feitos/)).toBeInTheDocument();
  });

  it("a capa da Steam chega depois e troca a capa gerada", async () => {
    const backend = createDemoBackend({ atraso: 0 });
    renderApp(backend);
    await cartoes();
    const academia = screen.getByRole("article", { name: "Academia Arcana" });
    expect(within(academia).queryByRole("img")).toBeNull();
    act(() => backend.emitirCapa(IDS.academia, "C:\\capas\\steam-10.jpg"));
    expect(await within(academia).findByRole("img")).toHaveAttribute("src", "C:\\capas\\steam-10.jpg");
  });
});
