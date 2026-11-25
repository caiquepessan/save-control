import { act, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { IDS, createDemoBackend, type DemoBackend } from "../api/demo";
import type { ActivityView } from "../api/gerado/ActivityView";
import { renderApp } from "../test/render";

async function abrirAtividade(backend: DemoBackend = createDemoBackend({ atraso: 0 })) {
  const r = renderApp(backend);
  const menu = await screen.findByRole("navigation", { name: "Menu" });
  await userEvent.click(within(menu).getByRole("button", { name: "Atividade" }));
  const tela = screen.getByRole("main");
  await within(tela).findByRole("heading", { level: 1, name: "Atividade" });
  return { ...r, backend, tela };
}

describe("tela Atividade", () => {
  it("mostra o mais novo primeiro, agrupado por dia", async () => {
    const backend = createDemoBackend({ atraso: 0 });
    const agora = new Date();
    const ontem = new Date(agora.getFullYear(), agora.getMonth(), agora.getDate() - 1, 12, 0);
    const eventos: ActivityView[] = [
      { at: agora.toISOString(), kind: "backupAuto", gameId: IDS.reino, title: "Reino Esmeralda: A Flauta do Tempo", detail: "7 arquivos · 1,2 MB" },
      { at: ontem.toISOString(), kind: "restore", gameId: IDS.baloes, title: "Balões em Defesa 6", detail: "1 arquivo restaurado" },
    ];
    vi.spyOn(backend, "atividade").mockResolvedValue(eventos);
    const { tela } = await abrirAtividade(backend);
    await within(tela).findByText("Restauração · Balões em Defesa 6");
    expect(within(tela).getAllByRole("heading", { level: 2 }).map((h) => h.textContent)).toEqual(["Hoje", "Ontem"]);
    const linhas = within(tela).getAllByRole("listitem");
    expect(linhas[0]).toHaveTextContent("Backup automático · Reino Esmeralda: A Flauta do Tempo");
    expect(linhas[0]).toHaveTextContent("7 arquivos · 1,2 MB");
    expect(linhas[1]).toHaveTextContent("Restauração · Balões em Defesa 6");
  });

  it("os filtros mostram quantos eventos cada um tem", async () => {
    const { tela } = await abrirAtividade();
    await within(tela).findByText("Backup manual · Academia Arcana");
    const filtros = within(tela).getByRole("group", { name: "Filtros" });
    expect(within(filtros).getAllByRole("button").map((b) => b.textContent)).toEqual([
      "Todos 10",
      "Backups 6",
      "Restaurações 1",
      "Erros 1",
    ]);
  });

  it("o filtro Erros só mostra os erros", async () => {
    const { tela } = await abrirAtividade();
    await within(tela).findByText("Backup manual · Academia Arcana");
    await userEvent.click(within(tela).getByRole("button", { name: /^Erros/ }));
    const linhas = within(tela).getAllByRole("listitem");
    expect(linhas).toHaveLength(1);
    expect(linhas[0]).toHaveTextContent("Erro · Estrela Saltitante 64");
  });

  it("clicar numa linha abre o painel do jogo", async () => {
    const { tela } = await abrirAtividade();
    await userEvent.click(await within(tela).findByRole("button", { name: /Backup manual · Academia Arcana/ }));
    const painel = await screen.findByRole("dialog", { name: "Academia Arcana" });
    expect(within(painel).getByRole("tab", { name: /Histórico/ })).toHaveAttribute("aria-selected", "true");
  });

  it("sem eventos, explica o que aparece aqui", async () => {
    const backend = createDemoBackend({ atraso: 0 });
    vi.spyOn(backend, "atividade").mockResolvedValue([]);
    const { tela } = await abrirAtividade(backend);
    expect(await within(tela).findByText("Nada por aqui ainda. Backups, restaurações e erros aparecem aqui.")).toBeInTheDocument();
  });

  it("recarrega quando o app muda a biblioteca", async () => {
    const backend = createDemoBackend({ atraso: 0 });
    const atividade = vi.spyOn(backend, "atividade");
    const { tela } = await abrirAtividade(backend);
    await within(tela).findByText("Backup manual · Academia Arcana");
    const antes = atividade.mock.calls.length;
    act(() => backend.emitirBibliotecaMudou());
    await waitFor(() => expect(atividade).toHaveBeenCalledTimes(antes + 1));
  });
});
