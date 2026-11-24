import { X } from "lucide-react";
import { useEffect } from "react";
import { useApp, type Toast } from "../store/app";
import { T } from "../texto";

function ToastItem({ toast, fechar }: { toast: Toast; fechar: (id: number) => void }) {
  useEffect(() => {
    if (toast.fixo) return;
    const h = setTimeout(() => fechar(toast.id), 4000);
    return () => clearTimeout(h);
  }, [toast, fechar]);
  return (
    <div className={`toast ${toast.tipo}`} role={toast.tipo === "erro" ? "alert" : "status"}>
      <span>{toast.texto}</span>
      <button type="button" aria-label={T.toasts.fechar} onClick={() => fechar(toast.id)}>
        <X className="ic" aria-hidden="true" />
      </button>
    </div>
  );
}

/** Avisos rápidos no canto: somem em 4 s; os de erro ficam até fechar. */
export function Toasts() {
  const toasts = useApp((s) => s.toasts);
  const fechar = useApp((s) => s.fecharToast);
  return (
    <div className="toasts">
      {toasts.map((t) => (
        <ToastItem key={t.id} toast={t} fechar={fechar} />
      ))}
    </div>
  );
}
