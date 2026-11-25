import { RotateCcw, Star, Trash2 } from "lucide-react";
import type { GameView } from "../../api/gerado/GameView";
import type { SnapshotView } from "../../api/gerado/SnapshotView";
import { tempoRelativo } from "../../lib/format";
import { detalheDaVersao, temEstrela, tituloDaVersao } from "../../lib/versao";
import { useApp } from "../../store/app";
import { T } from "../../texto";

/** Linha do tempo: o save atual e os backups, do mais novo ao mais velho. */
export function HistoryTab({ jogo, versoes, modoRestaurar }: { jogo: GameView; versoes: SnapshotView[] | undefined; modoRestaurar: boolean }) {
  const iniciarRestauracao = useApp((s) => s.iniciarRestauracao);
  const fixarBackup = useApp((s) => s.fixarBackup);
  const abrirDialogo = useApp((s) => s.abrirDialogo);
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
          // Só automáticos podem ser fixados (e os fixados, desafixados).
          const fixavel = v.kind === "auto" || v.kind === "fixado";
          const rotuloFixar = v.kind === "fixado" ? T.historico.desafixar : T.historico.fixar;
          return (
            <li key={v.fileName} className={estrela ? "ti pin" : "ti"}>
              <i className="nd" aria-hidden="true">
                {estrela ? <Star className="ic" /> : null}
              </i>
              <div>
                <b>{tituloDaVersao(v, i === versoes.length - 1)}</b>
                <em>{detalheDaVersao(v, agora)}</em>
              </div>
              <div className="ti-acoes">
                {fixavel ? (
                  <button
                    type="button"
                    className={v.kind === "fixado" ? "ti-ib fixado" : "ti-ib"}
                    aria-label={rotuloFixar}
                    title={rotuloFixar}
                    onClick={() => void fixarBackup(jogo.id, v.fileName, v.kind !== "fixado")}
                  >
                    <Star className="ic" aria-hidden="true" />
                  </button>
                ) : null}
                <button
                  type="button"
                  className="ti-ib"
                  aria-label={T.historico.apagar}
                  title={T.historico.apagar}
                  onClick={() => abrirDialogo({ tipo: "apagarBackup", id: jogo.id, arquivo: v.fileName })}
                >
                  <Trash2 className="ic" aria-hidden="true" />
                </button>
                <button type="button" className="rb" onClick={() => iniciarRestauracao(jogo.id, v.fileName)}>
                  <RotateCcw className="ic" aria-hidden="true" />
                  {T.painel.restaurarVersao}
                </button>
              </div>
            </li>
          );
        })}
      </ol>
      {versoes.length === 0 ? <p className="dvazio">{T.painel.semHistorico}</p> : null}
    </>
  );
}
