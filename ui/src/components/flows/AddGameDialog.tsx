import { FolderPlus, ImagePlus, Plus, X } from "lucide-react";
import { useState, type FormEvent } from "react";
import { useApp } from "../../store/app";
import { T } from "../../texto";
import { Modal } from "../Modal";

/** "Adicionar jogo": nome, pastas do save, pasta do jogo, processo e capa. */
export function AddGameDialog() {
  const backend = useApp((s) => s.backend);
  const fechar = useApp((s) => s.fecharDialogo);
  const adicionarJogo = useApp((s) => s.adicionarJogo);
  const [nome, setNome] = useState("");
  const [pastas, setPastas] = useState<string[]>([]);
  const [pastaJogo, setPastaJogo] = useState<string | null>(null);
  const [processo, setProcesso] = useState("");
  const [capa, setCapa] = useState<string | null>(null);
  const [erro, setErro] = useState<string | null>(null);
  const [enviando, setEnviando] = useState(false);

  const escolherSave = async () => {
    const p = await backend.escolherPasta(T.adicionar.tituloEscolherSave);
    if (p) setPastas((atual) => (atual.includes(p) ? atual : [...atual, p]));
  };
  const escolherJogo = async () => {
    const p = await backend.escolherPasta(T.adicionar.escolherJogo);
    if (p) setPastaJogo(p);
  };
  const escolherCapa = async () => {
    const p = await backend.escolherImagem();
    if (p) setCapa(p);
  };

  const enviar = async (e: FormEvent) => {
    e.preventDefault();
    if (enviando) return;
    if (!nome.trim()) return setErro(T.adicionar.erroNome);
    if (!pastas.length) return setErro(T.adicionar.erroPasta);
    setErro(null);
    setEnviando(true);
    const falha = await adicionarJogo({
      name: nome.trim(),
      saveDirs: pastas,
      gameDir: pastaJogo,
      processes: processo
        .split(/[,;]/)
        .map((p) => p.trim())
        .filter(Boolean),
      coverFile: capa,
    });
    setEnviando(false);
    if (falha) setErro(falha);
  };

  return (
    <Modal rotulo={T.adicionar.titulo} onFechar={fechar} className="modal-form">
      <form onSubmit={(e) => void enviar(e)}>
        <div className="mt">
          <span className="mico">
            <Plus className="ic" aria-hidden="true" />
          </span>
          <div>
            <b>{T.adicionar.titulo}</b>
          </div>
        </div>

        <label className="campo">
          <span>{T.adicionar.nome}</span>
          <input data-autofocus value={nome} maxLength={120} onChange={(e) => setNome(e.target.value)} />
        </label>

        <div className="campo">
          <span>{T.adicionar.pastaSave}</span>
          {pastas.map((p) => (
            <div className="pasta-escolhida" key={p}>
              <span className="mono" title={p}>
                {p}
              </span>
              <button type="button" className="tirar" aria-label={T.adicionar.tirarPasta(p)} onClick={() => setPastas(pastas.filter((x) => x !== p))}>
                <X className="ic" aria-hidden="true" />
              </button>
            </div>
          ))}
          <button type="button" className="btn btn-pequeno" onClick={() => void escolherSave()}>
            <FolderPlus className="ic" aria-hidden="true" />
            {pastas.length ? T.adicionar.outraPasta : T.adicionar.escolherPasta}
          </button>
        </div>

        <div className="campo">
          <span>{T.adicionar.pastaJogo}</span>
          <div className="pasta-escolhida">
            <span className="mono" title={pastaJogo ?? undefined}>
              {pastaJogo ?? T.adicionar.nenhuma}
            </span>
            <button type="button" className="btn btn-pequeno" aria-label={T.adicionar.escolherJogo} onClick={() => void escolherJogo()}>
              {T.adicionar.escolher}
            </button>
            {pastaJogo ? (
              <button type="button" className="btn btn-pequeno" onClick={() => setPastaJogo(null)}>
                {T.adicionar.limpar}
              </button>
            ) : null}
          </div>
          <small className="ajuda">{T.adicionar.ajudaPastaJogo}</small>
        </div>

        <div className="campo">
          <label className="campo-rotulo">
            <span>{T.adicionar.processo}</span>
            <input
              value={processo}
              placeholder={T.adicionar.exemploProcesso}
              aria-describedby="ajuda-processo"
              onChange={(e) => setProcesso(e.target.value)}
            />
          </label>
          <small className="ajuda" id="ajuda-processo">
            {T.adicionar.ajudaProcesso}
          </small>
        </div>

        <div className="campo">
          <span>{T.adicionar.capa}</span>
          <div className="pasta-escolhida">
            <span className="mono" title={capa ?? undefined}>
              {capa ?? T.adicionar.nenhuma}
            </span>
            <button type="button" className="btn btn-pequeno" onClick={() => void escolherCapa()}>
              <ImagePlus className="ic" aria-hidden="true" />
              {T.adicionar.escolherImagem}
            </button>
          </div>
        </div>

        {erro ? (
          <p className="erro-form" role="alert">
            {erro}
          </p>
        ) : null}

        <div className="mbtns">
          <button type="button" className="btn" onClick={fechar}>
            {T.adicionar.cancelar}
          </button>
          <button type="submit" className="btn pri" disabled={enviando}>
            {T.adicionar.confirmar}
          </button>
        </div>
      </form>
    </Modal>
  );
}
