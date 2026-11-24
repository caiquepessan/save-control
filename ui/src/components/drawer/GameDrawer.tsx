import { AlertTriangle, Download, Folder, FolderOpen, Info, MoreHorizontal, RotateCcw, X } from "lucide-react";
import { useEffect, useRef, type KeyboardEvent as ReactKeyboardEvent } from "react";
import type { GameView } from "../../api/gerado/GameView";
import { tamanho } from "../../lib/format";
import { useApp, type Aba, type Painel } from "../../store/app";
import { T } from "../../texto";
import { Cover } from "../Cover";
import { StatusPill } from "../StatusPill";
import { FilesTab } from "./FilesTab";
import { GameSettingsTab } from "./GameSettingsTab";
import { HistoryTab } from "./HistoryTab";

const ABAS: Aba[] = ["historico", "arquivos", "ajustes"];
const FOCAVEIS = 'button:not([disabled]), [href], input:not([disabled]), select, textarea, [tabindex]:not([tabindex="-1"])';

function chamada(j: GameView): string {
  if (j.vaultOnly) return T.painel.soNoCofre;
  if (j.kind === "manual") return T.painel.adicionadoAMao;
  return `${T.painel.tipo[j.kind]} · ${j.sourceLabel}`;
}

/** Os avisos do jogo (§15), na caixa amarela do mockup. */
function Avisos({ jogo }: { jogo: GameView }) {
  const avisos = jogo.warnings.filter((w) => w.kind !== "nuvem");
  if (!avisos.length) return null;
  return (
    <div className="davisos">
      {avisos.map((w, i) => {
        let conteudo;
        if (w.kind === "pastaArriscada") {
          conteudo = (
            <>
              <b>{T.avisos.arriscadaForte(w.detail ?? "")}</b> {T.avisos.arriscada}
              {jogo.backupCount > 0 ? ` ${T.avisos.copias(jogo.backupCount)}` : ""}
            </>
          );
        } else if (w.kind === "dentroDaPastaDoJogo") {
          conteudo = (
            <>
              <b>{T.avisos.pastaDoJogoForte}</b> {T.avisos.pastaDoJogo}
            </>
          );
        } else if (w.kind === "registro") {
          conteudo = T.avisos.registro;
        } else {
          conteudo = T.avisos.pastaAmpla(w.detail ?? "");
        }
        const informativo = w.kind === "registro" || w.kind === "pastaAmpla";
        return (
          <div key={i} className={informativo ? "warnbox info" : "warnbox"} role="note">
            {informativo ? <Info className="ic" aria-hidden="true" /> : <AlertTriangle className="ic" aria-hidden="true" />}
            <span>{conteudo}</span>
          </div>
        );
      })}
    </div>
  );
}

