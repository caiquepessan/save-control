import { HardDrive } from "lucide-react";
import type { AutoStatusView } from "../api/gerado/AutoStatusView";
import { caminhoCurto, hora, tamanho } from "../lib/format";
import { useApp } from "../store/app";
import { T } from "../texto";

function textoDaSituacao(s: AutoStatusView): string {
  switch (s.state) {
    case "ativo":
      return T.automatico.vigiando(s.watching);
    case "pausado":
      return s.pausedUntil ? T.automatico.pausadoAte(hora(s.pausedUntil)) : T.automatico.pausadoAteReabrir;
    case "desligado":
      return T.automatico.desligado;
    case "cofreFora":
      return T.automatico.cofreFora;
    case "parado":
      return T.automatico.iniciando;
  }
}

/** A linha "Backup automático" do cartão: chave, ponto pulsando e a situação. */
function Automatico({ s }: { s: AutoStatusView }) {
  const definirAutomatico = useApp((x) => x.definirAutomatico);
  const retomar = useApp((x) => x.retomar);
  const ponto = s.state === "ativo" ? "vivo" : s.state === "cofreFora" ? "ponto alerta" : "ponto";
  return (
    <div className="cofre-auto">
      <label className="cofre-auto-linha">
        <span className={ponto} aria-hidden="true" />
        <span>{T.automatico.rotulo}</span>
        <input
          type="checkbox"
          role="switch"
          className="chave pequena"
          checked={s.state !== "desligado"}
          disabled={s.state === "cofreFora"}
          onChange={(e) => void definirAutomatico(e.target.checked)}
        />
      </label>
      <div className={s.state === "cofreFora" ? "cofre-situacao alerta" : "cofre-situacao"}>
        <span>{textoDaSituacao(s)}</span>
        {s.state === "pausado" ? (
          <button type="button" className="cofre-retomar" onClick={() => void retomar()}>
            {T.automatico.retomar}
          </button>
        ) : null}
      </div>
    </div>
  );
}

/** O cartão do cofre no pé do menu lateral: pasta, uso do disco, versões guardadas e o backup automático. */
export function VaultCard() {
  const cofre = useApp((s) => s.biblioteca?.vault ?? null);
  const situacao = useApp((s) => s.situacao);
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
      {situacao ? <Automatico s={situacao} /> : null}
    </div>
  );
}
