import { Download, Plus, ScanLine, Search } from "lucide-react";
import { useEffect, useMemo, useRef } from "react";
import type { GameView } from "../api/gerado/GameView";
import { AlertBanner } from "../components/AlertBanner";
import { GameCard } from "../components/GameCard";
import { CHIPS, contagens, filtrar, ordenar } from "../lib/library";
import { useApp } from "../store/app";
import { T } from "../texto";

const NENHUM: GameView[] = [];
const INTERVALO_ABERTOS = 10_000;

/** Tela inicial: busca, filtros, aviso do topo e a grade de capas. */
export function Library() {
  const carregada = useApp((s) => s.biblioteca !== null);
  const jogos = useApp((s) => s.biblioteca?.games ?? NENHUM);
  const filtro = useApp((s) => s.filtro);
  const definirFiltro = useApp((s) => s.definirFiltro);
  const escaneando = useApp((s) => s.escaneando);
  const backupGeral = useApp((s) => s.backupGeral);
  const escanear = useApp((s) => s.escanear);
  const backupDeTudo = useApp((s) => s.backupDeTudo);
  const abrirDialogo = useApp((s) => s.abrirDialogo);
  const abertosLista = useApp((s) => s.abertos);
  const atualizarAbertos = useApp((s) => s.atualizarAbertos);
  const busca = useRef<HTMLInputElement>(null);

  // Ctrl+K: vai para a busca.
  useEffect(() => {
    const aoTeclar = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        busca.current?.focus();
        busca.current?.select();
      }
    };
    window.addEventListener("keydown", aoTeclar);
    return () => window.removeEventListener("keydown", aoTeclar);
  }, []);

  // "Jogando agora": confere os processos de tempos em tempos, só com a janela visível.
  useEffect(() => {
    const conferir = () => {
      if (document.visibilityState === "visible") void atualizarAbertos();
    };
    conferir();
    const h = setInterval(conferir, INTERVALO_ABERTOS);
    return () => clearInterval(h);
  }, [atualizarAbertos]);

  const c = useMemo(() => contagens(jogos), [jogos]);
  const visiveis = useMemo(() => ordenar(filtrar(jogos, filtro)), [jogos, filtro]);
  const abertos = useMemo(() => new Set(abertosLista), [abertosLista]);
  const protegidos = useMemo(() => jogos.filter((j) => j.status === "protegido" && !j.vaultOnly).length, [jogos]);
  const agora = new Date();

  return (
    <div className="biblioteca">
      <div className="topo">
        <label className="busca">
          <Search className="ic" aria-hidden="true" />
          <input
            ref={busca}
            type="search"
            placeholder={T.biblioteca.busca}
            aria-label={T.biblioteca.busca}
            value={filtro.busca}
            onChange={(e) => definirFiltro({ busca: e.target.value })}
          />
          <kbd>{T.biblioteca.atalhoBusca}</kbd>
        </label>
        <span className="cresce" />
        <button type="button" className="btn" disabled={escaneando} onClick={() => void escanear()}>
          <ScanLine className={escaneando ? "ic girando" : "ic"} aria-hidden="true" />
          {escaneando ? T.biblioteca.escaneando : T.biblioteca.escanear}
        </button>
        <button type="button" className="btn" onClick={() => abrirDialogo({ tipo: "adicionar" })}>
          <Plus className="ic" aria-hidden="true" />
          {T.biblioteca.adicionar}
        </button>
        <button type="button" className="btn pri" disabled={backupGeral} onClick={() => void backupDeTudo()}>
          <Download className="ic" aria-hidden="true" />
          {T.biblioteca.backupDeTudo}
        </button>
      </div>

      <div className="chips" role="group" aria-label="Filtros">
        {CHIPS.map((ch) => {
          const ativo = filtro.chip === ch && !filtro.fonte;
          return (
            <button
              key={ch}
              type="button"
              className={ativo ? "chip on" : "chip"}
              aria-pressed={ativo}
              onClick={() => definirFiltro({ chip: ch, fonte: null })}
            >
              {ch === "atencao" ? <i className="dot" aria-hidden="true" /> : null}
              {T.biblioteca.chips[ch]} <b>{c.chips[ch]}</b>
            </button>
          );
        })}
      </div>

      <AlertBanner />

      {carregada ? (
        <>
          <div className="hd">
            <h2 className="t">{T.biblioteca.titulo}</h2>
            <span className="s">{T.biblioteca.resumo(jogos.length, protegidos)}</span>
          </div>

          <div className="grade-rolagem">
            {visiveis.length ? (
              <div className="grade">
                {visiveis.map((j) => (
                  <GameCard key={j.id} jogo={j} abertos={abertos} agora={agora} />
                ))}
              </div>
            ) : (
              <p className="vazio">{jogos.length ? T.biblioteca.filtroVazio : T.biblioteca.vazia}</p>
            )}
          </div>
        </>
      ) : (
        <p className="vazio">{T.biblioteca.carregando}</p>
      )}
    </div>
  );
}
