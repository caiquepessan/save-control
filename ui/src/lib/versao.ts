// Como uma versão (backup) aparece no histórico e na confirmação de restaurar.
import type { SnapshotView } from "../api/gerado/SnapshotView";
import { T } from "../texto";
import { dataEHora, tamanho } from "./format";

/** O nome dado pelo usuário; sem nome, o tipo ("Automático"…). A mais antiga sem nome é o "Primeiro backup". */
export function tituloDaVersao(s: SnapshotView, maisAntiga: boolean): string {
  if (s.label) return s.label;
  if (maisAntiga) return T.versao.primeiro;
  return T.versao.titulos[s.kind];
}

/** "manual · hoje, 10:02 · 1,2 MB", com " · de NOTEBOOK" se veio de outro PC. */
export function detalheDaVersao(s: SnapshotView, agora: Date = new Date()): string {
  const partes = [T.versao.tipos[s.kind], dataEHora(s.createdAt, agora), tamanho(s.totalSize)];
  if (!s.fromThisPc) partes.push(T.versao.dePc(s.pcName));
  return partes.join(" · ");
}

/** Com estrela: versões com nome e automáticos fixados. */
export function temEstrela(s: SnapshotView): boolean {
  return !!s.label || s.kind === "fixado";
}
