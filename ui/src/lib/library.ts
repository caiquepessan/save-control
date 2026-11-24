// Regras da biblioteca: ordem, filtros, contagens, aviso do topo e a linha
// "origem · quando" de cada cartão.
import type { GameView } from "../api/gerado/GameView";
import type { VaultView } from "../api/gerado/VaultView";
import { T } from "../texto";
import { tempoRelativo } from "./format";

export type Chip = "todos" | "pc" | "recomp" | "emulador" | "atencao";
export type Fonte = "steam" | "epic" | "gog" | "pcOutros" | "recomp" | "emulador" | "manual" | "cofre";

export const CHIPS: Chip[] = ["todos", "pc", "recomp", "emulador", "atencao"];
export const FONTES: Fonte[] = ["steam", "epic", "gog", "pcOutros", "recomp", "emulador", "manual", "cofre"];

/** Cor do quadradinho de cada fonte (mockup A): menu lateral e resumo do primeiro uso. */
export const COR_FONTE: Record<Fonte, string> = {
  steam: "#5aa9ff",
  epic: "#c9ced8",
  gog: "#d18cff",
  pcOutros: "#8b95a8",
  recomp: "#3ee6a8",
  emulador: "#b18cff",
  manual: "#ffb547",
  cofre: "#6b7489",
};

export interface Filtro {
  chip: Chip;
  fonte: Fonte | null;
  busca: string;
}

export const FILTRO_INICIAL: Filtro = { chip: "todos", fonte: null, busca: "" };

const tempo = (iso: string | null) => (iso ? new Date(iso).getTime() : 0);

/** Com save (último save primeiro) → só no cofre (último backup) → sem save. */
export function ordenar(jogos: GameView[]): GameView[] {
  const grupo = (j: GameView) => (j.vaultOnly ? 1 : j.status === "semSave" ? 2 : 0);
  const chave = (j: GameView) => (j.vaultOnly ? tempo(j.lastBackup) : tempo(j.lastSaved));
  return [...jogos].sort(
    (a, b) => grupo(a) - grupo(b) || chave(b) - chave(a) || a.title.localeCompare(b.title, "pt-BR"),
  );
}

export function precisaAtencao(j: GameView): boolean {
  return (j.status === "semBackup" && !j.vaultOnly) || j.warnings.some((w) => w.kind === "pastaArriscada");
}

function noChip(j: GameView, chip: Chip): boolean {
  switch (chip) {
    case "todos":
      return true;
    case "atencao":
      return precisaAtencao(j);
    default:
      return j.kind === chip;
  }
}

function naFonte(j: GameView, fonte: Fonte): boolean {
  if (fonte === "cofre") return j.vaultOnly;
  if (j.vaultOnly) return false;
  switch (fonte) {
    case "steam":
    case "epic":
    case "gog":
      return j.sourceKey === fonte;
    case "pcOutros":
      return j.sourceKey === "pc";
    case "recomp":
    case "emulador":
    case "manual":
      return j.kind === fonte;
  }
}

/** Minúsculas e sem acento, para a busca. */
export function semAcento(s: string): string {
  return s.normalize("NFD").replace(/[̀-ͯ]/g, "").toLowerCase();
}

export function filtrar(jogos: GameView[], f: Filtro): GameView[] {
  const busca = semAcento(f.busca.trim());
  return jogos.filter(
    (j) =>
      noChip(j, f.chip) &&
      (!f.fonte || naFonte(j, f.fonte)) &&
      (!busca ||
        semAcento(j.title).includes(busca) ||
        semAcento(j.sourceLabel).includes(busca) ||
        j.roots.some((r) => semAcento(r.path).includes(busca))),
  );
}

export function contagens(jogos: GameView[]) {
  const chips = Object.fromEntries(CHIPS.map((c) => [c, jogos.filter((j) => noChip(j, c)).length])) as Record<Chip, number>;
  const fontes = Object.fromEntries(FONTES.map((f) => [f, jogos.filter((j) => naFonte(j, f)).length])) as Record<Fonte, number>;
  return { chips, fontes };
}

export type Alerta =
  | { tipo: "cofre"; caminho: string }
  | { tipo: "semBackup"; jogos: GameView[] }
  | { tipo: "arriscada"; jogos: GameView[] };

/** O aviso do topo: cofre fora > jogos sem backup > saves em pasta arriscada. */
export function alertaPrincipal(jogos: GameView[], cofre: VaultView | null): Alerta | null {
  if (cofre && !cofre.available) return { tipo: "cofre", caminho: cofre.path };
  const semBackup = jogos.filter((j) => j.status === "semBackup" && !j.vaultOnly);
  if (semBackup.length) return { tipo: "semBackup", jogos: semBackup };
  const arriscada = jogos.filter((j) => j.warnings.some((w) => w.kind === "pastaArriscada"));
  if (arriscada.length) return { tipo: "arriscada", jogos: arriscada };
  return null;
}

/** A linha "origem · quando" do cartão. */
export function subtitulo(j: GameView, agora: Date, abertos: Set<string>): string {
  let quando: string | null;
  if (abertos.has(j.id)) quando = T.cartao.jogandoAgora;
  else if (j.vaultOnly || j.status === "protegido") quando = j.lastBackup ? T.cartao.backup(tempoRelativo(j.lastBackup, agora)) : null;
  else if (j.status === "semSave") quando = T.cartao.nenhumSave;
  else quando = j.lastSaved ? tempoRelativo(j.lastSaved, agora) : null;
  return quando ? `${j.sourceLabel} · ${quando}` : j.sourceLabel;
}
