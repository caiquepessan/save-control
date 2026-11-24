import { AlertTriangle, Download, HardDrive, RotateCw, ShieldAlert } from "lucide-react";
import { useMemo } from "react";
import type { GameView } from "../api/gerado/GameView";
import { alertaPrincipal } from "../lib/library";
import { useApp } from "../store/app";
import { T } from "../texto";

const NENHUM: GameView[] = [];

/** O aviso do topo da biblioteca, quando algo pede ação. */
export function AlertBanner() {
  const jogos = useApp((s) => s.biblioteca?.games ?? NENHUM);
  const cofre = useApp((s) => s.biblioteca?.vault ?? null);
  const backupGeral = useApp((s) => s.backupGeral);
  const escaneando = useApp((s) => s.escaneando);
  const backupDeVarios = useApp((s) => s.backupDeVarios);
  const escanear = useApp((s) => s.escanear);
  const definirFiltro = useApp((s) => s.definirFiltro);
  const alerta = useMemo(() => alertaPrincipal(jogos, cofre), [jogos, cofre]);
  if (!alerta) return null;

  let icone, titulo, texto, botao;
  if (alerta.tipo === "cofre") {
    icone = <HardDrive className="ic" aria-hidden="true" />;
    titulo = T.alerta.cofreTitulo;
    texto = T.alerta.cofreTexto(alerta.caminho);
    botao = (
      <button type="button" className="btn" disabled={escaneando} onClick={() => void escanear()}>
        <RotateCw className="ic" aria-hidden="true" />
        {T.alerta.tentarDeNovo}
      </button>
    );
  } else if (alerta.tipo === "semBackup") {
    const [primeiro] = alerta.jogos;
    icone = <AlertTriangle className="ic" aria-hidden="true" />;
    if (alerta.jogos.length === 1) {
      titulo = T.alerta.umSemBackup(primeiro.title);
      const portatil = primeiro.warnings.some((w) => w.kind === "dentroDaPastaDoJogo");
      texto = primeiro.roots[0] ? T.alerta.ondeFoiAchado(primeiro.roots[0].path, portatil) : null;
    } else {
      titulo = T.alerta.variosSemBackup(alerta.jogos.length);
      texto = T.alerta.lista(alerta.jogos.map((j) => j.title));
    }
    botao = (
      <button type="button" className="btn" disabled={backupGeral} onClick={() => void backupDeVarios(alerta.jogos.map((j) => j.id))}>
        <Download className="ic" aria-hidden="true" />
        {T.alerta.fazerBackupAgora}
      </button>
    );
  } else {
    const [primeiro] = alerta.jogos;
    const pasta = primeiro.warnings.find((w) => w.kind === "pastaArriscada")?.detail ?? "";
    icone = <ShieldAlert className="ic" aria-hidden="true" />;
    titulo = T.alerta.arriscadaTitulo(alerta.jogos.length);
    texto = `${primeiro.title}: ${pasta}`;
    botao = (
      <button type="button" className="btn" onClick={() => definirFiltro({ chip: "atencao", fonte: null })}>
        {T.alerta.verJogos}
      </button>
    );
  }

  return (
    <div className="alerta" role="status" aria-label={T.alerta.rotulo}>
      <span className="alerta-icone">{icone}</span>
      <div className="alerta-textos">
        <div className="t">{titulo}</div>
        {texto ? <div className="d">{texto}</div> : null}
      </div>
      {botao}
    </div>
  );
}
