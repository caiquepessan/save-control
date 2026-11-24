import { useEffect, useState } from "react";
import { ErroApp } from "../../api/backend";
import type { SnapshotFileView } from "../../api/gerado/SnapshotFileView";
import { dataEHora, tamanho } from "../../lib/format";
import { tituloDaVersao } from "../../lib/versao";
import { useApp } from "../../store/app";
import { T } from "../../texto";
import { Modal } from "../Modal";

const chave = (f: { root: string; relPath: string }) => `${f.root}\u0000${f.relPath}`;

function Selecao({ id }: { id: string }) {
  const backend = useApp((s) => s.backend);
  const versoes = useApp((s) => s.historicos[id]);
  const fechar = useApp((s) => s.fecharSelecaoArquivos);
  const iniciarRestauracao = useApp((s) => s.iniciarRestauracao);
  const avisar = useApp((s) => s.avisar);
  const [arquivo, setArquivo] = useState(versoes?.[0]?.fileName ?? "");
  const [lista, setLista] = useState<SnapshotFileView[] | null>(null);
  const [marcados, setMarcados] = useState<Set<string>>(new Set());

  useEffect(() => {
    if (!arquivo) return;
    let vivo = true;
    setLista(null);
    backend
      .arquivosDoBackup(id, arquivo)
      .then((l) => {
        if (!vivo) return;
        setLista(l);
        setMarcados(new Set(l.map(chave)));
      })
      .catch((e) => avisar("erro", ErroApp.de(e).message));
    return () => {
      vivo = false;
    };
  }, [backend, id, arquivo, avisar]);

  const alternar = (k: string) => {
    const novo = new Set(marcados);
    if (novo.has(k)) novo.delete(k);
    else novo.add(k);
    setMarcados(novo);
  };

  const continuar = () => {
    const escolhidos = (lista ?? []).filter((f) => marcados.has(chave(f))).map((f) => ({ root: f.root, relPath: f.relPath }));
    fechar();
    iniciarRestauracao(id, arquivo, escolhidos);
  };

  return (
    <Modal rotulo={T.selecao.titulo} onFechar={fechar} className="modal-selecao">
      <div className="mt">
        <div>
          <b>{T.selecao.titulo}</b>
        </div>
      </div>
      <label className="campo">
        <span>{T.selecao.versao}</span>
        <select value={arquivo} onChange={(e) => setArquivo(e.target.value)}>
          {(versoes ?? []).map((v, i) => (
            <option key={v.fileName} value={v.fileName}>
              {`${tituloDaVersao(v, i === (versoes?.length ?? 0) - 1)} · ${dataEHora(v.createdAt)}`}
            </option>
          ))}
        </select>
      </label>
      <div className="lista-selecao">
        {lista === null ? <p className="modal-carregando">{T.restaurar.carregando}</p> : null}
        {lista?.length === 0 ? <p className="modal-carregando">{T.selecao.nenhum}</p> : null}
        {lista?.map((f) => {
          const k = chave(f);
          return (
            <div className="item-selecao" key={k}>
              <label>
                <input type="checkbox" checked={marcados.has(k)} onChange={() => alternar(k)} />
                {f.relPath}
              </label>
              <span className="arquivo-tam">{tamanho(f.size)}</span>
            </div>
          );
        })}
      </div>
      <div className="mbtns">
        <span className="cresce" />
        <button type="button" className="btn" data-autofocus onClick={fechar}>
          {T.selecao.cancelar}
        </button>
        <button type="button" className="btn pri" disabled={!lista || marcados.size === 0} onClick={continuar}>
          {T.selecao.continuar}
        </button>
      </div>
    </Modal>
  );
}

/** Escolher quais arquivos de uma versão restaurar (aba Arquivos). */
export function SelectFilesDialog() {
  const sel = useApp((s) => s.selecaoArquivos);
  if (!sel) return null;
  return <Selecao key={sel.id} id={sel.id} />;
}
