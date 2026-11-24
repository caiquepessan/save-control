import { MoreHorizontal } from "lucide-react";
import { useEffect, useRef, useState, type KeyboardEvent as ReactKeyboardEvent } from "react";

export interface ItemMenu {
  rotulo: string;
  acao: () => void;
  perigo?: boolean;
}

/**
 * Botão "⋯" com menu: abre com clique ou Enter, as setas andam pelos itens,
 * Esc fecha e devolve o foco ao botão, clicar fora fecha.
 */
export function Menu({ rotulo, itens }: { rotulo: string; itens: ItemMenu[] }) {
  const [aberto, setAberto] = useState(false);
  const ancora = useRef<HTMLDivElement>(null);
  const botao = useRef<HTMLButtonElement>(null);
  const refs = useRef<(HTMLButtonElement | null)[]>([]);

  useEffect(() => {
    if (!aberto) return;
    refs.current[0]?.focus({ preventScroll: true });
    const aoClicar = (e: MouseEvent) => {
      if (ancora.current && !ancora.current.contains(e.target as Node)) setAberto(false);
    };
    document.addEventListener("mousedown", aoClicar);
    return () => document.removeEventListener("mousedown", aoClicar);
  }, [aberto]);

  const fechar = (devolverFoco: boolean) => {
    setAberto(false);
    if (devolverFoco) botao.current?.focus({ preventScroll: true });
  };

  const aoTeclar = (e: ReactKeyboardEvent) => {
    const atual = refs.current.findIndex((r) => r === document.activeElement);
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      fechar(true);
    } else if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      const passo = e.key === "ArrowDown" ? 1 : -1;
      refs.current[(atual + passo + itens.length) % itens.length]?.focus();
    } else if (e.key === "Tab") {
      fechar(false);
    }
  };

  return (
    <div className="menu-ancora" ref={ancora}>
      <button
        type="button"
        ref={botao}
        className="ibtn"
        aria-label={rotulo}
        title={rotulo}
        aria-haspopup="menu"
        aria-expanded={aberto}
        onClick={() => setAberto((v) => !v)}
      >
        <MoreHorizontal className="ic" aria-hidden="true" />
      </button>
      {aberto ? (
        <div className="menu" role="menu" aria-label={rotulo} onKeyDown={aoTeclar}>
          {itens.map((it, i) => (
            <button
              key={it.rotulo}
              ref={(el) => {
                refs.current[i] = el;
              }}
              type="button"
              role="menuitem"
              tabIndex={-1}
              className={it.perigo ? "menu-item perigo" : "menu-item"}
              onClick={() => {
                fechar(false);
                it.acao();
              }}
            >
              {it.rotulo}
            </button>
          ))}
        </div>
      ) : null}
    </div>
  );
}
