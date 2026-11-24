import { RotateCcw, Star } from "lucide-react";
import type { GameView } from "../../api/gerado/GameView";
import type { SnapshotView } from "../../api/gerado/SnapshotView";
import { tempoRelativo } from "../../lib/format";
import { detalheDaVersao, temEstrela, tituloDaVersao } from "../../lib/versao";
import { useApp } from "../../store/app";
import { T } from "../../texto";

/** Linha do tempo: o save atual e os backups, do mais novo ao mais velho. */
export function HistoryTab({ jogo, versoes, modoRestaurar }: { jogo: GameView; versoes: SnapshotView[] | undefined; modoRestaurar: boolean }) {
  const iniciarRestauracao = useApp((s) => s.iniciarRestauracao);
  const agora = new Date();
  const temSaveAtual = !jogo.vaultOnly && jogo.status !== "semSave";
  const saveNovo = jogo.status === "saveNovo" || jogo.status === "semBackup";

  if (!versoes) return <p className="dvazio">{T.painel.carregando}</p>;

  return (
    <>
      {modoRestaurar && versoes.length ? <div className="faixa-escolha">{T.painel.escolhaVersao}</div> : null}
      <ol className="tl">
        {temSaveAtual || versoes.length ? <span className="track" aria-hidden="true" /> : null}
        {temSaveAtual ? (
          <li className="ti now">
            <i className="nd" aria-hidden="true" />
            <div>
              <b>{T.painel.saveAtual}</b>
              <em>{saveNovo && jogo.lastSaved ? T.painel.alterado(tempoRelativo(jogo.lastSaved, agora)) : T.painel.igualAoCofre}</em>
            </div>
            {saveNovo ? <span className="tag new">{T.painel.tagNovo}</span> : null}
          </li>
        ) : null}
        {versoes.map((v, i) => {
          const estrela = temEstrela(v);
          return (
            <li key={v.fileName} className={estrela ? "ti pin" : "ti"}>
              <i className="nd" aria-hidden="true">
                {estrela ? <Star className="ic" /> : null}
              </i>
              <div>
                <b>{tituloDaVersao(v, i === versoes.length - 1)}</b>
                <em>{detalheDaVersao(v, agora)}</em>
              </div>
              <button type="button" className="rb" onClick={() => iniciarRestauracao(jogo.id, v.fileName)}>
                <RotateCcw className="ic" aria-hidden="true" />
                {T.painel.restaurarVersao}
              </button>
            </li>
          );
        })}
      </ol>
      {versoes.length === 0 ? <p className="dvazio">{T.painel.semHistorico}</p> : null}
    </>
  );
}
