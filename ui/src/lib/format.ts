// Datas, tamanhos e caminhos no jeito brasileiro.
import { T } from "../texto";

const MIN = 60_000;
const HORA = 60 * MIN;
const DIA = 24 * HORA;

function mesmoDia(a: Date, b: Date): boolean {
  return a.getFullYear() === b.getFullYear() && a.getMonth() === b.getMonth() && a.getDate() === b.getDate();
}

function foiOntem(d: Date, agora: Date): boolean {
  const ontem = new Date(agora);
  ontem.setDate(ontem.getDate() - 1);
  return mesmoDia(d, ontem);
}

function diaEMes(d: Date, agora: Date): string {
  const base = `${d.getDate()} ${T.tempo.meses[d.getMonth()]}`;
  return d.getFullYear() === agora.getFullYear() ? base : `${base} ${d.getFullYear()}`;
}

/** "agora", "há 12 min", "há 2 h", "ontem", "há 3 d", "há 1 sem", "14 out", "14 out 2025". */
export function tempoRelativo(iso: string, agora: Date = new Date()): string {
  const d = new Date(iso);
  const diff = agora.getTime() - d.getTime();
  if (diff < MIN) return T.tempo.agora;
  if (diff < HORA) return T.tempo.minutos(Math.floor(diff / MIN));
  if (diff < DIA && mesmoDia(d, agora)) return T.tempo.horas(Math.floor(diff / HORA));
  if (foiOntem(d, agora)) return T.tempo.ontem;
  if (diff < 7 * DIA) return T.tempo.dias(Math.max(1, Math.floor(diff / DIA)));
  if (diff < 35 * DIA) return T.tempo.semanas(Math.floor(diff / (7 * DIA)));
  return diaEMes(d, agora);
}

/** Cabeçalho de um dia: "Hoje", "Ontem", "24 nov", "24 nov 2025". */
export function dia(iso: string, agora: Date = new Date()): string {
  const d = new Date(iso);
  if (mesmoDia(d, agora)) return T.tempo.hojeTitulo;
  if (foiOntem(d, agora)) return T.tempo.ontemTitulo;
  return diaEMes(d, agora);
}

/** "09:05" (hora local). */
export function hora(iso: string): string {
  const d = new Date(iso);
  return `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
}

/** "hoje, 10:02", "ontem, 19:05", "19 nov, 15:02", "19 nov 2025, 15:02". */
export function dataEHora(iso: string, agora: Date = new Date()): string {
  const d = new Date(iso);
  const dia = mesmoDia(d, agora) ? T.tempo.hoje : foiOntem(d, agora) ? T.tempo.ontem : diaEMes(d, agora);
  return `${dia}, ${hora(iso)}`;
}

/** "512 B", "41 KB", "0,9 MB", "1,2 MB", "1,4 GB" (a partir de meio MB/GB, sobe de unidade). */
export function tamanho(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 512 * 1024) return `${Math.round(bytes / 1024)} KB`;
  const umaCasa = (v: number) => v.toFixed(1).replace(".", ",");
  if (bytes < 512 * 1024 ** 2) return `${umaCasa(bytes / 1024 ** 2)} MB`;
  return `${umaCasa(bytes / 1024 ** 3)} GB`;
}

/** Só as últimas pastas: "D:\Jogos\A\B\C\saves" → "…\B\C\saves". */
export function caminhoCurto(caminho: string, partes = 3): string {
  const sep = caminho.includes("\\") ? "\\" : "/";
  const p = caminho.split(/[\\/]/).filter(Boolean);
  if (p.length <= partes) return caminho;
  return `…${sep}${p.slice(-partes).join(sep)}`;
}
