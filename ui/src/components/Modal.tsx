import { useEffect, useRef, type KeyboardEvent as ReactKeyboardEvent, type ReactNode } from "react";

const FOCAVEIS = 'button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea, [tabindex]:not([tabindex="-1"])';

/**
 * Diálogo por cima de tudo (visual do mockup "Confirmação de restaurar").
 * Esc e clique fora = cancelar; o foco fica preso dentro e volta para onde
 * estava ao fechar. Um elemento com `data-autofocus` recebe o foco ao abrir.
 */
export function Modal({ rotulo, onFechar, children, className = "" }: { rotulo: string; onFechar: () => void; children: ReactNode; className?: string }) {
  const caixa = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const anterior = document.activeElement as HTMLElement | null;
    const alvo = caixa.current?.querySelector<HTMLElement>("[data-autofocus]") ?? caixa.current?.querySelector<HTMLElement>(FOCAVEIS);
    // preventScroll: durante a animação de entrada, o foco não pode rolar a página.
    (alvo ?? caixa.current)?.focus({ preventScroll: true });
    return () => anterior?.focus?.({ preventScroll: true });
  }, []);

  const aoTeclar = (e: ReactKeyboardEvent) => {
    if (e.key === "Escape") {
      e.stopPropagation();
      onFechar();
      return;
    }
    if (e.key !== "Tab" || !caixa.current) return;
    const itens = Array.from(caixa.current.querySelectorAll<HTMLElement>(FOCAVEIS));
    if (!itens.length) return;
    const [primeiro, ultimo] = [itens[0], itens[itens.length - 1]];
    if (e.shiftKey && document.activeElement === primeiro) {
      e.preventDefault();
      ultimo.focus();
    } else if (!e.shiftKey && document.activeElement === ultimo) {
      e.preventDefault();
      primeiro.focus();
    }
  };

  return (
    <div
      className="modal-fundo"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onFechar();
      }}
    >
      <div className={`modal ${className}`} role="dialog" aria-modal="true" aria-label={rotulo} ref={caixa} tabIndex={-1} onKeyDown={aoTeclar}>
        {children}
      </div>
    </div>
  );
}
