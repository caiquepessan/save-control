// Capas geradas pelo app: gradiente, emblema e título, sempre iguais para o
// mesmo jogo (paletas e emblemas do mockup aprovado).

export interface Paleta {
  c0: string;
  c1: string;
  c2: string;
  c3: string;
}

export const PALETAS: Paleta[] = [
  { c0: "#04231c", c1: "#0e5a44", c2: "#1fae7a", c3: "#d4f57e" },
  { c0: "#170a2b", c1: "#4a1a78", c2: "#c43ad6", c3: "#ffb36b" },
  { c0: "#3a1405", c1: "#b0480c", c2: "#ff9f1c", c3: "#ffe066" },
  { c0: "#0a1846", c1: "#1d4ed8", c2: "#ef4444", c3: "#fde047" },
  { c0: "#0a2814", c1: "#166534", c2: "#84cc16", c3: "#fef08a" },
  { c0: "#0a0a22", c1: "#2a1f6b", c2: "#7c3aed", c3: "#f5c86b" },
  { c0: "#06284a", c1: "#0ea5e9", c2: "#f472b6", c3: "#fde68a" },
  { c0: "#110303", c1: "#5b0a0a", c2: "#dc2626", c3: "#fca5a5" },
  { c0: "#050a15", c1: "#1e293b", c2: "#38bdf8", c3: "#e0f2fe" },
  { c0: "#0e2a10", c1: "#3f8f2a", c2: "#facc15", c3: "#bae6fd" },
];

/** Emblemas (viewBox 0 0 100 100): folha, colmeia, brilho, estrela, escudo, balão, lâmina, lua, gota, broto. */
export const EMBLEMAS: string[] = [
  "M50 4c26 14 40 40 30 66-6 16-18 24-30 26-12-2-24-10-30-26C10 44 24 18 50 4z",
  "M30 8l20 11.5v23L30 54 10 42.5v-23zM70 8l20 11.5v23L70 54 50 42.5v-23zM50 46l20 11.5v23L50 92 30 80.5v-23z",
  "M50 2l9 39 39 9-39 9-9 39-9-39-39-9 39-9z",
  "M50 4l13.5 28.5L94 36.5 71.5 58 77 89 50 74 23 89l5.5-31L6 36.5l30.5-4z",
  "M50 4l40 14v28c0 26-18 42-40 50C28 88 10 72 10 46V18z",
  "M20 40a30 36 0 1 0 60 0a30 36 0 1 0-60 0zM45 76h10l-5 8z",
  "M18 92 70 8h12L30 92zM42 92 86 20v18L54 92z",
  "M62 6a44 44 0 1 0 32 74A36 36 0 1 1 62 6z",
  "M50 4c18 26 34 44 34 62a34 34 0 0 1-68 0C16 48 32 30 50 4z",
  "M46 58C46 36 30 22 8 24c0 22 16 36 38 34zM54 58c0-26 16-42 40-42 0 26-16 42-40 42zM46 56h8v40h-8z",
];

/** FNV-1a de 32 bits: rápido e estável entre versões. */
function hash(texto: string): number {
  let h = 0x811c9dc5;
  for (let i = 0; i < texto.length; i++) {
    h ^= texto.charCodeAt(i);
    h = Math.imul(h, 0x01000193) >>> 0;
  }
  return h >>> 0;
}

/** "Reino Esmeralda: A Flauta do Tempo" → ["Reino Esmeralda", "A Flauta do Tempo"]. */
function dividir(titulo: string): [string | null, string] {
  const i = titulo.indexOf(": ");
  if (i <= 0 || i + 2 >= titulo.length) return [null, titulo];
  return [titulo.slice(0, i), titulo.slice(i + 2)];
}

export function tituloCurto(titulo: string): string {
  return dividir(titulo)[1];
}

export interface CapaGerada {
  paleta: Paleta;
  emblema: string;
  kicker: string | null;
  titulo: string;
}

export function capaGerada(id: string, titulo: string): CapaGerada {
  const h = hash(id);
  const [kicker, resto] = dividir(titulo);
  return { paleta: PALETAS[h % PALETAS.length], emblema: EMBLEMAS[Math.floor(h / 10) % EMBLEMAS.length], kicker, titulo: resto };
}
