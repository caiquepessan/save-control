import { FolderOpen, OctagonAlert } from "lucide-react";
import { useApp } from "../store/app";
import { T } from "../texto";

/** O app não conseguiu abrir (cofre de versão mais nova, arquivo danificado…). */
export function Failed({ erro, pastaDeDados }: { erro: string; pastaDeDados: string | null }) {
  const backend = useApp((s) => s.backend);
  return (
    <main className="tela-centro">
      <div className="caixa-centro">
        <span className="icone-grande perigo">
          <OctagonAlert className="ic" aria-hidden="true" />
        </span>
        <h1>{T.falhou.titulo}</h1>
        <p className="erro-detalhe">{erro}</p>
        {pastaDeDados ? (
          <div className="tela-botoes">
            <button type="button" className="btn pri" onClick={() => void backend.abrirPasta(pastaDeDados)}>
              <FolderOpen className="ic" aria-hidden="true" />
              {T.falhou.abrirDados}
            </button>
          </div>
        ) : null}
      </div>
    </main>
  );
}
