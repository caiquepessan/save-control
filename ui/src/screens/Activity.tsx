import { AlertTriangle, Info, RefreshCw, RotateCcw, Save, type LucideIcon } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { ErroApp } from "../api/backend";
import type { ActivityKind } from "../api/gerado/ActivityKind";
import type { ActivityView } from "../api/gerado/ActivityView";
import { dia, hora } from "../lib/format";
import { useApp } from "../store/app";
import { T } from "../texto";

/** Quantos eventos a tela pede ao app. */
const LIMITE = 200;

type Filtro = keyof typeof T.atividade.filtro;
const FILTROS: Filtro[] = ["todos", "backups", "restauracoes", "erros"];

function passa(f: Filtro, k: ActivityKind): boolean {
  switch (f) {
    case "todos":
      return true;
    case "backups":
      return k === "backupAuto" || k === "backupManual";
    case "restauracoes":
      return k === "restore";
    case "erros":
      return k === "error";
  }
}

const ICONE: Record<ActivityKind, LucideIcon> = {
  backupAuto: RefreshCw,
  backupManual: Save,
  restore: RotateCcw,
  error: AlertTriangle,
  notice: Info,
};

const CLASSE: Record<ActivityKind, string> = {
  backupAuto: "auto",
  backupManual: "manual",
  restore: "restauracao",
  error: "erro",
  notice: "aviso",
};

function texto(e: ActivityView): string {
  const tipo = T.atividade.tipos[e.kind];
  return e.title ? `${tipo} · ${e.title}` : tipo;
}

/** Agrupa por dia, do mais novo ao mais velho. */
function porDia(eventos: ActivityView[], agora: Date): { dia: string; eventos: ActivityView[] }[] {
  const grupos: { dia: string; eventos: ActivityView[] }[] = [];
  for (const e of eventos) {
    const d = dia(e.at, agora);
    const ultimo = grupos[grupos.length - 1];
    if (ultimo?.dia === d) ultimo.eventos.push(e);
    else grupos.push({ dia: d, eventos: [e] });
  }
  return grupos;
}

function Linha({ e, onAbrir }: { e: ActivityView; onAbrir: (() => void) | null }) {
  const Icone = ICONE[e.kind];
  const conteudo = (
    <>
      <b>{texto(e)}</b>
      <em>{e.detail}</em>
    </>
  );
  return (
    <li className={onAbrir ? `ti ativ clicavel ${CLASSE[e.kind]}` : `ti ativ ${CLASSE[e.kind]}`}>
      <i className="nd" aria-hidden="true">
        <Icone className="ic" />
      </i>
      {onAbrir ? (
        <button type="button" className="ativ-abrir" onClick={onAbrir}>
          {conteudo}
        </button>
      ) : (
        <div>{conteudo}</div>
      )}
      <time className="ativ-hora" dateTime={e.at}>
        {hora(e.at)}
      </time>
    </li>
  );
}

/**
 * Atividade (spec §15): backups, restaurações, erros e avisos, do mais novo ao
 * mais velho e agrupados por dia. Recarrega quando o app muda a biblioteca.
 */
export function Activity() {
  const backend = useApp((s) => s.backend);
  const avisar = useApp((s) => s.avisar);
  const abrirPainel = useApp((s) => s.abrirPainel);
  const jogos = useApp((s) => s.biblioteca?.games);
  const [eventos, setEventos] = useState<ActivityView[] | null>(null);
  const [filtro, setFiltro] = useState<Filtro>("todos");
  const montado = useRef(false);

  const carregar = useCallback(async () => {
    try {
      const lista = await backend.atividade(LIMITE);
      const ordenada = [...lista].sort((a, b) => new Date(b.at).getTime() - new Date(a.at).getTime());
      if (montado.current) setEventos(ordenada);
    } catch (e) {
      if (montado.current) avisar("erro", ErroApp.de(e).message);
    }
  }, [backend, avisar]);

  useEffect(() => {
    montado.current = true;
    void carregar();
    const parar = backend.aoMudarBiblioteca(() => void carregar());
    return () => {
      montado.current = false;
      parar();
    };
  }, [backend, carregar]);

  // Só abre o painel de jogo que ainda está na biblioteca.
  const naBiblioteca = useMemo(() => new Set((jogos ?? []).map((g) => g.id)), [jogos]);
  const contagem = useMemo(() => {
    const c = { todos: 0, backups: 0, restauracoes: 0, erros: 0 } satisfies Record<Filtro, number>;
    for (const e of eventos ?? []) for (const f of FILTROS) if (passa(f, e.kind)) c[f] += 1;
    return c;
  }, [eventos]);

  let corpo;
  if (!eventos) corpo = <p className="atividade-vazio">{T.atividade.carregando}</p>;
  else if (!eventos.length) corpo = <p className="atividade-vazio">{T.atividade.vazio}</p>;
  else {
    const grupos = porDia(
      eventos.filter((e) => passa(filtro, e.kind)),
      new Date(),
    );
    corpo = (
      <>
        <div className="chips" role="group" aria-label={T.atividade.filtros}>
          {FILTROS.map((f) => (
            <button key={f} type="button" className={filtro === f ? "chip on" : "chip"} aria-pressed={filtro === f} onClick={() => setFiltro(f)}>
              {T.atividade.filtro[f]} <b>{contagem[f]}</b>
            </button>
          ))}
        </div>
        <div className="atividade-rolagem">
          {grupos.length ? (
            grupos.map((g) => (
              <section key={g.dia} className="atividade-dia">
                <h2>{g.dia}</h2>
                <ol className="tl">
                  <span className="track" aria-hidden="true" />
                  {g.eventos.map((e, i) => {
                    const id = e.gameId;
                    const abrir = id && naBiblioteca.has(id) ? () => abrirPainel(id, { aba: "historico" }) : null;
                    return <Linha key={`${e.at}-${i}`} e={e} onAbrir={abrir} />;
                  })}
                </ol>
              </section>
            ))
          ) : (
            <p className="atividade-vazio">{T.atividade.vazioFiltro}</p>
          )}
        </div>
      </>
    );
  }

  return (
    <div className="atividade">
      <div className="hd">
        <h1 className="t">{T.atividade.titulo}</h1>
      </div>
      {corpo}
    </div>
  );
}
