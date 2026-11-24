import { LayoutGrid, SlidersHorizontal } from "lucide-react";
import { useApp, type Tela } from "../store/app";
import { T } from "../texto";
import { VaultCard } from "./VaultCard";

function ItemMenu({ tela, rotulo, icone, contagem }: { tela: Tela; rotulo: string; icone: React.ReactNode; contagem?: number }) {
  const atual = useApp((s) => s.tela);
  const irPara = useApp((s) => s.irPara);
  const ativo = atual === tela;
  return (
    <button
      type="button"
      aria-label={rotulo}
      aria-current={ativo ? "page" : undefined}
      className={ativo ? "nav on" : "nav"}
      onClick={() => irPara(tela)}
    >
      {icone}
      {rotulo}
      {contagem ? <span className="cnt">{contagem}</span> : null}
    </button>
  );
}

/** Menu lateral: só o que já funciona (Atividade, Cofre e Detetive chegam nos próximos planos). */
export function Sidebar() {
  const total = useApp((s) => s.biblioteca?.games.length ?? 0);
  return (
    <aside className="lateral">
      <nav aria-label={T.menu.rotulo} className="lateral-nav">
        <ItemMenu tela="biblioteca" rotulo={T.menu.biblioteca} icone={<LayoutGrid className="ic" aria-hidden="true" />} contagem={total} />
        <ItemMenu tela="configuracoes" rotulo={T.menu.configuracoes} icone={<SlidersHorizontal className="ic" aria-hidden="true" />} />
      </nav>
      <VaultCard />
    </aside>
  );
}
