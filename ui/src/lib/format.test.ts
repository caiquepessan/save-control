import { caminhoCurto, dataEHora, tamanho, tempoRelativo } from "./format";

// Datas locais, para não depender do fuso de quem roda os testes.
const d = (a: number, m: number, dia: number, h = 0, min = 0, s = 0) => new Date(a, m - 1, dia, h, min, s);
const iso = (x: Date) => x.toISOString();

describe("tempoRelativo", () => {
  const agora = d(2026, 11, 23, 12, 0);

  it("menos de um minuto é agora", () => {
    expect(tempoRelativo(iso(d(2026, 11, 23, 11, 59, 30)), agora)).toBe("agora");
  });
  it("minutos e horas", () => {
    expect(tempoRelativo(iso(d(2026, 11, 23, 11, 48)), agora)).toBe("há 12 min");
    expect(tempoRelativo(iso(d(2026, 11, 23, 10, 0)), agora)).toBe("há 2 h");
  });
  it("logo depois da meia-noite ainda conta em minutos", () => {
    expect(tempoRelativo(iso(d(2026, 11, 22, 23, 50)), d(2026, 11, 23, 0, 30))).toBe("há 40 min");
  });
  it("dia anterior é ontem", () => {
    expect(tempoRelativo(iso(d(2026, 11, 22, 10, 0)), d(2026, 11, 23, 9, 0))).toBe("ontem");
  });
  it("dias e semanas", () => {
    expect(tempoRelativo(iso(d(2026, 11, 20, 12, 0)), agora)).toBe("há 3 d");
    expect(tempoRelativo(iso(d(2026, 11, 15, 12, 0)), agora)).toBe("há 1 sem");
  });
  it("mais antigo vira data", () => {
    expect(tempoRelativo(iso(d(2026, 10, 14, 12, 0)), agora)).toBe("14 out");
    expect(tempoRelativo(iso(d(2025, 10, 14, 12, 0)), agora)).toBe("14 out 2025");
  });
});

describe("dataEHora", () => {
  const agora = d(2026, 11, 23, 12, 0);
  it("hoje, ontem, este ano e outro ano", () => {
    expect(dataEHora(iso(d(2026, 11, 23, 10, 2)), agora)).toBe("hoje, 10:02");
    expect(dataEHora(iso(d(2026, 11, 22, 19, 5)), agora)).toBe("ontem, 19:05");
    expect(dataEHora(iso(d(2026, 11, 19, 15, 2)), agora)).toBe("19 nov, 15:02");
    expect(dataEHora(iso(d(2025, 11, 19, 15, 2)), agora)).toBe("19 nov 2025, 15:02");
  });
});

describe("tamanho", () => {
  it("bytes, KB, MB e GB no jeito brasileiro", () => {
    expect(tamanho(0)).toBe("0 B");
    expect(tamanho(512)).toBe("512 B");
    expect(tamanho(41 * 1024)).toBe("41 KB");
    expect(tamanho(Math.round(0.9 * 1024 * 1024))).toBe("0,9 MB");
    expect(tamanho(Math.round(1.2 * 1024 * 1024))).toBe("1,2 MB");
    expect(tamanho(Math.round(1.4 * 1024 * 1024 * 1024))).toBe("1,4 GB");
  });
});

describe("caminhoCurto", () => {
  it("mostra só as últimas pastas", () => {
    expect(caminhoCurto("D:\\Jogos\\A\\B\\C\\saves")).toBe("…\\B\\C\\saves");
    expect(caminhoCurto("D:\\Jogos\\saves")).toBe("D:\\Jogos\\saves");
  });
});
