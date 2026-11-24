import { useEffect } from "react";
import { Sidebar } from "./components/Sidebar";
import { TitleBar } from "./components/TitleBar";
import { useApp } from "./store/app";
import { T } from "./texto";

function Principal() {
  const tela = useApp((s) => s.tela);
  return (
    <main className="principal">
      <section aria-label={tela === "biblioteca" ? T.menu.biblioteca : T.menu.configuracoes} />
    </main>
  );
}

function Pronto() {
  return (
    <div className="corpo">
      <Sidebar />
      <Principal />
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
    </div>
  );
}
