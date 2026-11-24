import { useEffect } from "react";
import { Sidebar } from "./components/Sidebar";
import { TitleBar } from "./components/TitleBar";
import { Toasts } from "./components/Toasts";
import { GameDrawer } from "./components/drawer/GameDrawer";
import { Library } from "./screens/Library";
import { useApp } from "./store/app";
import { T } from "./texto";

function Principal() {
  const tela = useApp((s) => s.tela);
  return (
    <main className="principal">
      {tela === "biblioteca" ? <Library /> : <section aria-label={T.menu.configuracoes} />}
    </main>
  );
}

function Pronto() {
  const backend = useApp((s) => s.backend);
  const aplicarCapa = useApp((s) => s.aplicarCapa);
  // Capas da Steam baixadas em segundo plano chegam por evento.
  useEffect(() => backend.aoReceberCapa(aplicarCapa), [backend, aplicarCapa]);
  return (
    <div className="corpo">
      <Sidebar />
      <Principal />
      <GameDrawer />
    </div>
  );
}

/** Raiz da interface: escolhe a tela pela fase do app. */
export function App() {
  const estado = useApp((s) => s.estado);
  const erroInicial = useApp((s) => s.erroInicial);
  const carregarEstado = useApp((s) => s.carregarEstado);

  useEffect(() => {
    void carregarEstado();
  }, [carregarEstado]);

  let conteudo;
  if (erroInicial) conteudo = <div className="carregando">{erroInicial}</div>;
  else if (!estado) conteudo = <div className="carregando">{T.app.carregando}</div>;
  else if (estado.phase === "pronto") conteudo = <Pronto />;
  else conteudo = <div className="carregando" data-fase={estado.phase} />;

  return (
    <div className="app">
      <TitleBar />
      {conteudo}
      <Toasts />
    </div>
  );
}
