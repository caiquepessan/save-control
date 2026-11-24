import { Download, RotateCcw } from "lucide-react";
import type { GameView } from "../api/gerado/GameView";
import { subtitulo } from "../lib/library";
import { useApp } from "../store/app";
import { T } from "../texto";
import { Cover } from "./Cover";
import { StatusPill } from "./StatusPill";

/** Cartão da grade: capa, selo, nome e "origem · quando"; no hover, Fazer backup e Restaurar…. */
export function GameCard({ jogo, abertos, agora }: { jogo: GameView; abertos: Set<string>; agora: Date }) {
  const abrirPainel = useApp((s) => s.abrirPainel);
  const fazerBackup = useApp((s) => s.fazerBackup);
  const pendente = useApp((s) => s.pendentes[jogo.id]);
  const dim = jogo.status === "semSave" && !jogo.vaultOnly;
  return (
    <article className={dim ? "gc dim" : "gc"} aria-label={jogo.title} data-id={jogo.id}>
      <button type="button" className="gc-abrir" aria-label={T.cartao.abrir(jogo.title)} onClick={() => abrirPainel(jogo.id)} />
      <Cover jogo={jogo} />
      <StatusPill jogo={jogo} />
      <div className="gc-acoes">
        {jogo.fileCount > 0 && !jogo.vaultOnly ? (
          <button type="button" className="b p" disabled={!!pendente} onClick={() => void fazerBackup(jogo.id)}>
            <Download className="ic" aria-hidden="true" />
            {T.cartao.fazerBackup}
          </button>
        ) : null}
        {jogo.backupCount > 0 ? (
          <button type="button" className="b s" onClick={() => abrirPainel(jogo.id, { aba: "historico", modoRestaurar: true })}>
            <RotateCcw className="ic" aria-hidden="true" />
            {T.cartao.restaurar}
          </button>
        ) : null}
      </div>
      <div className="meta">
        <div className="name" title={jogo.title}>
          {jogo.title}
        </div>
        <div className="sub">{subtitulo(jogo, agora, abertos)}</div>
      </div>
    </article>
  );
}
