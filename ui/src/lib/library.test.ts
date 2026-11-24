import type { GameView } from "../api/gerado/GameView";
import type { VaultView } from "../api/gerado/VaultView";
import { alertaPrincipal, contagens, filtrar, ordenar, precisaAtencao, subtitulo } from "./library";

const agora = new Date(2026, 10, 23, 12, 0);
const ha = (min: number) => new Date(agora.getTime() - min * 60_000).toISOString();

function jogo(id: string, extra: Partial<GameView> = {}): GameView {
  return {
    id,
    title: id,
    originalTitle: id,
    kind: "pc",
    sourceKey: "steam",
    sourceLabel: "Steam",
    status: "protegido",
    vaultOnly: false,
    lastSaved: ha(60),
    lastBackup: ha(60),
    backupCount: 1,
    totalSize: 10,
    fileCount: 1,
    roots: [],
    warnings: [],
    cover: { kind: "gerada", path: null },
    processNames: [],
    gameDir: null,
    needsGameDir: false,
    isCustom: false,
    rememberedTargets: [],
    ...extra,
  };
}

const cofreOk: VaultView = { path: "D:\\cofre", available: true, versions: 1, zipBytes: 1, diskTotal: null, diskFree: null };

describe("biblioteca", () => {
  it("ordena pelo último save, só no cofre depois e sem save no fim", () => {
    const lista = [
      jogo("velho", { lastSaved: ha(600) }),
      jogo("vazio", { status: "semSave", lastSaved: null }),
      jogo("cofre", { vaultOnly: true, status: "semSave", lastSaved: null, lastBackup: ha(30) }),
      jogo("novo", { lastSaved: ha(5) }),
    ];
    expect(ordenar(lista).map((j) => j.id)).toEqual(["novo", "velho", "cofre", "vazio"]);
  });

  it("contagens por chip e por fonte", () => {
    const lista = [
      jogo("a"),
      jogo("b", { sourceKey: "epic" }),
      jogo("c", { kind: "recomp", sourceKey: "catalogo" }),
      jogo("d", { kind: "emulador", sourceKey: "catalogo" }),
      jogo("e", { kind: "manual", sourceKey: "manual", status: "semBackup" }),
      jogo("f", { vaultOnly: true }),
    ];
    const c = contagens(lista);
    expect(c.chips).toEqual({ todos: 6, pc: 3, recomp: 1, emulador: 1, atencao: 1 });
    expect(c.fontes).toEqual({ steam: 1, epic: 1, gog: 0, pcOutros: 0, recomp: 1, emulador: 1, manual: 1, cofre: 1 });
  });

  it("precisa de atenção inclui sem backup e pasta arriscada", () => {
    expect(precisaAtencao(jogo("a", { status: "semBackup" }))).toBe(true);
    expect(precisaAtencao(jogo("b", { warnings: [{ kind: "pastaArriscada", detail: "Downloads" }] }))).toBe(true);
    expect(precisaAtencao(jogo("c", { status: "saveNovo" }))).toBe(false);
    expect(precisaAtencao(jogo("d", { warnings: [{ kind: "nuvem", detail: "Steam" }] }))).toBe(false);
  });

  it("filtra por chip e por fonte", () => {
    const lista = [jogo("a"), jogo("c", { kind: "recomp", sourceKey: "catalogo" }), jogo("f", { vaultOnly: true })];
    expect(filtrar(lista, { chip: "recomp", fonte: null, busca: "" }).map((j) => j.id)).toEqual(["c"]);
    expect(filtrar(lista, { chip: "todos", fonte: "cofre", busca: "" }).map((j) => j.id)).toEqual(["f"]);
    expect(filtrar(lista, { chip: "todos", fonte: "steam", busca: "" }).map((j) => j.id)).toEqual(["a"]);
  });

  it("busca ignora acento e maiúscula e acha pelo caminho", () => {
    const lista = [
      jogo("a", { title: "Ação & Reação" }),
      jogo("b", { title: "Outro", roots: [{ path: "D:\\Jogos\\Portatil\\saves", template: "<gameDir>/saves", files: [] }] }),
      jogo("c", { title: "Terceiro", sourceLabel: "Project64" }),
    ];
    expect(filtrar(lista, { chip: "todos", fonte: null, busca: "acao" }).map((j) => j.id)).toEqual(["a"]);
    expect(filtrar(lista, { chip: "todos", fonte: null, busca: "PORTATIL" }).map((j) => j.id)).toEqual(["b"]);
    expect(filtrar(lista, { chip: "todos", fonte: null, busca: "project" }).map((j) => j.id)).toEqual(["c"]);
  });

  it("alerta: cofre indisponível tem prioridade", () => {
    const lista = [jogo("a", { status: "semBackup" })];
    expect(alertaPrincipal(lista, { ...cofreOk, available: false })).toEqual({ tipo: "cofre", caminho: "D:\\cofre" });
    expect(alertaPrincipal(lista, cofreOk)).toMatchObject({ tipo: "semBackup" });
    const arriscado = [jogo("b", { warnings: [{ kind: "pastaArriscada", detail: "Downloads" }] })];
    expect(alertaPrincipal(arriscado, cofreOk)).toMatchObject({ tipo: "arriscada" });
    expect(alertaPrincipal([jogo("c")], cofreOk)).toBeNull();
  });

  it("subtítulo de cada status", () => {
    const abertos = new Set(["aberto"]);
    expect(subtitulo(jogo("aberto", { status: "saveNovo" }), agora, abertos)).toBe("Steam · jogando agora");
    expect(subtitulo(jogo("p", { lastBackup: ha(120) }), agora, abertos)).toBe("Steam · backup há 2 h");
    expect(subtitulo(jogo("n", { status: "saveNovo", lastSaved: ha(12) }), agora, abertos)).toBe("Steam · há 12 min");
    expect(subtitulo(jogo("s", { status: "semBackup", lastSaved: ha(40 * 24 * 60) }), agora, abertos)).toBe("Steam · 14 out");
    expect(subtitulo(jogo("v", { status: "semSave", lastSaved: null }), agora, abertos)).toBe("Steam · nenhum save ainda");
    expect(
      subtitulo(jogo("c", { vaultOnly: true, sourceLabel: "Só no cofre", status: "semSave", lastBackup: ha(3 * 24 * 60) }), agora, abertos),
    ).toBe("Só no cofre · backup há 3 d");
  });
});
