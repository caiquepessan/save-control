import { useApp } from "../store/app";
import { T } from "../texto";
import { Logo } from "./Logo";

/** Barra de título própria (a janela não tem moldura): arrastável, com os três botões. */
export function TitleBar() {
  const janela = useApp((s) => s.backend.janela);
  return (
    <header className="barra-titulo" data-testid="barra-titulo" data-tauri-drag-region>
      <Logo className="barra-logo" />
      <span className="barra-nome" data-tauri-drag-region>
        {T.app.nome}
      </span>
      <div className="barra-controles">
        <button type="button" aria-label={T.janela.minimizar} title={T.janela.minimizar} onClick={() => void janela.minimizar()}>
          <svg viewBox="0 0 10 10" aria-hidden="true">
            <path d="M1 5h8" />
          </svg>
        </button>
        <button type="button" aria-label={T.janela.maximizar} title={T.janela.maximizar} onClick={() => void janela.alternarMaximizar()}>
          <svg viewBox="0 0 10 10" aria-hidden="true">
            <rect x="1.5" y="1.5" width="7" height="7" rx="1" />
          </svg>
        </button>
        <button type="button" className="fechar" aria-label={T.janela.fechar} title={T.janela.fechar} onClick={() => void janela.fechar()}>
          <svg viewBox="0 0 10 10" aria-hidden="true">
            <path d="M1.5 1.5l7 7M8.5 1.5l-7 7" />
          </svg>
        </button>
      </div>
    </header>
  );
}
