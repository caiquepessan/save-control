import { EMBLEMAS, PALETAS, capaGerada, tituloCurto } from "./cover";

describe("capa gerada", () => {
  it("o mesmo jogo tem sempre a mesma capa", () => {
    const a = capaGerada("pc:Jogo Teste", "Jogo Teste");
    const b = capaGerada("pc:Jogo Teste", "Jogo Teste");
    expect(b).toEqual(a);
    expect(PALETAS).toContainEqual(a.paleta);
    expect(EMBLEMAS).toContain(a.emblema);
  });

  it("jogos diferentes espalham pelas paletas", () => {
    const ids = Array.from({ length: 40 }, (_, i) => `pc:Jogo ${i}`);
    const usadas = new Set(ids.map((id) => capaGerada(id, id).paleta.c1));
    expect(usadas.size).toBeGreaterThan(4);
  });

  it("título com dois-pontos vira chamada + título", () => {
    const c = capaGerada("x", "Reino Esmeralda: A Flauta do Tempo");
    expect(c.kicker).toBe("Reino Esmeralda");
    expect(c.titulo).toBe("A Flauta do Tempo");
    expect(tituloCurto("Reino Esmeralda: A Flauta do Tempo")).toBe("A Flauta do Tempo");
  });

  it("título sem dois-pontos fica inteiro", () => {
    const c = capaGerada("x", "Jogo Sem Dois Pontos");
    expect(c.kicker).toBeNull();
    expect(c.titulo).toBe("Jogo Sem Dois Pontos");
    expect(tituloCurto("Jogo Sem Dois Pontos")).toBe("Jogo Sem Dois Pontos");
  });

  it("título longo ou com ideogramas volta inteiro (quem corta é o CSS)", () => {
    const longo = "Ação & Reação Edição Especial ".repeat(4).trim();
    expect(capaGerada("y", longo).titulo).toBe(longo);
    expect(capaGerada("z", "星の旅人").titulo).toBe("星の旅人");
  });

  it("as 10 paletas do mockup, na ordem", () => {
    expect(PALETAS).toHaveLength(10);
    expect(PALETAS[0]).toEqual({ c0: "#04231c", c1: "#0e5a44", c2: "#1fae7a", c3: "#d4f57e" });
    expect(PALETAS[9]).toEqual({ c0: "#0e2a10", c1: "#3f8f2a", c2: "#facc15", c3: "#bae6fd" });
    expect(EMBLEMAS).toHaveLength(10);
  });
});
