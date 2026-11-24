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

  useEffect(() => {
    void carregarEstado();
  }, [carregarEstado]);
  // Capas da Steam baixadas em segundo plano chegam por evento (também durante o primeiro uso).
  useEffect(() => backend.aoReceberCapa(aplicarCapa), [backend, aplicarCapa]);
  // A janela não fecha no meio de um backup ou restauração: avisa por quê.
  useEffect(() => backend.aoBloquearFechar(() => avisar("info", T.restaurar.fecharBloqueado)), [backend, avisar]);

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
