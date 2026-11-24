import { Download } from "lucide-react";
import { useState } from "react";
import { useApp } from "../../store/app";
import { T } from "../../texto";
import { Popover } from "../Popover";

/** "Fazer backup" do painel: pergunta um nome opcional (Enter confirma). */
export function BackupNamePopover({ id, onFechar }: { id: string; onFechar: () => void }) {
  const fazerBackup = useApp((s) => s.fazerBackup);
  const [nome, setNome] = useState("");
  const enviar = () => {
    onFechar();
    void fazerBackup(id, nome.trim() || null);
  };
  return (
    <Popover rotulo={T.backupNome.titulo} onFechar={onFechar} className="pop-backup">
      <form
        onSubmit={(e) => {
          e.preventDefault();
          enviar();
        }}
      >
        <label className="campo">
          <span>{T.backupNome.rotulo}</span>
          <input autoFocus value={nome} maxLength={80} placeholder={T.backupNome.exemplo} onChange={(e) => setNome(e.target.value)} />
        </label>
        <div className="pop-botoes">
          <button type="button" className="btn" onClick={onFechar}>
            {T.backupNome.cancelar}
          </button>
          <button type="submit" className="btn pri">
            <Download className="ic" aria-hidden="true" />
            {T.backupNome.confirmar}
          </button>
        </div>
      </form>
    </Popover>
  );
}
