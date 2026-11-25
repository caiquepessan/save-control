import { dataEHora } from "../../lib/format";
import { tituloDaVersao } from "../../lib/versao";
import { useApp } from "../../store/app";
import { T } from "../../texto";
import { Modal } from "../Modal";
import { AddGameDialog } from "./AddGameDialog";
import { RemoveDialog, RenameDialog } from "./RenameDialog";

/** Confirmação de apagar um backup do cofre (não dá para desfazer). */
function ApagarBackupDialog({ id, arquivo }: { id: string; arquivo: string }) {
  const versoes = useApp((s) => s.historicos[id]);
  const fechar = useApp((s) => s.fecharDialogo);
  const apagar = useApp((s) => s.apagarBackup);
  const i = versoes ? versoes.findIndex((v) => v.fileName === arquivo) : -1;
  if (!versoes || i < 0) return null;
  const v = versoes[i];
  const nome = `${tituloDaVersao(v, i === versoes.length - 1)} (${dataEHora(v.createdAt)})`;
  return (
    <Modal rotulo={T.apagarBackup.titulo} onFechar={fechar}>
      <div className="mt">
        <div>
          <b>{T.apagarBackup.titulo}</b>
          <em>{T.apagarBackup.texto(nome)}</em>
        </div>
      </div>
      <div className="mbtns">
        <button type="button" className="btn" data-autofocus onClick={fechar}>
          {T.apagarBackup.cancelar}
        </button>
        <button
          type="button"
          className="btn perigo"
          onClick={() => {
            fechar();
            void apagar(id, arquivo);
          }}
        >
          {T.apagarBackup.confirmar}
        </button>
      </div>
    </Modal>
  );
}

/** O diálogo aberto agora (adicionar, renomear, remover ou apagar um backup), se houver. */
export function Dialogos() {
  const dialogo = useApp((s) => s.dialogo);
  if (!dialogo) return null;
  if (dialogo.tipo === "adicionar") return <AddGameDialog />;
  if (dialogo.tipo === "renomear") return <RenameDialog key={dialogo.id} id={dialogo.id} />;
  if (dialogo.tipo === "apagarBackup") return <ApagarBackupDialog key={dialogo.arquivo} id={dialogo.id} arquivo={dialogo.arquivo} />;
  return <RemoveDialog key={dialogo.id} id={dialogo.id} />;
}
