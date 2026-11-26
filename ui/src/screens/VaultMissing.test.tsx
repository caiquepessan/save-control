import { act, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { createDemoBackend } from "../api/demo";
import { renderApp } from "../test/render";

describe("cofre sumido", () => {
  it("Tentar de novo abre a biblioteca quando o cofre volta", async () => {
    const backend = createDemoBackend({ atraso: 0, fase: "cofreSumiu" });
    const spy = vi.spyOn(backend, "tentarCofreDeNovo");
    renderApp(backend);
    expect(await screen.findByRole("heading", { name: "Não encontramos o cofre" })).toBeInTheDocument();
    expect(screen.getByText(/Ele deveria estar em D:\\Save Control\\cofre\./)).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Tentar de novo" }));
    expect(spy).toHaveBeenCalled();
    expect(await screen.findByRole("heading", { name: "Seus jogos" })).toBeInTheDocument();
  });

  it("cofre sumido não cria nada sem confirmar", async () => {
    const backend = createDemoBackend({ atraso: 0, fase: "cofreSumiu" });
    const trocar = vi.spyOn(backend, "trocarCofre");
    renderApp(backend);
    await userEvent.click(await screen.findByRole("button", { name: "Criar um cofre novo aqui" }));
    const d = await screen.findByRole("dialog", { name: "Criar um cofre vazio em D:\\Save Control\\cofre? Os backups antigos continuam onde estiverem." });
    expect(trocar).not.toHaveBeenCalled();
    await userEvent.click(within(d).getByRole("button", { name: "Criar" }));
    await waitFor(() => expect(trocar).toHaveBeenCalledWith("D:\\Save Control\\cofre", true));
  });

  it("Procurar o cofre usa a pasta escolhida, se tiver um cofre", async () => {
    const backend = createDemoBackend({ atraso: 0, fase: "cofreSumiu" });
    const escolher = vi.spyOn(backend, "escolherPasta");
    const trocar = vi.spyOn(backend, "trocarCofre");
    renderApp(backend);
    escolher.mockResolvedValueOnce("D:\\Jogos\\Pasta Escolhida");
    await userEvent.click(await screen.findByRole("button", { name: "Procurar o cofre…" }));
    expect(await screen.findByText("Não há um cofre nessa pasta.")).toBeInTheDocument();
    expect(trocar).not.toHaveBeenCalled();
    escolher.mockResolvedValueOnce("E:\\Backup\\cofre");
    await userEvent.click(screen.getByRole("button", { name: "Procurar o cofre…" }));
    const d = await screen.findByRole("dialog", { name: "Usar o cofre desta pasta? Ele tem 212 versões." });
    expect(trocar).not.toHaveBeenCalled();
    await userEvent.click(within(d).getByRole("button", { name: "Usar" }));
    await waitFor(() => expect(trocar).toHaveBeenCalledWith("E:\\Backup\\cofre", false));
    expect(await screen.findByRole("heading", { name: "Seus jogos" })).toBeInTheDocument();
  });

  it("quando o app não abre, mostra o erro e a pasta de dados", async () => {
    const backend = createDemoBackend({ atraso: 0, fase: "falhou" });
    const abrir = vi.spyOn(backend, "abrirPasta");
    renderApp(backend);
    expect(await screen.findByRole("heading", { name: "Não foi possível abrir o Save Control" })).toBeInTheDocument();
    expect(screen.getByText("Arquivo inválido (jogos.json)")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Abrir a pasta de dados" }));
    expect(abrir).toHaveBeenCalledWith("D:\\Save Control\\SaveControl-dados");
  });

  it("quando o cofre volta, a tela sai sozinha", async () => {
    const backend = createDemoBackend({ atraso: 0, fase: "cofreSumiu" });
    renderApp(backend);
    expect(await screen.findByRole("heading", { name: "Não encontramos o cofre" })).toBeInTheDocument();
    // O trabalhador do app viu o HD de volta: o app já está pronto e avisa que a biblioteca mudou.
    const original = backend.estado;
    vi.spyOn(backend, "estado").mockImplementation(async () => ({ ...(await original()), phase: "pronto" }));
    act(() => backend.emitirBibliotecaMudou());
    expect(await screen.findByRole("heading", { name: "Seus jogos" })).toBeInTheDocument();
  });
});
