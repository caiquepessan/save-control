// Backend de demonstração: jogos fictícios para ver a interface no navegador
// (npm run dev) e para os testes. Nada aqui toca em arquivos de verdade.
import { ErroApp, type Backend } from "./backend";
import type { ActivityView } from "./gerado/ActivityView";
import type { AppPhase } from "./gerado/AppPhase";
import type { AppStateView } from "./gerado/AppStateView";
import type { AutoBackupView } from "./gerado/AutoBackupView";
import type { AutoStatusView } from "./gerado/AutoStatusView";
import type { AutoTimesView } from "./gerado/AutoTimesView";
import type { BackupResultView } from "./gerado/BackupResultView";
import type { GameView } from "./gerado/GameView";
import type { LibraryView } from "./gerado/LibraryView";
import type { RetentionView } from "./gerado/RetentionView";
import type { RootView } from "./gerado/RootView";
import type { SnapshotKindView } from "./gerado/SnapshotKindView";
import type { SnapshotView } from "./gerado/SnapshotView";
import type { VaultView } from "./gerado/VaultView";

export interface OpcoesDemo {
  /** Espera em ms antes de cada resposta (0 nos testes). */
  atraso?: number;
  fase?: AppPhase;
}

export interface DemoBackend extends Backend {
  emitirCapa(id: string, caminho: string): void;
  emitirFecharBloqueado(): void;
  emitirSituacao(s: AutoStatusView): void;
  emitirBibliotecaMudou(): void;
  emitirBackupAutomatico(itens: AutoBackupView[]): void;
  emitirAbrirJogo(id: string): void;
  emitirAviso(texto: string): void;
}

/** Ouvintes de um evento do app. */
function canal<A extends unknown[]>() {
  const ouvintes = new Set<(...a: A) => void>();
  return {
    ouvir(cb: (...a: A) => void) {
      ouvintes.add(cb);
      return () => {
        ouvintes.delete(cb);
      };
    },
    emitir(...a: A) {
      ouvintes.forEach((cb) => cb(...a));
    },
  };
}

export const IDS = {
  reino: "cat:reinorecomp",
  urso: "cat:ursorecomp",
  estrela: "cat:project64:estrela",
  academia: "pc:Academia Arcana",
  escudo: "cat:retroarch:escudo",
  baloes: "pc:Balões em Defesa 6",
  cavaleiro: "pc:Cavaleiro das Cavernas",
  vale: "pc:Vale das Sementes",
  laminas: "pc:Lâminas do Olimpo III",
  mascara: "cat:mascararecomp",
  fortaleza: "pc:Fortaleza Esquecida",
  acao: "man:acao",
  viajante: "pc:星の旅人",
  corrida: "pc:Corrida Pixel",
} as const;

const KB = 1024;
const MB = 1024 * KB;
const GB = 1024 * MB;

function pad(n: number): string {
  return String(n).padStart(2, "0");
}

