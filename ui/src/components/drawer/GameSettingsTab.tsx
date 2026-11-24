import { AlertTriangle, EyeOff } from "lucide-react";
import type { GameView } from "../../api/gerado/GameView";
import { useApp } from "../../store/app";
import { T } from "../../texto";

/** Ajustes deste jogo neste PC: pasta do jogo, destinos lembrados e ignorar. */
export function GameSettingsTab({ jogo }: { jogo: GameView }) {
  const escolherPasta = useApp((s) => s.backend.escolherPasta);
  const definirPastaDoJogo = useApp((s) => s.definirPastaDoJogo);
  const esquecerDestino = useApp((s) => s.esquecerDestino);
  const ignorar = useApp((s) => s.ignorar);

  const escolher = async () => {
    const pasta = await escolherPasta(T.ajustesJogo.escolherTitulo);
    if (pasta) await definirPastaDoJogo(jogo.id, pasta);
  };

  return (
    <div className="ajustes-jogo">
      <section className="ajuste">
        <b className="ajuste-titulo">{T.ajustesJogo.pastaDoJogo}</b>
        <p className="ajuda">{T.ajustesJogo.ajuda}</p>
        {jogo.needsGameDir ? (
          <div className="warnbox" role="note">
            <AlertTriangle className="ic" aria-hidden="true" />
            <span>{T.ajustesJogo.precisa}</span>
          </div>
        ) : null}
        <div className="pasta-escolhida">
          <span className="mono" title={jogo.gameDir ?? undefined}>
            {jogo.gameDir ?? T.ajustesJogo.nenhuma}
          </span>
          <button type="button" className="btn btn-pequeno" onClick={() => void escolher()}>
            {T.ajustesJogo.escolher}
          </button>
          {jogo.gameDir ? (
            <button type="button" className="btn btn-pequeno" onClick={() => void definirPastaDoJogo(jogo.id, null)}>
              {T.ajustesJogo.limpar}
            </button>
          ) : null}
        </div>
      </section>

      <section className="ajuste">
        <b className="ajuste-titulo">{T.ajustesJogo.destinos}</b>
        {jogo.rememberedTargets.length ? (
          jogo.rememberedTargets.map((d) => (
            <div className="destino-lembrado" key={d.template}>
              <span className="mono" title={d.template}>
                {d.template}
              </span>
              <span aria-hidden="true">→</span>
              <span className="mono" title={d.path}>
                {d.path}
              </span>
              <button type="button" className="lnk" onClick={() => void esquecerDestino(jogo.id, d.template)}>
                {T.ajustesJogo.esquecer}
              </button>
            </div>
          ))
        ) : (
          <p className="ajuda">{T.ajustesJogo.semDestinos}</p>
        )}
      </section>

      <section className="ajuste">
        <button type="button" className="btn btn-pequeno" onClick={() => void ignorar(jogo.id)}>
          <EyeOff className="ic" aria-hidden="true" />
          {T.ajustesJogo.ignorar}
        </button>
      </section>
    </div>
  );
}
