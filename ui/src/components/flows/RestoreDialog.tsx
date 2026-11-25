import { AlertTriangle, FolderSearch, RotateCcw, RotateCw } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { ErroApp } from "../../api/backend";
import type { RestorePreviewView } from "../../api/gerado/RestorePreviewView";
import { textosDaRestauracao } from "../../lib/restore";
import { useApp, type PedidoRestauracao } from "../../store/app";
import { T } from "../../texto";
import { Modal } from "../Modal";

type Fase =
  | { tipo: "carregando" }
  | { tipo: "confirmar"; previa: RestorePreviewView; restaurando: boolean }
  | { tipo: "destino"; alias: string; template: string; candidatos: string[] };

function Fluxo({ pedido }: { pedido: PedidoRestauracao }) {
  const backend = useApp((s) => s.backend);
  const jogo = useApp((s) => s.biblioteca?.games.find((g) => g.id === pedido.id));
  const versoes = useApp((s) => s.historicos[pedido.id]);
  const cancelar = useApp((s) => s.cancelarRestauracao);
  const avisar = useApp((s) => s.avisar);
  const marcarPendente = useApp((s) => s.marcarPendente);
  const aposRestaurar = useApp((s) => s.aposRestaurar);
  const [fase, setFase] = useState<Fase>({ tipo: "carregando" });
  const [destinos, setDestinos] = useState<Record<string, string>>({});
  // Só lembra o destino quando foi o app que perguntou onde restaurar.
  const [lembrar, setLembrar] = useState(false);

  const pedirPrevia = useCallback(
    async (d: Record<string, string>) => {
      setFase({ tipo: "carregando" });
      try {
        const previa = await backend.previaRestauracao(pedido.id, pedido.arquivo, d);
        setFase({ tipo: "confirmar", previa, restaurando: false });
      } catch (e) {
        const erro = ErroApp.de(e);
        if (erro.kind === "precisaDestino") {
          setFase({ tipo: "destino", alias: erro.alias ?? "r0", template: erro.root ?? "", candidatos: erro.candidates });
        } else {
          cancelar();
          avisar("erro", erro.message);
        }
      }
    },
    [backend, pedido.id, pedido.arquivo, cancelar, avisar],
  );

  useEffect(() => {
    void pedirPrevia({});
  }, [pedirPrevia]);

  const usarDestino = (alias: string, caminho: string, lembrarEscolha: boolean) => {
    const d = { ...destinos, [alias]: caminho };
    setDestinos(d);
    setLembrar(lembrarEscolha);
    void pedirPrevia(d);
  };

  const escolherPasta = async (alias: string, lembrarEscolha: boolean) => {
    const pasta = await backend.escolherPasta(T.restaurar.escolherPasta);
    if (pasta) usarDestino(alias, pasta, lembrarEscolha);
  };

  const confirmar = async (previa: RestorePreviewView) => {
    setFase({ tipo: "confirmar", previa, restaurando: true });
    marcarPendente(pedido.id, "restauracao");
    try {
      const r = await backend.restaurar(pedido.id, pedido.arquivo, {
        targets: destinos,
        onlyFiles: pedido.soArquivos ?? null,
        remember: lembrar,
      });
      cancelar();
      await aposRestaurar(pedido.id, r);
    } catch (e) {
      const erro = ErroApp.de(e);
      if (erro.kind === "jogoAberto") {
        setFase({ tipo: "confirmar", previa: { ...previa, running: true }, restaurando: false });
      } else {
        cancelar();
        avisar("erro", erro.message);
      }
    } finally {
      marcarPendente(pedido.id, null);
    }
  };

  if (!jogo) return null;

  if (fase.tipo === "carregando") {
    return (
      <Modal rotulo={T.restaurar.carregando} onFechar={cancelar}>
        <p className="modal-carregando">{T.restaurar.carregando}</p>
      </Modal>
    );
  }

  if (fase.tipo === "destino") {
    return (
      <Modal rotulo={T.restaurar.ondeTitulo} onFechar={cancelar}>
        <div className="mt">
          <span className="mico">
            <FolderSearch className="ic" aria-hidden="true" />
          </span>
          <div>
            <b>{T.restaurar.ondeTitulo}</b>
            <em>{T.restaurar.ondeTexto(fase.template)}</em>
          </div>
        </div>
        <div className="opcoes-destino">
          {fase.candidatos.map((c) => (
            <button key={c} type="button" className="opcao" onClick={() => usarDestino(fase.alias, c, true)}>
              {c}
            </button>
          ))}
          <button type="button" className="opcao outra" onClick={() => void escolherPasta(fase.alias, true)}>
            {T.restaurar.escolherOutra}
          </button>
        </div>
        <div className="mbtns">
          <button type="button" className="btn" data-autofocus onClick={cancelar}>
            {T.restaurar.cancelar}
          </button>
        </div>
      </Modal>
    );
  }

  const { previa, restaurando } = fase;
  const maisAntiga = !!versoes?.length && versoes[versoes.length - 1].fileName === previa.snapshot.fileName;
  // Pasta escolhida por "Restaurar em outra pasta…" (a de "Onde restaurar?" fica lembrada e é a do jogo).
  const outraPasta = Object.keys(destinos).length > 0 && !lembrar;
  const t = textosDaRestauracao(previa, jogo, { maisAntiga, soArquivos: pedido.soArquivos?.length, outraPasta });
  return (
    <Modal rotulo={t.titulo} onFechar={cancelar}>
      <div className="mt">
        <span className="mico">
          <RotateCcw className="ic" aria-hidden="true" />
        </span>
        <div>
          <b>{t.titulo}</b>
          <em>{t.corpo}</em>
        </div>
      </div>
      <div className="mbox">
        <div className="mrow">
          <span>{T.restaurar.versao}</span>
          <b>{t.versao}</b>
        </div>
        <div className="mrow">
          <span>{T.restaurar.arquivos}</span>
          <b>{t.arquivos}</b>
        </div>
        {t.destinos.map((d) => (
          <div className="mrow" key={d.caminho}>
            <span>{T.restaurar.destino}</span>
            <b title={d.caminho}>{d.curto}</b>
          </div>
        ))}
      </div>
      {t.aviso ? (
        <div className="mwarn">
          <AlertTriangle className="ic" aria-hidden="true" />
          {t.aviso}
        </div>
      ) : null}
      <div className="mlinks">
        <button
          type="button"
          className="lnk-sec"
          disabled={!previa.singleRoot || restaurando}
          title={previa.singleRoot ? undefined : T.restaurar.variasPastas}
          onClick={() => void escolherPasta(previa.targets[0]?.alias ?? "r0", false)}
        >
          {T.restaurar.outraPasta}
        </button>
      </div>
      <div className="mbtns">
        <button type="button" className="btn" data-autofocus onClick={cancelar} disabled={restaurando}>
          {T.restaurar.cancelar}
        </button>
        {previa.running ? (
          <button type="button" className="btn pri" onClick={() => void pedirPrevia(destinos)}>
            <RotateCw className="ic" aria-hidden="true" />
            {T.restaurar.tentarDeNovo}
          </button>
        ) : (
          <button type="button" className="btn pri" disabled={restaurando} onClick={() => void confirmar(previa)}>
            <RotateCcw className="ic" aria-hidden="true" />
            {T.restaurar.confirmar}
          </button>
        )}
      </div>
    </Modal>
  );
}

/** A confirmação de restaurar (mockup), com a escolha de destino quando o app não sabe onde. */
export function RestoreDialog() {
  const pedido = useApp((s) => s.restauracao);
  if (!pedido) return null;
  return <Fluxo key={`${pedido.id}|${pedido.arquivo}`} pedido={pedido} />;
}
