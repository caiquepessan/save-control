import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { createDemoBackend } from "../../api/demo";
import { renderApp } from "../../test/render";

async function abrirDialogo() {
  await screen.findByRole("heading", { name: "Seus jogos" });
  await userEvent.click(screen.getByRole("button", { name: "Adicionar jogo" }));
  return screen.findByRole("dialog", { name: "Adicionar jogo" });
}

describe("adicionar jogo", () => {
  it("sem nome ou sem pasta mostra o erro e não chama o app", async () => {
    const backend = createDemoBackend({ atraso: 0 });
    const spy = vi.spyOn(backend, "adicionarJogo");
    renderApp(backend);
    const d = await abrirDialogo();
    await userEvent.click(within(d).getByRole("button", { name: "Adicionar" }));
    expect(within(d).getByText("Dê um nome ao jogo.")).toBeInTheDocument();
    await userEvent.type(within(d).getByLabelText("Nome"), "Meu Jogo");
    await userEvent.click(within(d).getByRole("button", { name: "Adicionar" }));
    expect(within(d).getByText("Escolha pelo menos uma pasta do save.")).toBeInTheDocument();
    expect(spy).not.toHaveBeenCalled();
  });

  it("Escolher pasta… põe a pasta na lista", async () => {
    renderApp();
    const d = await abrirDialogo();
    await userEvent.click(within(d).getByRole("button", { name: "Escolher pasta…" }));
    expect(await within(d).findByText("D:\\Jogos\\Pasta Escolhida")).toBeInTheDocument();
  });

  it("adicionar manda tudo e recarrega a biblioteca", async () => {
    const backend = createDemoBackend({ atraso: 0 });
    const escolher = vi.spyOn(backend, "escolherPasta");
    escolher.mockResolvedValueOnce("D:\\Jogos\\Meu Jogo\\saves").mockResolvedValueOnce("D:\\Jogos\\Meu Jogo");
    const adicionar = vi.spyOn(backend, "adicionarJogo");
    const biblioteca = vi.spyOn(backend, "biblioteca");
    renderApp(backend);
    const d = await abrirDialogo();
    const antes = biblioteca.mock.calls.length;
    await userEvent.type(within(d).getByLabelText("Nome"), "Meu Jogo");
    await userEvent.click(within(d).getByRole("button", { name: "Escolher pasta…" }));
    await within(d).findByText("D:\\Jogos\\Meu Jogo\\saves");
    await userEvent.click(within(d).getByRole("button", { name: "Escolher a pasta do jogo" }));
    await within(d).findByText("D:\\Jogos\\Meu Jogo");
    await userEvent.type(within(d).getByLabelText("Executável ou processo (opcional)"), "Jogo.exe");
    await userEvent.click(within(d).getByRole("button", { name: "Escolher imagem…" }));
    await within(d).findByText("C:\\Imagens\\capa.png");
    await userEvent.click(within(d).getByRole("button", { name: "Adicionar" }));
    await waitFor(() =>
      expect(adicionar).toHaveBeenCalledWith({
        name: "Meu Jogo",
        saveDirs: ["D:\\Jogos\\Meu Jogo\\saves"],
        gameDir: "D:\\Jogos\\Meu Jogo",
        processes: ["Jogo.exe"],
        coverFile: "C:\\Imagens\\capa.png",
      }),
    );
    await waitFor(() => expect(screen.queryByRole("dialog", { name: "Adicionar jogo" })).toBeNull());
    await waitFor(() => expect(biblioteca.mock.calls.length).toBeGreaterThan(antes));
    expect(await screen.findByRole("article", { name: "Meu Jogo" })).toBeInTheDocument();
  });
});
