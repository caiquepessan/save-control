import { act, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ErroApp } from "../../api/backend";
import { IDS, createDemoBackend } from "../../api/demo";
import { renderApp } from "../../test/render";

async function abrirPainel(titulo: string) {
  await screen.findByRole("heading", { name: "Seus jogos" });
  const cartao = screen.getByRole("article", { name: titulo });
  await userEvent.click(within(cartao).getByRole("button", { name: `Abrir ${titulo}` }));
  return screen.findByRole("dialog", { name: titulo });
}

/** Abre o painel e clica em "Restaurar" na versão de índice `i` do histórico. */
async function restaurarVersao(titulo: string, i = 0) {
  const painel = await abrirPainel(titulo);
  const versoes = (await within(painel).findAllByRole("listitem")).filter((li) => within(li).queryByRole("button", { name: "Restaurar" }));
  await userEvent.click(within(versoes[i]).getByRole("button", { name: "Restaurar" }));
}

const confirmacao = () => screen.findByRole("dialog", { name: /^Restaurar "/ });

describe("restaurar", () => {
  it("a confirmação mostra os textos do mockup", async () => {
    renderApp();
    await restaurarVersao("Academia Arcana");
    const d = await confirmacao();
    expect(d).toHaveAccessibleName('Restaurar "Backup manual"?');
    expect(d).toHaveTextContent(/O save do Academia Arcana volta a como estava .+\. Antes disso, o save atual é guardado no cofre, então dá pra desfazer\./);
    expect(within(d).getByText("2 arquivos · 14,0 MB")).toBeInTheDocument();
    expect(within(d).getByText("…\\Saved\\SaveGames\\7656")).toBeInTheDocument();
    expect(within(d).getByRole("button", { name: "Cancelar" })).toBeInTheDocument();
    expect(within(d).getByRole("button", { name: "Restaurar" })).toBeInTheDocument();
    expect(within(d).getByRole("button", { name: "Restaurar em outra pasta…" })).toBeEnabled();
  });

  it("com o jogo aberto, pede para fechar e oferece Tentar de novo", async () => {
    const backend = createDemoBackend({ atraso: 0 });
    const previa = vi.spyOn(backend, "previaRestauracao");
    renderApp(backend);
    await restaurarVersao("Reino Esmeralda: A Flauta do Tempo");
    const d = await confirmacao();
    expect(within(d).getByText("O jogo está aberto. Feche Reino64Recompiled antes de restaurar.")).toBeInTheDocument();
    expect(within(d).queryByRole("button", { name: "Restaurar" })).toBeNull();
    await userEvent.click(within(d).getByRole("button", { name: "Tentar de novo" }));
    await waitFor(() => expect(previa).toHaveBeenCalledTimes(2));
  });

  it("quando o app pergunta onde restaurar, a escolha é lembrada", async () => {
    const backend = createDemoBackend({ atraso: 0 });
    const previa = vi.spyOn(backend, "previaRestauracao");
    const restaurar = vi.spyOn(backend, "restaurar");
    renderApp(backend);
    await restaurarVersao("Fortaleza Esquecida");
    const onde = await screen.findByRole("dialog", { name: "Onde restaurar?" });
    await userEvent.click(within(onde).getByRole("button", { name: "E:\\Jogos\\Fortaleza\\saves" }));
    const arquivo = previa.mock.calls[0][1];
    await waitFor(() => expect(previa).toHaveBeenLastCalledWith(IDS.fortaleza, arquivo, { r0: "E:\\Jogos\\Fortaleza\\saves" }));
    const d = await confirmacao();
    await userEvent.click(within(d).getByRole("button", { name: "Restaurar" }));
    await waitFor(() =>
      expect(restaurar).toHaveBeenCalledWith(IDS.fortaleza, arquivo, {
        targets: { r0: "E:\\Jogos\\Fortaleza\\saves" },
        onlyFiles: null,
        remember: true,
      }),
    );
  });

  it("restaurar em outra pasta não fica lembrado", async () => {
    const backend = createDemoBackend({ atraso: 0 });
    const escolher = vi.spyOn(backend, "escolherPasta");
    const restaurar = vi.spyOn(backend, "restaurar");
    renderApp(backend);
    await restaurarVersao("Academia Arcana");
    const d = await confirmacao();
    await userEvent.click(within(d).getByRole("button", { name: "Restaurar em outra pasta…" }));
    expect(escolher).toHaveBeenCalled();
    await waitFor(() => expect(within(d).getByText("D:\\Jogos\\Pasta Escolhida")).toBeInTheDocument());
    expect(within(d).getByText(/o save do jogo não muda/)).toBeInTheDocument();
    await userEvent.click(within(d).getByRole("button", { name: "Restaurar" }));
    await waitFor(() =>
      expect(restaurar).toHaveBeenCalledWith(IDS.academia, expect.any(String), {
        targets: { r0: "D:\\Jogos\\Pasta Escolhida" },
        onlyFiles: null,
        remember: false,
      }),
    );
  });

  it("dá para restaurar só alguns arquivos de uma versão", async () => {
    const backend = createDemoBackend({ atraso: 0 });
    const restaurar = vi.spyOn(backend, "restaurar");
    renderApp(backend);
    const painel = await abrirPainel("Academia Arcana");
    await userEvent.click(within(painel).getByRole("tab", { name: /Arquivos/ }));
    await userEvent.click(within(painel).getByRole("button", { name: "Restaurar arquivos de uma versão…" }));
    const sel = await screen.findByRole("dialog", { name: "Restaurar arquivos de uma versão" });
    await userEvent.click(await within(sel).findByRole("checkbox", { name: "config.sav" }));
    await userEvent.click(within(sel).getByRole("button", { name: "Continuar" }));
    const d = await confirmacao();
    expect(within(d).getByText("1 de 2 arquivos")).toBeInTheDocument();
    await userEvent.click(within(d).getByRole("button", { name: "Restaurar" }));
    await waitFor(() =>
      expect(restaurar).toHaveBeenCalledWith(IDS.academia, expect.any(String), {
        targets: {},
        onlyFiles: [{ root: "r0", relPath: "perfil.sav" }],
        remember: false,
      }),
    );
  });

  it("quando nem o desfazer funciona, o aviso fica até fechar", async () => {
    const backend = createDemoBackend({ atraso: 0 });
    const msg = "A restauração falhou (disco cheio) e não foi possível desfazer tudo. O save de antes está guardado no cofre em 2026-11-24_x_antes-de-restaurar.zip";
    vi.spyOn(backend, "restaurar").mockRejectedValue(new ErroApp({ kind: "desfazerFalhou", message: msg, root: null, alias: null, candidates: [] }));
    renderApp(backend);
    await restaurarVersao("Academia Arcana");
    await userEvent.click(within(await confirmacao()).getByRole("button", { name: "Restaurar" }));
    expect(await screen.findByRole("alert")).toHaveTextContent(msg);
  });

  it("avisa quando o fechar é bloqueado", async () => {
    const backend = createDemoBackend({ atraso: 0 });
    renderApp(backend);
    await screen.findByRole("heading", { name: "Seus jogos" });
    act(() => backend.emitirFecharBloqueado());
    expect(await screen.findByText("Espere terminar: o Save Control não fecha no meio de um backup ou restauração.")).toBeInTheDocument();
  });

  it("depois de restaurar, avisa onde está o save anterior e recarrega o histórico", async () => {
    const backend = createDemoBackend({ atraso: 0 });
    const historico = vi.spyOn(backend, "historico");
    renderApp(backend);
    await restaurarVersao("Academia Arcana");
    await userEvent.click(within(await confirmacao()).getByRole("button", { name: "Restaurar" }));
    expect(await screen.findByText('Restaurado. Seu save anterior está no histórico como "Antes de restaurar".')).toBeInTheDocument();
    await waitFor(() => expect(historico.mock.calls.length).toBeGreaterThanOrEqual(2));
    expect(screen.queryByRole("dialog", { name: /^Restaurar "/ })).toBeNull();
  });
});
