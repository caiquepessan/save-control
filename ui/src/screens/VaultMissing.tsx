import { FolderSearch, HardDrive, RotateCw } from "lucide-react";
import { useState } from "react";
import { ErroApp } from "../api/backend";
import { Confirmacao, ConfirmarCofre } from "../components/flows/Confirmacao";
import { useApp, type CofreEscolhido } from "../store/app";
import { T } from "../texto";

/**
 * O cofre configurado não está lá (HD desconectado, pasta movida). Nada é
 * criado sem a pessoa pedir e confirmar (Review Focus 1).
 */
export function VaultMissing() {
  const backend = useApp((s) => s.backend);
  const caminho = useApp((s) => s.estado?.vaultDir ?? "");
  const aplicarEstado = useApp((s) => s.aplicarEstado);
  const escolherCofre = useApp((s) => s.escolherCofre);
  const trocarCofre = useApp((s) => s.trocarCofre);
  const [ocupado, setOcupado] = useState(false);
  const [mensagem, setMensagem] = useState<string | null>(null);
  const [criar, setCriar] = useState(false);
  const [achado, setAchado] = useState<CofreEscolhido | null>(null);

  async function tentarDeNovo() {
    setOcupado(true);
    setMensagem(null);
    try {
      const novo = await backend.tentarCofreDeNovo();
      if (novo.phase === "cofreSumiu") setMensagem(T.cofreSumido.aindaNao);
      await aplicarEstado(novo);
    } catch (e) {
      setMensagem(ErroApp.de(e).message);
    } finally {
      setOcupado(false);
    }
  }

  async function procurar() {
    setMensagem(null);
    const escolha = await escolherCofre(T.cofreSumido.procurarTitulo);
    if (!escolha) return;
    if (escolha.existe) setAchado(escolha);
    else setMensagem(T.cofreSumido.semCofre);
  }

  async function usar(pasta: string, criarNovo: boolean) {
    setCriar(false);
    setAchado(null);
    setOcupado(true);
    const erro = await trocarCofre(pasta, criarNovo);
    if (erro) setMensagem(erro);
    setOcupado(false);
  }

  return (
    <main className="tela-centro">
      <div className="caixa-centro">
        <span className="icone-grande aviso">
          <HardDrive className="ic" aria-hidden="true" />
        </span>
        <h1>{T.cofreSumido.titulo}</h1>
        <p className="tela-texto">{T.cofreSumido.texto(caminho)}</p>
        {mensagem ? (
          <p className="nota aviso" role="status">
            {mensagem}
          </p>
        ) : null}
        <div className="tela-botoes">
          <button type="button" className="btn pri" disabled={ocupado} onClick={() => void tentarDeNovo()}>
            <RotateCw className={ocupado ? "ic girando" : "ic"} aria-hidden="true" />
            {T.cofreSumido.tentarDeNovo}
          </button>
          <button type="button" className="btn" disabled={ocupado} onClick={() => void procurar()}>
            <FolderSearch className="ic" aria-hidden="true" />
            {T.cofreSumido.procurar}
          </button>
        </div>
        <button type="button" className="lnk-sec" disabled={ocupado} onClick={() => setCriar(true)}>
          {T.cofreSumido.criarAqui}
        </button>
      </div>
      {criar ? (
        <Confirmacao
          titulo={T.cofreSumido.criarTitulo(caminho)}
          texto={T.cofreSumido.criarTexto}
          confirmar={T.escolherCofre.botaoCriar}
          onConfirmar={() => void usar(caminho, true)}
          onCancelar={() => setCriar(false)}
        />
      ) : null}
      {achado ? (
        <ConfirmarCofre escolha={achado} onConfirmar={() => void usar(achado.pasta, false)} onCancelar={() => setAchado(null)} />
      ) : null}
    </main>
  );
}
