import { useEffect, useRef, type ReactNode } from "react";

/** Caixinha presa a um botão: fecha com Esc ou clicando fora. */
export function Popover({ rotulo, onFechar, children, className = "" }: { rotulo: string; onFechar: () => void; children: ReactNode; className?: string }) {
  const caixa = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const aoClicar = (e: MouseEvent) => {
      if (caixa.current && !caixa.current.contains(e.target as Node)) onFechar();
    };
    document.addEventListener("mousedown", aoClicar);
    return () => document.removeEventListener("mousedown", aoClicar);
  }, [onFechar]);

  return (
    <div
      className={`popover ${className}`}
      role="dialog"
      aria-label={rotulo}
      ref={caixa}
      onKeyDown={(e) => {
        if (e.key === "Escape") {
          e.stopPropagation();
          onFechar();
        }
      }}
    >
      {children}
    </div>
  );
}
