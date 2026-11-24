import { LayoutGrid, SlidersHorizontal } from "lucide-react";
import { useMemo } from "react";
import type { GameView } from "../api/gerado/GameView";
import { FONTES, contagens, type Fonte } from "../lib/library";
import { useApp, type Tela } from "../store/app";
import { T } from "../texto";
import { VaultCard } from "./VaultCard";

const NENHUM: GameView[] = [];

/** Cor do quadradinho de cada fonte (mockup A). */
const COR_FONTE: Record<Fonte, string> = {
  steam: "#5aa9ff",
  epic: "#c9ced8",
  gog: "#d18cff",
  pcOutros: "#8b95a8",
  recomp: "#3ee6a8",
  emulador: "#b18cff",
  manual: "#ffb547",
  cofre: "#6b7489",
};

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

function Fontes() {
  const jogos = useApp((s) => s.biblioteca?.games ?? NENHUM);
  const fonteAtual = useApp((s) => s.filtro.fonte);
  const definirFiltro = useApp((s) => s.definirFiltro);
  const irPara = useApp((s) => s.irPara);
  const c = useMemo(() => contagens(jogos).fontes, [jogos]);
  const visiveis = FONTES.filter((f) => c[f] > 0);
  if (!visiveis.length) return null;
  return (
    <>
      <div className="secao">{T.menu.fontes}</div>
      {visiveis.map((f) => {
        const ativo = fonteAtual === f;
        return (
          <button
            key={f}
            type="button"
            className={ativo ? "fonte on" : "fonte"}
            aria-pressed={ativo}
            onClick={() => {
              definirFiltro({ fonte: ativo ? null : f, chip: "todos" });
              irPara("biblioteca");
            }}
          >
            <i style={{ background: COR_FONTE[f] }} aria-hidden="true" />
            {T.fontes[f]}
            <span className="cnt">{c[f]}</span>
          </button>
        );
      })}
    </>
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
      <Fontes />
      <VaultCard />
    </aside>
  );
}
