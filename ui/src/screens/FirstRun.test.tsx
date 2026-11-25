import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { IDS, createDemoBackend } from "../api/demo";
import { renderApp } from "../test/render";

const SEM_INTERNET =
  "Sem internet agora: dá pra baixar a base de jogos de PC depois, em Configurações. Recomps, emuladores e jogos adicionados à mão funcionam normalmente.";

async function irAteOCofre() {
  await userEvent.click(await screen.findByRole("button", { name: "Começar" }));
  await screen.findByRole("heading", { name: "Onde guardar o cofre?" });
}

describe("primeiro uso", () => {
  it("mostra o cofre padrão e deixa escolher outra pasta", async () => {
    renderApp(createDemoBackend({ atraso: 0, fase: "primeiroUso" }));
    expect(await screen.findByRole("heading", { name: "Bem-vindo ao Save Control" })).toBeInTheDocument();
    await irAteOCofre();
    expect(screen.getByText("D:\\Save Control\\cofre")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Escolher outra pasta…" }));
    // A pasta escolhida tem outros arquivos: o cofre vai para a subpasta "cofre".
    expect(await screen.findByText("D:\\Jogos\\Pasta Escolhida\\cofre")).toBeInTheDocument();
  });

  it("o resumo diz onde adicionar as pastas de jogos", async () => {
    renderApp(createDemoBackend({ atraso: 0, fase: "primeiroUso" }));
    await irAteOCofre();
    await userEvent.click(screen.getByRole("button", { name: "Continuar" }));
    await screen.findByRole("heading", { name: "Encontramos 11 jogos com save" });
    expect(
      screen.getByText("Recomps, ports ou emuladores numa pasta sua (ex.: D:\\Jogos)? Adicione a pasta em Configurações → Pastas de jogos."),
    ).toBeInTheDocument();
  });

  it("sem internet avisa e segue para o escaneamento", async () => {
    const backend = createDemoBackend({ atraso: 0, fase: "primeiroUso" });
    vi.spyOn(backend, "atualizarBase").mockResolvedValue({ outcome: "falhou", message: "sem conexão" });
    renderApp(backend);
    await irAteOCofre();
    await userEvent.click(screen.getByRole("button", { name: "Continuar" }));
    expect(await screen.findByText(SEM_INTERNET)).toBeInTheDocument();
    expect(await screen.findByRole("heading", { name: "Encontramos 11 jogos com save" })).toBeInTheDocument();
  });

  it("conclui antes de escanear e faz o primeiro backup de tudo", async () => {
    const backend = createDemoBackend({ atraso: 0, fase: "primeiroUso" });
    const concluir = vi.spyOn(backend, "concluirPrimeiroUso");
    const escanear = vi.spyOn(backend, "escanear");
    const backup = vi.spyOn(backend, "backupDeVarios");
    renderApp(backend);
    await irAteOCofre();
    await userEvent.click(screen.getByRole("button", { name: "Continuar" }));
    await screen.findByRole("heading", { name: "Encontramos 11 jogos com save" });
    expect(concluir).toHaveBeenCalledWith("D:\\Save Control\\cofre");
    expect(concluir.mock.invocationCallOrder[0]).toBeLessThan(escanear.mock.invocationCallOrder[0]);
    await userEvent.click(screen.getByRole("button", { name: "Fazer o primeiro backup de tudo (recomendado)" }));
    await waitFor(() => expect(backup).toHaveBeenCalledTimes(1));
    expect(backup.mock.calls[0][0].sort()).toEqual([IDS.reino, IDS.urso, IDS.estrela, IDS.acao].sort());
    expect(await screen.findByText("Pronto! 4 jogos protegidos.")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Continuar" }));
    await screen.findByRole("heading", { name: "Deixe o Save Control cuidar disso" });
    await userEvent.click(screen.getByRole("button", { name: "Abrir a biblioteca" }));
    expect(await screen.findByRole("heading", { name: "Seus jogos" })).toBeInTheDocument();
  });

  it("Agora não pula o backup e passa pelo passo do automático", async () => {
    const backend = createDemoBackend({ atraso: 0, fase: "primeiroUso" });
    const backup = vi.spyOn(backend, "backupDeVarios");
    renderApp(backend);
    await irAteOCofre();
    await userEvent.click(screen.getByRole("button", { name: "Continuar" }));
    await userEvent.click(await screen.findByRole("button", { name: "Agora não" }));
    await screen.findByRole("heading", { name: "Deixe o Save Control cuidar disso" });
    await userEvent.click(screen.getByRole("button", { name: "Abrir a biblioteca" }));
    expect(await screen.findByRole("heading", { name: "Seus jogos" })).toBeInTheDocument();
    expect(backup).not.toHaveBeenCalled();
  });

  async function irAteOAutomatico(backend: ReturnType<typeof createDemoBackend>) {
    renderApp(backend);
    await irAteOCofre();
    await userEvent.click(screen.getByRole("button", { name: "Continuar" }));
    await userEvent.click(await screen.findByRole("button", { name: "Agora não" }));
    await screen.findByRole("heading", { name: "Deixe o Save Control cuidar disso" });
  }

  it("o passo 6 liga o automático e deixa o início com o Windows desligado por padrão", async () => {
    const backend = createDemoBackend({ atraso: 0, fase: "primeiroUso" });
    const auto = vi.spyOn(backend, "definirAutomatico");
    const iniciar = vi.spyOn(backend, "definirIniciarComWindows");
    await irAteOAutomatico(backend);
    expect(screen.getByRole("img", { name: "Passo 6 de 6" })).toBeInTheDocument();
    expect(
      screen.getByText("O backup automático guarda uma cópia quando você fecha o jogo. Para isso, o app fica aberto ou na bandeja."),
    ).toBeInTheDocument();
    expect(screen.getByRole("switch", { name: "Backup automático (recomendado)" })).toBeChecked();
    expect(screen.getByRole("switch", { name: "Iniciar com o Windows, na bandeja" })).not.toBeChecked();
    await userEvent.click(screen.getByRole("button", { name: "Abrir a biblioteca" }));
    expect(auto).toHaveBeenCalledWith(true);
    expect(iniciar).toHaveBeenCalledWith(false);
    expect(await screen.findByRole("heading", { name: "Seus jogos" })).toBeInTheDocument();
  });

  it("o passo 6 manda o que a pessoa escolheu", async () => {
    const backend = createDemoBackend({ atraso: 0, fase: "primeiroUso" });
    const auto = vi.spyOn(backend, "definirAutomatico");
    const iniciar = vi.spyOn(backend, "definirIniciarComWindows");
    await irAteOAutomatico(backend);
    await userEvent.click(screen.getByRole("switch", { name: "Backup automático (recomendado)" }));
    await userEvent.click(screen.getByRole("switch", { name: "Iniciar com o Windows, na bandeja" }));
    await userEvent.click(screen.getByRole("button", { name: "Abrir a biblioteca" }));
    expect(auto).toHaveBeenCalledWith(false);
    expect(iniciar).toHaveBeenCalledWith(true);
    expect(await screen.findByRole("heading", { name: "Seus jogos" })).toBeInTheDocument();
  });
});
