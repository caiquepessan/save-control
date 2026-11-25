// Textos da confirmação de restaurar (mockup "Confirmação de restaurar" e spec §10).
import type { GameView } from "../api/gerado/GameView";
import type { RestorePreviewView } from "../api/gerado/RestorePreviewView";
import { T } from "../texto";
import { tituloCurto } from "./cover";
import { caminhoCurto, dataEHora, tamanho } from "./format";
import { tituloDaVersao } from "./versao";

export interface TextosRestauracao {
  titulo: string;
  corpo: string;
  versao: string;
  arquivos: string;
  destinos: { caminho: string; curto: string }[];
  /** Jogo aberto: o que fechar antes de restaurar. */
  aviso: string | null;
}

/** Nome para pedir que feche o jogo: o processo sem ".exe", ou o título curto. */
function nomeParaFechar(jogo: Pick<GameView, "title" | "processNames">): string {
  const processo = jogo.processNames[0];
  return processo ? processo.replace(/\.exe$/i, "") : tituloCurto(jogo.title);
}

export function textosDaRestauracao(
  previa: RestorePreviewView,
  jogo: Pick<GameView, "title" | "processNames">,
  /** `outraPasta`: a pessoa escolheu restaurar numa pasta qualquer (o save do jogo não muda). */
  opcoes: { maisAntiga: boolean; soArquivos?: number; outraPasta?: boolean },
  agora: Date = new Date(),
): TextosRestauracao {
  const s = previa.snapshot;
  const quando = dataEHora(s.createdAt, agora);
  return {
    titulo: T.restaurar.titulo(tituloDaVersao(s, opcoes.maisAntiga)),
    corpo: opcoes.outraPasta ? T.restaurar.corpoOutraPasta : T.restaurar.corpo(tituloCurto(jogo.title), quando),
    versao: `${quando} · ${T.versao.tipos[s.kind]}`,
    arquivos:
      opcoes.soArquivos !== undefined
        ? T.restaurar.algunsArquivos(opcoes.soArquivos, previa.fileCount)
        : `${T.painel.arquivos(previa.fileCount)} · ${tamanho(previa.totalSize)}`,
    destinos: previa.targets.map((t) => ({ caminho: t.path, curto: caminhoCurto(t.path) })),
    aviso: previa.running ? T.restaurar.jogoAberto(nomeParaFechar(jogo)) : null,
  };
}
