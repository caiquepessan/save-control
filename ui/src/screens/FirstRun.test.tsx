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
    expect(await screen.findByText("D:\\Jogos\\Pasta Escolhida")).toBeInTheDocument();
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
    await userEvent.click(screen.getByRole("button", { name: "Abrir a biblioteca" }));
    expect(await screen.findByRole("heading", { name: "Seus jogos" })).toBeInTheDocument();
  });

  it("Agora não vai para a biblioteca sem backup", async () => {
    const backend = createDemoBackend({ atraso: 0, fase: "primeiroUso" });
    const backup = vi.spyOn(backend, "backupDeVarios");
    renderApp(backend);
    await irAteOCofre();
    await userEvent.click(screen.getByRole("button", { name: "Continuar" }));
    await userEvent.click(await screen.findByRole("button", { name: "Agora não" }));
    expect(await screen.findByRole("heading", { name: "Seus jogos" })).toBeInTheDocument();
    expect(backup).not.toHaveBeenCalled();
  });
});
