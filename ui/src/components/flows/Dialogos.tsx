import { useApp } from "../../store/app";
import { AddGameDialog } from "./AddGameDialog";
import { RemoveDialog, RenameDialog } from "./RenameDialog";

/** O diálogo aberto agora (adicionar, renomear ou remover), se houver. */
export function Dialogos() {
  const dialogo = useApp((s) => s.dialogo);
  if (!dialogo) return null;
  if (dialogo.tipo === "adicionar") return <AddGameDialog />;
  if (dialogo.tipo === "renomear") return <RenameDialog key={dialogo.id} id={dialogo.id} />;
  return <RemoveDialog key={dialogo.id} id={dialogo.id} />;
}
