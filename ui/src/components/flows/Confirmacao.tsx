import { FolderOpen } from "lucide-react";
import type { ReactNode } from "react";
import type { CofreEscolhido } from "../../store/app";
import { T } from "../../texto";
import { Modal } from "../Modal";

/** Pergunta de sim ou não por cima de tudo; o foco começa no "Cancelar". */
export function Confirmacao({
  titulo,
  texto,
  confirmar,
  onConfirmar,
  onCancelar,
  children,
}: {
  titulo: string;
  texto?: string;
  confirmar: string;
  onConfirmar: () => void;
  onCancelar: () => void;
  /** Detalhe mostrado abaixo do texto (fica fora do nome do diálogo). */
  children?: ReactNode;
}) {
  return (
    <Modal rotulo={texto ? `${titulo} ${texto}` : titulo} onFechar={onCancelar}>
      <div className="mt">
        <div>
          <b>{titulo}</b>
          {texto ? <em>{texto}</em> : null}
        </div>
      </div>
      {children}
      <div className="mbtns">
        <button type="button" className="btn" data-autofocus onClick={onCancelar}>
          {T.escolherCofre.cancelar}
        </button>
        <button type="button" className="btn pri" onClick={onConfirmar}>
          {confirmar}
        </button>
      </div>
    </Modal>
  );
}

/** "Usar o cofre desta pasta?" ou "Criar um cofre novo nesta pasta?", com a pasta escolhida. */
export function ConfirmarCofre({
  escolha,
  onConfirmar,
  onCancelar,
}: {
  escolha: CofreEscolhido;
  onConfirmar: () => void;
  onCancelar: () => void;
}) {
  return (
    <Confirmacao
      titulo={escolha.existe ? T.escolherCofre.usar(escolha.versoes) : T.escolherCofre.criar}
      confirmar={escolha.existe ? T.escolherCofre.botaoUsar : T.escolherCofre.botaoCriar}
      onConfirmar={onConfirmar}
      onCancelar={onCancelar}
    >
      <div className="pasta-cofre">
        <FolderOpen className="ic" aria-hidden="true" />
        <span className="mono">{escolha.pasta}</span>
      </div>
    </Confirmacao>
  );
}
