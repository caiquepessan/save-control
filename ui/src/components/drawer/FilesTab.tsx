import { File, FolderOpen, RotateCcw } from "lucide-react";
import type { GameView } from "../../api/gerado/GameView";
import { tamanho } from "../../lib/format";
import { useApp } from "../../store/app";
import { T } from "../../texto";

/** Os arquivos do save neste PC, por pasta. */
export function FilesTab({ jogo }: { jogo: GameView }) {
  const abrirPasta = useApp((s) => s.backend.abrirPasta);
  const abrirSelecao = useApp((s) => s.abrirSelecaoArquivos);
  const raizes = jogo.roots.filter((r) => r.files.length);
  const selecionar =
    jogo.backupCount > 0 ? (
      <button type="button" className="btn btn-pequeno" onClick={() => abrirSelecao(jogo.id)}>
        <RotateCcw className="ic" aria-hidden="true" />
        {T.selecao.abrir}
      </button>
    ) : null;
  if (!raizes.length) {
    return (
      <div className="arquivos">
        <p className="dvazio">{T.painel.semArquivos}</p>
        {selecionar}
      </div>
    );
  }
  return (
    <div className="arquivos">
      {selecionar}
      {raizes.map((r) => (
        <section key={r.path} className="raiz">
          <div className="raiz-topo">
            <span className="caminho" title={r.path}>
              {r.path}
            </span>
            <button type="button" className="lnk" onClick={() => void abrirPasta(r.path)}>
              <FolderOpen className="ic" aria-hidden="true" />
              {T.painel.abrirPasta}
            </button>
          </div>
          <ul className="lista-arquivos">
            {r.files.map((f) => (
              <li key={f.relPath}>
                <File className="ic" aria-hidden="true" />
                <span className="arquivo-nome" title={f.relPath}>
                  {f.relPath}
                </span>
                <span className="arquivo-tam">{tamanho(f.size)}</span>
              </li>
            ))}
          </ul>
        </section>
      ))}
    </div>
  );
}