function Conteudo({ jogo, painel }: { jogo: GameView; painel: Painel }) {
  const fechar = useApp((s) => s.fecharPainel);
  const definirAba = useApp((s) => s.definirAba);
  const carregarHistorico = useApp((s) => s.carregarHistorico);
  const versoes = useApp((s) => s.historicos[jogo.id]);
  const aberto = useApp((s) => s.abertos.includes(jogo.id));
  const pendente = useApp((s) => s.pendentes[jogo.id]);
  const restauracaoAberta = useApp((s) => s.restauracao !== null);
  const fazerBackup = useApp((s) => s.fazerBackup);
  const abrirPasta = useApp((s) => s.backend.abrirPasta);
  const caixa = useRef<HTMLElement>(null);
  const botaoFechar = useRef<HTMLButtonElement>(null);
  const abas = useRef<Record<Aba, HTMLButtonElement | null>>({ historico: null, arquivos: null, ajustes: null });

  // Foco entra no painel ao abrir e volta para onde estava ao fechar.
  useEffect(() => {
    const anterior = document.activeElement as HTMLElement | null;
    botaoFechar.current?.focus();
    return () => anterior?.focus();
  }, [jogo.id]);

  useEffect(() => {
    void carregarHistorico(jogo.id);
  }, [jogo.id, carregarHistorico]);

  // Esc fecha (a menos que a confirmação de restaurar esteja por cima).
  useEffect(() => {
    const aoTeclar = (e: KeyboardEvent) => {
      if (e.key === "Escape" && !restauracaoAberta) fechar();
    };
    window.addEventListener("keydown", aoTeclar);
    return () => window.removeEventListener("keydown", aoTeclar);
  }, [fechar, restauracaoAberta]);

  // O Tab não sai do painel enquanto ele está aberto.
  const prenderFoco = (e: ReactKeyboardEvent) => {
    if (e.key !== "Tab" || !caixa.current) return;
    const itens = Array.from(caixa.current.querySelectorAll<HTMLElement>(FOCAVEIS));
    if (!itens.length) return;
    const [primeiro, ultimo] = [itens[0], itens[itens.length - 1]];
    if (e.shiftKey && document.activeElement === primeiro) {
      e.preventDefault();
      ultimo.focus();
    } else if (!e.shiftKey && document.activeElement === ultimo) {
      e.preventDefault();
      primeiro.focus();
    }
  };

  const setasNasAbas = (e: ReactKeyboardEvent) => {
    const i = ABAS.indexOf(painel.aba);
    const proxima = e.key === "ArrowRight" ? ABAS[(i + 1) % ABAS.length] : e.key === "ArrowLeft" ? ABAS[(i + ABAS.length - 1) % ABAS.length] : null;
    if (!proxima) return;
    e.preventDefault();
    definirAba(proxima);
    abas.current[proxima]?.focus();
  };

  const raiz = jogo.roots[0]?.path ?? null;
  const nuvens = jogo.warnings.filter((w) => w.kind === "nuvem");
  const contagemAba: Record<Aba, number | null> = {
    historico: versoes?.length ?? jogo.backupCount,
    arquivos: jogo.fileCount,
    ajustes: null,
  };

  return (
    <>
      <div className="painel-fundo" data-testid="fundo-painel" onClick={fechar} />
      <aside className="painel" role="dialog" aria-modal="true" aria-label={jogo.title} ref={caixa} onKeyDown={prenderFoco}>
        <Cover jogo={jogo} variante="painel" className="dhero">
          <StatusPill jogo={jogo} />
          <button type="button" className="dx" ref={botaoFechar} aria-label={T.painel.fechar} onClick={fechar}>
            <X className="ic" aria-hidden="true" />
          </button>
          <div className="dhin">
            <div className="kick">{chamada(jogo)}</div>
            <h2 className="ht">{jogo.title}</h2>
            <div className="hchips">
              {aberto ? (
                <span>
                  <i className="vivo" aria-hidden="true" />
                  {T.painel.jogandoAgora}
                </span>
              ) : null}
              {jogo.fileCount > 0 ? <span>{T.painel.arquivos(jogo.fileCount)}</span> : null}
              {jogo.totalSize > 0 ? <span>{tamanho(jogo.totalSize)}</span> : null}
              {nuvens.map((n) => (
                <span key={n.detail}>{T.painel.nuvem(n.detail ?? "")}</span>
              ))}
            </div>
          </div>
        </Cover>

        <div className="dact">
          {jogo.fileCount > 0 && !jogo.vaultOnly ? (
            <button type="button" className="btn pri" disabled={!!pendente} onClick={() => void fazerBackup(jogo.id)}>
              <Download className="ic" aria-hidden="true" />
              {T.painel.fazerBackup}
            </button>
          ) : null}
          {jogo.backupCount > 0 ? (
            <button type="button" className="btn" onClick={() => definirAba("historico")}>
              <RotateCcw className="ic" aria-hidden="true" />
              {T.painel.restaurar}
            </button>
          ) : null}
          <span className="cresce" />
          {raiz ? (
            <button type="button" className="ibtn" aria-label={T.painel.abrirPastaDoSave} title={T.painel.abrirPastaDoSave} onClick={() => void abrirPasta(raiz)}>
              <Folder className="ic" aria-hidden="true" />
            </button>
          ) : null}
          <button type="button" className="ibtn" aria-label={T.painel.maisAcoes} title={T.painel.maisAcoes}>
            <MoreHorizontal className="ic" aria-hidden="true" />
          </button>
        </div>

        <Avisos jogo={jogo} />

        <div className="dtabs" role="tablist" aria-label={jogo.title} onKeyDown={setasNasAbas}>
          {ABAS.map((a) => {
            const ativa = painel.aba === a;
            const n = contagemAba[a];
            return (
              <button
                key={a}
                ref={(el) => {
                  abas.current[a] = el;
                }}
                type="button"
                role="tab"
                id={`aba-${a}`}
                aria-selected={ativa}
                aria-controls={`painel-${a}`}
                tabIndex={ativa ? 0 : -1}
                className={ativa ? "dtab on" : "dtab"}
                onClick={() => definirAba(a)}
              >
                {T.painel.abas[a]}
                {n !== null ? <b>{n}</b> : null}
              </button>
            );
          })}
        </div>

        <div className="dbody" role="tabpanel" id={`painel-${painel.aba}`} aria-labelledby={`aba-${painel.aba}`}>
          {painel.aba === "historico" ? <HistoryTab jogo={jogo} versoes={versoes} modoRestaurar={painel.modoRestaurar} /> : null}
          {painel.aba === "arquivos" ? <FilesTab jogo={jogo} /> : null}
          {painel.aba === "ajustes" ? <GameSettingsTab jogo={jogo} /> : null}
        </div>

        {raiz ? (
          <div className="dfoot">
            <FolderOpen className="ic" aria-hidden="true" />
            <span className="mono" title={raiz}>
              {raiz}
            </span>
            <button type="button" className="lnk" onClick={() => void abrirPasta(raiz)}>
              {T.painel.abrirPasta}
            </button>
          </div>
        ) : null}
      </aside>
    </>
  );
}

/** Painel lateral do jogo: desliza da direita sobre a biblioteca escurecida. */
export function GameDrawer() {
  const painel = useApp((s) => s.painel);
  const jogo = useApp((s) => (s.painel ? s.biblioteca?.games.find((g) => g.id === s.painel?.id) : undefined));
  if (!painel || !jogo) return null;
  return <Conteudo key={jogo.id} jogo={jogo} painel={painel} />;
}
