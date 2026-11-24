import { HardDrive } from "lucide-react";
import { caminhoCurto, tamanho } from "../lib/format";
import { useApp } from "../store/app";
import { T } from "../texto";

/** O cartão do cofre no pé do menu lateral: pasta, uso do disco e versões guardadas. */
export function VaultCard() {
  const cofre = useApp((s) => s.biblioteca?.vault ?? null);
  if (!cofre) return <div className="cofre vazio" data-testid="cartao-cofre" />;
  const usado =
    cofre.diskTotal && cofre.diskFree != null && cofre.diskTotal > 0 ? (cofre.diskTotal - cofre.diskFree) / cofre.diskTotal : 0;
  return (
    <div className={cofre.available ? "cofre" : "cofre indisponivel"} data-testid="cartao-cofre">
      <div className="cofre-topo">
        <span className="cofre-icone">
          <HardDrive className="ic" aria-hidden="true" />
        </span>
        <div className="cofre-textos">
          <div className="cofre-titulo">{T.cofre.titulo}</div>
          <div className="cofre-caminho" title={cofre.path}>
            {caminhoCurto(cofre.path)}
          </div>
        </div>
      </div>
      <div className="barra-uso" aria-hidden="true">
        <i style={{ width: `${Math.round(usado * 100)}%` }} />
      </div>
      <div className="cofre-linha">
        {cofre.available ? T.cofre.versoes(tamanho(cofre.zipBytes), cofre.versions) : T.cofre.indisponivel}
      </div>
    </div>
  );
}
