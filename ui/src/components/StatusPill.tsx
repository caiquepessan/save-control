import { Check } from "lucide-react";
import type { GameView } from "../api/gerado/GameView";
import { T } from "../texto";

/** O selo de status na capa: Protegido, Save novo, Sem backup, Sem save ou Só no cofre. */
export function StatusPill({ jogo }: { jogo: GameView }) {
  if (jogo.vaultOnly) {
    return (
      <span className="pill cofre">
        <i />
        {T.status.soNoCofre}
      </span>
    );
  }
  if (jogo.status === "protegido") {
    return (
      <span className="pill ok">
        <Check className="ic" aria-hidden="true" />
        {T.status.protegido}
      </span>
    );
  }
  const classe = { saveNovo: "novo", semBackup: "aviso", semSave: "nenhum" }[jogo.status];
  return (
    <span className={`pill ${classe}`}>
      <i />
      {T.status[jogo.status]}
    </span>
  );
}
