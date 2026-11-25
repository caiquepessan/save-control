import { useEffect } from "react";
import { Sidebar } from "./components/Sidebar";
import { TitleBar } from "./components/TitleBar";
import { Toasts } from "./components/Toasts";
import { GameDrawer } from "./components/drawer/GameDrawer";
import { Dialogos } from "./components/flows/Dialogos";
import { RestoreDialog } from "./components/flows/RestoreDialog";
import { SelectFilesDialog } from "./components/flows/SelectFilesDialog";
import { Failed } from "./screens/Failed";
import { FirstRun } from "./screens/FirstRun";
import { Library } from "./screens/Library";
import { Settings } from "./screens/Settings";
import { VaultMissing } from "./screens/VaultMissing";
import { useApp } from "./store/app";
import { T } from "./texto";

function Principal() {
  const tela = useApp((s) => s.tela);
  return <main className="principal">{tela === "biblioteca" ? <Library /> : <Settings />}</main>;
}

function Pronto() {
  return (
    <div className="corpo">
      <Sidebar />
      <Principal />
      <GameDrawer />
      <RestoreDialog />
      <SelectFilesDialog />
      <Dialogos />
    </div>
  );
}

/** Raiz da interface: escolhe a tela pela fase do app. */
export function App() {
  const backend = useApp((s) => s.backend);
  const estado = useApp((s) => s.estado);
  const erroInicial = useApp((s) => s.erroInicial);
  const assistente = useApp((s) => s.assistente);
  const carregarEstado = useApp((s) => s.carregarEstado);
  const aplicarCapa = useApp((s) => s.aplicarCapa);
  const avisar = useApp((s) => s.avisar);
  const aplicarSituacao = useApp((s) => s.aplicarSituacao);
  const recarregarDoApp = useApp((s) => s.recarregarDoApp);
  const abrirJogoNoHistorico = useApp((s) => s.abrirJogoNoHistorico);

  useEffect(() => {
    void carregarEstado();
  }, [carregarEstado]);
  // Capas da Steam baixadas em segundo plano chegam por evento (também durante o primeiro uso).
  useEffect(() => backend.aoReceberCapa(aplicarCapa), [backend, aplicarCapa]);
  // O "Sair" espera o backup ou a restauração em andamento: avisa por quê.
  useEffect(() => backend.aoBloquearFechar(() => avisar("info", T.restaurar.fecharBloqueado)), [backend, avisar]);
  // O que o backup automático conta em segundo plano.
  useEffect(() => backend.aoMudarSituacao(aplicarSituacao), [backend, aplicarSituacao]);
  useEffect(() => backend.aoMudarBiblioteca(() => void recarregarDoApp()), [backend, recarregarDoApp]);
  useEffect(
    () => backend.aoFazerBackupAutomatico((itens) => avisar("ok", T.automatico.feito(itens.map((i) => i.title)))),
    [backend, avisar],
  );
  useEffect(() => backend.aoAbrirJogo(abrirJogoNoHistorico), [backend, abrirJogoNoHistorico]);
  useEffect(() => backend.aoAvisar((texto) => avisar("aviso", texto)), [backend, avisar]);

  let conteudo;
  if (erroInicial) conteudo = <Failed erro={erroInicial} pastaDeDados={null} />;
  else if (!estado) conteudo = <div className="carregando">{T.app.carregando}</div>;
  else if (assistente || estado.phase === "primeiroUso") conteudo = <FirstRun />;
  else if (estado.phase === "cofreSumiu") conteudo = <VaultMissing />;
  else if (estado.phase === "falhou") conteudo = <Failed erro={estado.error ?? ""} pastaDeDados={estado.dataDir} />;
  else conteudo = <Pronto />;

  return (
    <div className="app">
      <TitleBar />
      {conteudo}
      <Toasts />
    </div>
  );
}
