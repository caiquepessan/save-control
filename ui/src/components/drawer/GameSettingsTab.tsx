import type { GameView } from "../../api/gerado/GameView";
import { T } from "../../texto";

/** Ajustes deste jogo neste PC (os controles entram na Task 11). */
export function GameSettingsTab({ jogo }: { jogo: GameView }) {
  return (
    <div className="ajustes-jogo">
      <div className="kv">
        <span>{T.painel.pastaDoJogo}</span>
        <b className="mono" title={jogo.gameDir ?? undefined}>
          {jogo.gameDir ?? T.painel.nenhuma}
        </b>
      </div>
    </div>
  );
}
