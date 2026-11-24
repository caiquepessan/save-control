import { useState } from "react";
import { useApp } from "../../store/app";
import { T } from "../../texto";
import { Modal } from "../Modal";

/** Renomear (só neste PC), com volta ao nome original. */
export function RenameDialog({ id }: { id: string }) {
  const jogo = useApp((s) => s.biblioteca?.games.find((g) => g.id === id));
  const fechar = useApp((s) => s.fecharDialogo);
  const renomear = useApp((s) => s.renomear);
  const [nome, setNome] = useState(jogo?.title ?? "");
  if (!jogo) return null;
  return (
    <Modal rotulo={T.renomear.titulo} onFechar={fechar}>
      <form
        onSubmit={(e) => {
          e.preventDefault();
          fechar();
          void renomear(id, nome.trim() || null);
        }}
      >
        <div className="mt">
          <div>
            <b>{T.renomear.titulo}</b>
          </div>
        </div>
        <label className="campo">
          <span>{T.renomear.rotulo}</span>
          <input data-autofocus value={nome} maxLength={120} onChange={(e) => setNome(e.target.value)} />
        </label>
        {jogo.title !== jogo.originalTitle ? (
          <div className="mlinks">
            <button
              type="button"
              className="lnk-sec"
              onClick={() => {
                fechar();
                void renomear(id, null);
              }}
            >
              {T.renomear.original}
            </button>
          </div>
        ) : null}
        <div className="mbtns">
          <button type="button" className="btn" onClick={fechar}>
            {T.renomear.cancelar}
          </button>
          <button type="submit" className="btn pri">
            {T.renomear.salvar}
          </button>
        </div>
      </form>
    </Modal>
  );
}

/** Confirmação de remover um jogo adicionado à mão (os backups ficam). */
export function RemoveDialog({ id }: { id: string }) {
  const jogo = useApp((s) => s.biblioteca?.games.find((g) => g.id === id));
  const fechar = useApp((s) => s.fecharDialogo);
  const remover = useApp((s) => s.remover);
  if (!jogo) return null;
  const titulo = T.remover.titulo(jogo.title);
  return (
    <Modal rotulo={titulo} onFechar={fechar}>
      <div className="mt">
        <div>
          <b>{titulo}</b>
          <em>{T.remover.texto}</em>
        </div>
      </div>
      <div className="mbtns">
        <button type="button" className="btn" data-autofocus onClick={fechar}>
          {T.remover.cancelar}
        </button>
        <button
          type="button"
          className="btn perigo"
          onClick={() => {
            fechar();
            void remover(id);
          }}
        >
          {T.remover.confirmar}
        </button>
      </div>
    </Modal>
  );
}
