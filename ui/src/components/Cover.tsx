import { useState, type CSSProperties } from "react";
import type { GameView } from "../api/gerado/GameView";
import { capaGerada } from "../lib/cover";
import { useApp } from "../store/app";

type Variante = "cartao" | "painel" | "mini";

/**
 * Capa do jogo: a imagem (Steam ou escolhida) quando houver; senão, a capa
 * gerada (gradiente + emblema + título). Se a imagem falhar, volta para a gerada.
 */
export function Cover({ jogo, variante = "cartao", className = "", children }: { jogo: GameView; variante?: Variante; className?: string; children?: React.ReactNode }) {
  const srcDaImagem = useApp((s) => s.backend.srcDaImagem);
  const [falhou, setFalhou] = useState<string | null>(null);
  const g = capaGerada(jogo.id, jogo.title);
  const estilo = { "--c0": g.paleta.c0, "--c1": g.paleta.c1, "--c2": g.paleta.c2, "--c3": g.paleta.c3 } as CSSProperties;
  const imagem = jogo.cover.kind !== "gerada" && jogo.cover.path && falhou !== jogo.cover.path ? jogo.cover.path : null;
  return (
    <div className={`capa capa-${variante}${className ? ` ${className}` : ""}`} style={estilo}>
      <svg className="emb" viewBox="0 0 100 100" aria-hidden="true">
        <path d={g.emblema} />
      </svg>
      {variante === "cartao" && g.kicker ? <span className="cvk">{g.kicker}</span> : null}
      {variante === "cartao" ? <span className="cvt">{g.titulo}</span> : null}
      {imagem ? (
        <img className="capa-img" src={srcDaImagem(imagem)} alt={`Capa de ${jogo.title}`} onError={() => setFalhou(imagem)} />
      ) : null}
      {children}
    </div>
  );
}