function nomeZip(d: Date, kind: SnapshotKindView): string {
  const sufixo = { manual: "manual", auto: "auto", antesDeRestaurar: "antes-de-restaurar", fixado: "fixado" }[kind];
  const data = `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
  const hora = `${pad(d.getHours())}-${pad(d.getMinutes())}-${pad(d.getSeconds())}`;
  return `${data}_${hora}_ESTE-PC-1A2B_${sufixo}.zip`;
}

function base(id: string, title: string, extra: Partial<GameView>): GameView {
  return {
    id,
    title,
    originalTitle: title,
    kind: "pc",
    sourceKey: "steam",
    sourceLabel: "Steam",
    status: "protegido",
    vaultOnly: false,
    lastSaved: null,
    lastBackup: null,
    backupCount: 0,
    totalSize: 0,
    fileCount: 0,
    roots: [],
    warnings: [],
    cover: { kind: "gerada", path: null },
    processNames: [],
    gameDir: null,
    needsGameDir: false,
    isCustom: false,
    rememberedTargets: [],
    autoBackup: true,
    retention: null,
    includeConfig: null,
    ...extra,
  };
}

/** Uma raiz com arquivos; devolve também os totais. */
function raiz(path: string, template: string, arquivos: [string, number][], quando: string) {
  const roots: RootView[] = [{ path, template, files: arquivos.map(([relPath, size]) => ({ relPath, size, modified: quando })) }];
  return { roots, fileCount: arquivos.length, totalSize: arquivos.reduce((s, [, t]) => s + t, 0), lastSaved: quando };
}

function criarJogos(agora: number): GameView[] {
  const ha = (min: number) => new Date(agora - min * 60_000).toISOString();
  const H = 60;
  const D = 24 * H;
  const slots: [string, number][] = [0, 1, 2, 3, 4, 5].map((i) => [`slot_0${i}.sav`, 190 * KB]);
  return [
    base(IDS.reino, "Reino Esmeralda: A Flauta do Tempo", {
      kind: "recomp",
      sourceKey: "catalogo",
      sourceLabel: "Reino64: Recompiled",
      status: "saveNovo",
      lastBackup: ha(26 * H),
      backupCount: 5,
      processNames: ["Reino64Recompiled.exe"],
      ...raiz("C:\\Users\\voce\\AppData\\Local\\Reino64Recompiled\\saves", "<winLocalAppData>/Reino64Recompiled/saves", [...slots, ["option.sav", 41 * KB]], ha(3)),
    }),
    base(IDS.urso, "Urso & Pássaro", {
      kind: "recomp",
      sourceKey: "catalogo",
      sourceLabel: "Urso: Recompiled",
      status: "semBackup",
      warnings: [{ kind: "dentroDaPastaDoJogo", detail: null }],
      gameDir: "D:\\Jogos\\Urso Recompiled",
      rememberedTargets: [{ template: "<gameDir>/saves", path: "E:\\Saves\\Urso" }],
      ...raiz("D:\\Jogos\\Urso Recompiled\\saves", "<gameDir>/saves", [["save.bin", 512 * KB]], ha(41 * D)),
    }),
    base(IDS.estrela, "Estrela Saltitante 64", {
      kind: "emulador",
      sourceKey: "catalogo",
      sourceLabel: "Project64",
      status: "saveNovo",
      lastBackup: ha(2 * D),
      backupCount: 3,
      ...raiz("C:\\Program Files\\Project64 3.0\\Save", "<gameDir>/Save", [["ESTRELA SALTITANTE.eep", 2 * KB]], ha(12)),
    }),
    base(IDS.academia, "Academia Arcana", {
      lastBackup: ha(2 * H),
      backupCount: 8,
      warnings: [{ kind: "nuvem", detail: "Steam" }],
      ...raiz("C:\\Users\\voce\\AppData\\Local\\AcademiaArcana\\Saved\\SaveGames\\7656", "<winLocalAppData>/AcademiaArcana/Saved/SaveGames/<storeUserId>", [["perfil.sav", 14 * MB], ["config.sav", 3 * KB]], ha(3 * H)),
    }),
    base(IDS.escudo, "O Escudo do Passado", {
      kind: "emulador",
      sourceKey: "catalogo",
      sourceLabel: "RetroArch",
      lastBackup: ha(3 * D),
      backupCount: 4,
      ...raiz("C:\\RetroArch-Win64\\saves", "<gameDir>/saves", [["O Escudo do Passado.srm", 8 * KB]], ha(3 * D + 10)),
    }),
    base(IDS.baloes, "Balões em Defesa 6", {
      lastBackup: ha(D),
      backupCount: 6,
      ...raiz("C:\\Users\\voce\\AppData\\LocalLow\\Estudio\\BaloesTD6", "<winLocalAppDataLow>/Estudio/BaloesTD6", [["perfil.save", 220 * KB]], ha(D + H)),
    }),
    base(IDS.cavaleiro, "Cavaleiro das Cavernas", {
      lastBackup: ha(5 * D),
      backupCount: 3,
      ...raiz("C:\\Users\\voce\\AppData\\LocalLow\\Estudio\\Cavaleiro", "<winLocalAppDataLow>/Estudio/Cavaleiro", [["user1.dat", 380 * KB], ["user2.dat", 120 * KB]], ha(5 * D + H)),
    }),
    base(IDS.vale, "Vale das Sementes", {
      lastBackup: ha(8 * D),
      backupCount: 2,
      ...raiz("C:\\Users\\voce\\AppData\\Roaming\\ValeDasSementes\\Saves", "<winAppData>/ValeDasSementes/Saves", [["Fazenda_01", 1.1 * MB]], ha(8 * D + H)),
    }),
    base(IDS.laminas, "Lâminas do Olimpo III", { sourceKey: "pc", sourceLabel: "PC", status: "semSave" }),
    base(IDS.mascara, "Reino Esmeralda: Máscara da Lua", {
      kind: "recomp",
      sourceKey: "catalogo",
      sourceLabel: "Reino64: Recompiled",
      status: "semSave",
    }),
    base(IDS.fortaleza, "Fortaleza Esquecida", {
      vaultOnly: true,
      sourceLabel: "Só no cofre",
      status: "semSave",
      lastBackup: ha(20 * D),
      backupCount: 2,
      needsGameDir: true,
      roots: [{ path: "E:\\Jogos\\Fortaleza\\saves", template: "<gameDir>/saves", files: [] }],
    }),
    base(IDS.acao, "Ação & Reação: Edição Especial do Colecionador com Todos os Extras Inclusos", {
      kind: "manual",
      sourceKey: "manual",
      sourceLabel: "Adicionado à mão",
      status: "saveNovo",
      isCustom: true,
      lastBackup: ha(9 * D),
      backupCount: 2,
      warnings: [{ kind: "pastaArriscada", detail: "Downloads" }],
      processNames: ["AcaoReacao.exe"],
      ...raiz(
        "C:\\Users\\voce\\Downloads\\Acao e Reacao Edicao Especial\\Saves\\perfil-principal-com-um-nome-bem-comprido\\slots",
        "<home>/Downloads/Acao e Reacao Edicao Especial/Saves/perfil-principal-com-um-nome-bem-comprido/slots",
        [["slot1.dat", 64 * KB], ["slot2.dat", 64 * KB]],
        ha(50),
      ),
    }),
    base(IDS.viajante, "星の旅人", {
      sourceKey: "epic",
      sourceLabel: "Epic Games",
      lastBackup: ha(4 * D),
      backupCount: 2,
      ...raiz("C:\\Users\\voce\\Documents\\HoshiNoTabibito", "<winDocuments>/HoshiNoTabibito", [["save.dat", 96 * KB]], ha(4 * D + H)),
    }),
    base(IDS.corrida, "Corrida Pixel", {
      sourceKey: "gog",
      sourceLabel: "GOG",
      lastBackup: ha(12 * D),
      backupCount: 1,
      ...raiz("C:\\Users\\voce\\Saved Games\\CorridaPixel", "<home>/Saved Games/CorridaPixel", [["garagem.sav", 30 * KB]], ha(12 * D + H)),
    }),
  ];
}

/** Histórico de um jogo: do mais novo ao mais velho, um por dia. */
function versoes(g: GameView, especiais: Partial<SnapshotView>[] = []): SnapshotView[] {
  if (!g.backupCount || !g.lastBackup) return [];
  const t0 = new Date(g.lastBackup).getTime();
  return Array.from({ length: g.backupCount }, (_, i) => {
    const criado = new Date(t0 - i * 26 * 3_600_000);
    const kind: SnapshotKindView = i === 0 || i === g.backupCount - 1 ? "manual" : "auto";
    return {
      fileName: nomeZip(criado, kind),
      kind,
      label: null,
      createdAt: criado.toISOString(),
      pcName: "ESTE-PC",
      fromThisPc: true,
      totalSize: g.totalSize || 300 * KB,
      fileCount: g.fileCount || 2,
      ...(especiais[i] ?? {}),
    };
  });
}

const ESPECIAIS: Record<string, Partial<SnapshotView>[]> = {
  [IDS.reino]: [{ label: "Antes de atualizar o recomp" }],
  [IDS.academia]: [{}, { pcName: "NOTEBOOK-SALA", fromThisPc: false }],
};

/** A Atividade de exemplo, da mais nova à mais velha. */
function criarAtividade(agora: number): ActivityView[] {
  const ha = (min: number) => new Date(agora - min * 60_000).toISOString();
  return [
    { at: ha(4), kind: "backupAuto", gameId: IDS.reino, title: "Reino Esmeralda: A Flauta do Tempo", detail: "7 arquivos · 1,2 MB" },
    { at: ha(2 * 60), kind: "backupManual", gameId: IDS.academia, title: "Academia Arcana", detail: "2 arquivos · 14,0 MB" },
    {
      at: ha(3 * 60),
      kind: "error",
      gameId: IDS.estrela,
      title: "Estrela Saltitante 64",
      detail: "O arquivo ESTRELA SALTITANTE.eep estava em uso. Nova tentativa em 5 min.",
    },
    { at: ha(26 * 60), kind: "restore", gameId: IDS.baloes, title: "Balões em Defesa 6", detail: "1 arquivo restaurado · o save anterior ficou no histórico" },
    { at: ha(2 * 24 * 60), kind: "notice", gameId: null, title: null, detail: "O cofre voltou. O backup automático continua de onde parou." },
  ];
}

const RETENCAO_PADRAO: RetentionView = { autoRecent: 10, autoDailyDays: 7, autoWeeklyWeeks: 4 };
const TEMPOS_PADRAO: AutoTimesView = { quietSecs: 60, afterCloseSecs: 10, whileRunningMins: 15 };

export function createDemoBackend(opcoes: OpcoesDemo = {}): DemoBackend {
  const atraso = opcoes.atraso ?? 500;
  let fase: AppPhase = opcoes.fase ?? "pronto";
  const jogos = criarJogos(Date.now());
  const historicos = new Map<string, SnapshotView[]>(jogos.map((g) => [g.id, versoes(g, ESPECIAIS[g.id])]));
  const ignorados: GameView[] = [];
  const vault: VaultView = {
    path: "D:\\Save Control\\cofre",
    available: true,
    versions: 212,
    zipBytes: Math.round(1.4 * GB),
    diskTotal: 1000 * GB,
    diskFree: 620 * GB,
  };
  let cofre = vault.path;
  // Pastas que já têm cofre: um "HD de backup" de exemplo e, fora do primeiro uso
  // e do cofre sumido, o cofre padrão.
  const cofres = new Set<string>(["E:\\Backup\\cofre", ...(fase === "pronto" || fase === "falhou" ? [vault.path] : [])]);
  const temCofre = (pasta: string) => cofres.has(pasta);
  // Como o app: uma pasta que não é cofre nem se chama "cofre" finge ter outros
  // arquivos, então o cofre novo vai para a subpasta "cofre" dela.
  const destino = (pasta: string) => (temCofre(pasta) || /\\cofre$/i.test(pasta) ? pasta : `${pasta}\\cofre`);
  const config = {
    userRoots: ["D:\\Jogos"],
    includeConfig: false,
    steamCovers: true,
    manifestLoaded: true,
    autoBackup: true,
    retention: { ...RETENCAO_PADRAO },
    autoTimes: { ...TEMPOS_PADRAO },
    closeToTray: true,
    notifications: false,
    startWithWindows: false,
  };
  const abertos = new Set<string>([IDS.reino]);
  const ouvintesCapa = new Set<(id: string, caminho: string) => void>();
  const ouvintesFechar = new Set<() => void>();
  const atividade = criarAtividade(Date.now());
  const vigiados = () => jogos.filter((g) => !g.vaultOnly && g.roots.length > 0 && g.autoBackup).length;
  let situacao: AutoStatusView = {
    state: fase === "pronto" ? "ativo" : "parado",
    pausedUntil: null,
    watching: vigiados(),
    busy: false,
    lastBackup: { title: "Reino Esmeralda: A Flauta do Tempo", at: atividade[0].at },
  };
  const canais = {
    situacao: canal<[AutoStatusView]>(),
    biblioteca: canal<[]>(),
    backupAutomatico: canal<[AutoBackupView[]]>(),
    abrirJogo: canal<[string]>(),
    aviso: canal<[string]>(),
  };
  /** Como o trabalhador do app: muda a situação e avisa a janela. */
  const mudarSituacao = (s: Partial<AutoStatusView>) => {
    situacao = { ...situacao, ...s, watching: vigiados() };
    canais.situacao.emitir(copia(situacao));
  };
  /** O app sai do primeiro uso ou do cofre sumido: o automático começa. */
  const comecar = () => {
    if (situacao.state === "parado") mudarSituacao({ state: config.autoBackup ? "ativo" : "desligado" });
  };

  const espera = () => (atraso > 0 ? new Promise<void>((r) => setTimeout(r, atraso)) : Promise.resolve());
  const copia = <X>(x: X): X => structuredClone(x);
  const erro = (kind: ErroApp["kind"], message: string, root: string | null = null, candidates: string[] = []) =>
    new ErroApp({ kind, message, root, alias: root ? "r0" : null, candidates });
  const achar = (id: string): GameView => {
    const g = jogos.find((j) => j.id === id);
    if (!g) throw erro("naoEncontrado", `Não encontrado: jogo ${id}`);
    return g;
  };
  const versao = (id: string, arquivo: string): SnapshotView => {
    const s = (historicos.get(id) ?? []).find((v) => v.fileName === arquivo);
    if (!s) throw erro("naoEncontrado", `Não encontrado: backup ${arquivo}`);
    return s;
  };
  const estado = (): AppStateView => ({
    phase: fase,
    vaultDir: cofre,
    dataDir: "D:\\Save Control\\SaveControl-dados",
    defaultVault: "D:\\Save Control\\cofre",
    warnings: [],
    version: "0.1.0",
    error: fase === "falhou" ? "Arquivo inválido (jogos.json)" : null,
  });
  const biblioteca = (): LibraryView =>
    copia({ games: jogos, vault: { ...vault, path: cofre }, scannedAt: new Date().toISOString(), manifestLoaded: config.manifestLoaded });

  function backupDe(g: GameView, nome?: string | null): BackupResultView {
    if (g.fileCount === 0) return { outcome: "semArquivos", snapshot: null };
    const agora = new Date();
    const s: SnapshotView = {
      fileName: nomeZip(agora, "manual"),
      kind: "manual",
      label: nome?.trim() ? nome.trim() : null,
      createdAt: agora.toISOString(),
      pcName: "ESTE-PC",
      fromThisPc: true,
      totalSize: g.totalSize,
      fileCount: g.fileCount,
    };
    historicos.set(g.id, [s, ...(historicos.get(g.id) ?? [])]);
    g.status = "protegido";
    g.lastBackup = s.createdAt;
    g.backupCount += 1;
    vault.versions += 1;
    return { outcome: "feito", snapshot: copia(s) };
  }

  const backend: DemoBackend = {
    async estado() {
      await espera();
      return estado();
    },
    async biblioteca() {
      await espera();
      return biblioteca();
    },
    async escanear() {
      await espera();
      return biblioteca();
    },
    async fazerBackup(id, nome) {
      await espera();
      return backupDe(achar(id), nome);
    },
    async backupDeVarios(ids) {
      await espera();
      const r = { done: [] as string[], unchanged: [] as string[], failed: [] as { id: string; title: string; message: string }[] };
      for (const id of ids) {
        const g = jogos.find((j) => j.id === id);
        if (!g) {
          r.failed.push({ id, title: id, message: `Não encontrado: jogo ${id}` });
          continue;
        }
        const out = backupDe(g);
        (out.outcome === "feito" ? r.done : r.unchanged).push(id);
      }
      return r;
    },
    async jogosParaBackup() {
      await espera();
      return jogos.filter((g) => !g.vaultOnly && (g.status === "saveNovo" || g.status === "semBackup")).map((g) => g.id);
    },
    async historico(id) {
      await espera();
      return copia(historicos.get(id) ?? []);
    },
    async arquivosDoBackup(id, arquivo) {
      await espera();
      const g = achar(id);
      versao(id, arquivo);
      const arquivos = g.roots.flatMap((r) => r.files.map((f) => ({ root: "r0", relPath: f.relPath, size: f.size })));
      return arquivos.length
        ? arquivos
        : [
            { root: "r0", relPath: "save1.dat", size: 120 * KB },
            { root: "r0", relPath: "save2.dat", size: 80 * KB },
          ];
    },
    async previaRestauracao(id, arquivo, destinos) {
      await espera();
      const g = achar(id);
      const s = versao(id, arquivo);
      const r = g.roots[0];
      const template = r?.template ?? "<gameDir>/saves";
      if (g.needsGameDir && !destinos?.r0) {
        const candidatos = r?.path ? [r.path] : [];
        throw erro("precisaDestino", `Escolha onde restaurar ${template}`, template, candidatos);
      }
      return {
        snapshot: copia(s),
        targets: [{ alias: "r0", template, path: destinos?.r0 ?? r?.path ?? "" }],
        fileCount: s.fileCount,
        totalSize: s.totalSize,
        running: abertos.has(id),
        singleRoot: true,
      };
    },
    async restaurar(id, arquivo, pedido) {
      await espera();
      const g = achar(id);
      const s = versao(id, arquivo);
      if (abertos.has(id)) throw erro("jogoAberto", `Feche ${g.title} antes de restaurar`);
      const template = g.roots[0]?.template ?? "<gameDir>/saves";
      if (g.needsGameDir && !pedido.targets.r0) throw erro("precisaDestino", `Escolha onde restaurar ${template}`, template);
      let safety: SnapshotView | null = null;
      if (g.fileCount > 0) {
        const agora = new Date();
        safety = {
          fileName: nomeZip(agora, "antesDeRestaurar"),
          kind: "antesDeRestaurar",
          label: null,
          createdAt: agora.toISOString(),
          pcName: "ESTE-PC",
          fromThisPc: true,
          totalSize: g.totalSize,
          fileCount: g.fileCount,
        };
        historicos.set(id, [safety, ...(historicos.get(id) ?? [])]);
        g.backupCount += 1;
      }
      if (pedido.remember && pedido.targets.r0) g.rememberedTargets = [{ template, path: pedido.targets.r0 }];
      g.status = "protegido";
      return { written: pedido.onlyFiles?.length ?? s.fileCount, removed: 0, safety: copia(safety) };
    },
    async abrirPasta() {
      await espera();
    },
    async escolherPasta() {
      await espera();
      return "D:\\Jogos\\Pasta Escolhida";
    },
    async escolherImagem() {
      await espera();
      return "C:\\Imagens\\capa.png";
    },
    async adicionarJogo(pedido) {
      await espera();
      if (!pedido.name.trim()) throw erro("erro", "Dê um nome ao jogo.");
      if (!pedido.saveDirs.length) throw erro("erro", "Escolha pelo menos uma pasta do save.");
      const agora = new Date().toISOString();
      const g = base(`man:${Math.random().toString(36).slice(2, 10)}`, pedido.name.trim(), {
        kind: "manual",
        sourceKey: "manual",
        sourceLabel: "Adicionado à mão",
        status: "semBackup",
        isCustom: true,
        processNames: pedido.processes,
        gameDir: pedido.gameDir ?? null,
        cover: pedido.coverFile ? { kind: "propria", path: pedido.coverFile } : { kind: "gerada", path: null },
        ...raiz(pedido.saveDirs[0], pedido.saveDirs[0], [["save.dat", 2 * KB]], agora),
      });
      jogos.unshift(g);
      historicos.set(g.id, []);
      return copia(g);
    },
    async renomearJogo(id, titulo) {
      await espera();
      const g = achar(id);
      g.title = titulo?.trim() ? titulo.trim() : g.originalTitle;
    },
    async ignorarJogo(id, ignorado) {
      await espera();
      if (ignorado) {
        const i = jogos.findIndex((j) => j.id === id);
        if (i >= 0) ignorados.push(...jogos.splice(i, 1));
      } else {
        const i = ignorados.findIndex((j) => j.id === id);
        if (i >= 0) jogos.push(...ignorados.splice(i, 1));
      }
    },
    async definirPastaDoJogo(id, pasta) {
      await espera();
      const g = achar(id);
      g.gameDir = pasta;
      g.needsGameDir = g.roots.some((r) => r.template.includes("<gameDir>")) && !pasta;
    },
    async definirCapa(id, arquivo) {
      await espera();
      achar(id).cover = arquivo ? { kind: "propria", path: arquivo } : { kind: "gerada", path: null };
    },
    async esquecerDestino(id, template) {
      await espera();
      const g = achar(id);
      g.rememberedTargets = g.rememberedTargets.filter((t) => t.template !== template);
    },
    async removerJogoManual(id) {
      await espera();
      const g = achar(id);
      if (!g.isCustom) throw erro("erro", "Só jogos adicionados à mão podem ser removidos.");
      jogos.splice(jogos.indexOf(g), 1);
      ignorados.push(g);
    },
    async configuracoes() {
      await espera();
      return {
        vaultDir: cofre,
        userRoots: [...config.userRoots],
        includeConfig: config.includeConfig,
        steamCovers: config.steamCovers,
        manifestLoaded: config.manifestLoaded,
        autoBackup: config.autoBackup,
        retention: { ...config.retention },
        autoTimes: { ...config.autoTimes },
        closeToTray: config.closeToTray,
        notifications: config.notifications,
        startWithWindows: config.startWithWindows,
        ignored: ignorados.map((g) => ({ id: g.id, title: g.title })),
        version: "0.1.0",
      };
    },
    async definirPastasDeJogos(pastas) {
      await espera();
      config.userRoots = [...pastas];
    },
    async definirIncluirConfig(ligado) {
      await espera();
      config.includeConfig = ligado;
    },
    async definirCapasSteam(ligado) {
      await espera();
      config.steamCovers = ligado;
    },
    async trocarCofre(pasta, criar) {
      await espera();
      const p = destino(pasta);
      if (!criar && !temCofre(p)) throw erro("naoEncontrado", "Não há um cofre nessa pasta.");
      cofres.add(p);
      cofre = p;
      fase = "pronto";
      comecar();
      return estado();
    },
    async inspecionarCofre(pasta) {
      await espera();
      const p = destino(pasta);
      return temCofre(p) ? { exists: true, versions: 212, path: p } : { exists: false, versions: 0, path: p };
    },
    async tentarCofreDeNovo() {
      await espera();
      // Na demonstração, o HD "volta" na primeira tentativa.
      cofres.add(cofre);
      fase = "pronto";
      comecar();
      return estado();
    },
    async atualizarBase() {
      await espera();
      config.manifestLoaded = true;
      return { outcome: "atualizada", message: null };
    },
    async concluirPrimeiroUso(pastaDoCofre) {
      await espera();
      const p = destino(pastaDoCofre);
      cofres.add(p);
      cofre = p;
      fase = "pronto";
      comecar();
      return estado();
    },
    async jogosAbertos() {
      await espera();
      return [...abertos];
    },
    async situacaoAutomatico() {
      await espera();
      return copia({ ...situacao, watching: vigiados() });
    },
    async definirAutomatico(ligado) {
      await espera();
      config.autoBackup = ligado;
      mudarSituacao({ state: ligado ? "ativo" : "desligado", pausedUntil: null });
    },
    async pausarAutomatico(minutos) {
      await espera();
      mudarSituacao({ state: "pausado", pausedUntil: minutos ? new Date(Date.now() + minutos * 60_000).toISOString() : null });
    },
    async retomarAutomatico() {
      await espera();
      mudarSituacao({ state: config.autoBackup ? "ativo" : "desligado", pausedUntil: null });
    },
    async definirAutomaticoDoJogo(id, ligado) {
      await espera();
      achar(id).autoBackup = ligado;
      mudarSituacao({});
    },
    async definirRetencao(politica) {
      await espera();
      config.retention = { ...politica };
    },
    async definirRetencaoDoJogo(id, politica) {
      await espera();
      achar(id).retention = politica ? { ...politica } : null;
    },
    async definirTemposDoAutomatico(tempos) {
      await espera();
      const segundos = (n: number) => n >= 5 && n <= 600;
      if (!segundos(tempos.quietSecs) || !segundos(tempos.afterCloseSecs)) throw erro("erro", "Use um valor entre 5 e 600 segundos.");
      if (tempos.whileRunningMins < 5 || tempos.whileRunningMins > 120) throw erro("erro", "Use um valor entre 5 e 120 minutos.");
      config.autoTimes = { ...tempos };
    },
    async definirIncluirConfigDoJogo(id, valor) {
      await espera();
      achar(id).includeConfig = valor;
    },
    async fixarBackup(id, arquivo, fixado) {
      await espera();
      achar(id);
      const s = versao(id, arquivo);
      if (fixado && s.kind === "auto") {
        s.kind = "fixado";
        s.fileName = s.fileName.replace(/_auto\.zip$/, "_fixado.zip");
      } else if (!fixado && s.kind === "fixado") {
        s.kind = "auto";
        s.fileName = s.fileName.replace(/_fixado\.zip$/, "_auto.zip");
      } else {
        throw erro("naoEncontrado", `Não encontrado: backup automático ${arquivo}`);
      }
      return copia(s);
    },
    async apagarBackup(id, arquivo) {
      await espera();
      const g = achar(id);
      versao(id, arquivo);
      historicos.set(id, (historicos.get(id) ?? []).filter((v) => v.fileName !== arquivo));
      g.backupCount = Math.max(0, g.backupCount - 1);
      vault.versions = Math.max(0, vault.versions - 1);
      if (g.backupCount === 0) {
        g.lastBackup = null;
        if (g.fileCount > 0) g.status = "semBackup";
      }
      atividade.unshift({ at: new Date().toISOString(), kind: "notice", gameId: id, title: g.title, detail: "Backup apagado" });
    },
    async atividade(limite) {
      await espera();
      return copia(atividade.slice(0, limite ?? 200));
    },
    async definirFecharParaBandeja(ligado) {
      await espera();
      config.closeToTray = ligado;
    },
    async definirIniciarComWindows(ligado) {
      await espera();
      config.startWithWindows = ligado;
    },
    async definirNotificacoes(ligado) {
      await espera();
      config.notifications = ligado;
    },
    async removerRegistrosDoWindows() {
      await espera();
      config.startWithWindows = false;
      config.notifications = false;
    },
    async jogoParaAbrir() {
      await espera();
      return null;
    },
    aoReceberCapa(cb) {
      ouvintesCapa.add(cb);
      return () => {
        ouvintesCapa.delete(cb);
      };
    },
    aoBloquearFechar(cb) {
      ouvintesFechar.add(cb);
      return () => {
        ouvintesFechar.delete(cb);
      };
    },
    aoMudarSituacao: (cb) => canais.situacao.ouvir(cb),
    aoMudarBiblioteca: (cb) => canais.biblioteca.ouvir(cb),
    aoFazerBackupAutomatico: (cb) => canais.backupAutomatico.ouvir(cb),
    aoAbrirJogo: (cb) => canais.abrirJogo.ouvir(cb),
    aoAvisar: (cb) => canais.aviso.ouvir(cb),
    srcDaImagem: (caminho) => caminho,
    janela: {
      async minimizar() {},
      async alternarMaximizar() {},
      async fechar() {},
      async estaMaximizada() {
        return false;
      },
    },
    emitirCapa(id, caminho) {
      const g = jogos.find((j) => j.id === id);
      if (g) g.cover = { kind: "steam", path: caminho };
      ouvintesCapa.forEach((cb) => cb(id, caminho));
    },
    emitirFecharBloqueado() {
      ouvintesFechar.forEach((cb) => cb());
    },
    emitirSituacao(s) {
      situacao = { ...s };
      canais.situacao.emitir(copia(situacao));
    },
    emitirBibliotecaMudou() {
      canais.biblioteca.emitir();
    },
    emitirBackupAutomatico(itens) {
      canais.backupAutomatico.emitir(copia(itens));
    },
    emitirAbrirJogo(id) {
      canais.abrirJogo.emitir(id);
    },
    emitirAviso(texto) {
      canais.aviso.emitir(texto);
    },
  };
  return backend;
}
