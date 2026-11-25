import { Check, CircleAlert, FolderOpen, HardDrive, LoaderCircle, ShieldCheck } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { ErroApp } from "../api/backend";
import type { BackupManyView } from "../api/gerado/BackupManyView";
import type { GameView } from "../api/gerado/GameView";
import { Logo } from "../components/Logo";
import { COR_FONTE, FONTES, contagens } from "../lib/library";
import { useApp, type CofreEscolhido } from "../store/app";
import { T } from "../texto";

type Etapa = "boasVindas" | "cofre" | "procurando" | "resumo" | "backup";
type Progresso = "esperando" | "rodando" | "ok" | "falhou";

const ETAPAS: Etapa[] = ["boasVindas", "cofre", "procurando", "resumo", "backup"];
const NENHUM: GameView[] = [];
const SEM_RESULTADO: BackupManyView = { done: [], unchanged: [], failed: [] };

/** Os pontos que mostram em que passo a pessoa está. */
function Pontos({ etapa }: { etapa: Etapa }) {
  const atual = ETAPAS.indexOf(etapa);
  return (
    <div className="pontos" role="img" aria-label={T.primeiroUso.passo(atual + 1, ETAPAS.length)}>
      {ETAPAS.map((e, i) => (
        <i key={e} className={i < atual ? "feito" : i === atual ? "on" : undefined} />
      ))}
    </div>
  );
}

function Passo({ estado, texto }: { estado: Progresso; texto: string }) {
  let icone;
  if (estado === "rodando") icone = <LoaderCircle className="ic girando" aria-hidden="true" />;
  else if (estado === "ok") icone = <Check className="ic" aria-hidden="true" />;
  else if (estado === "falhou") icone = <CircleAlert className="ic" aria-hidden="true" />;
  else icone = <i className="espera" aria-hidden="true" />;
  return (
    <li className={`passo ${estado}`}>
      {icone}
      <span>{texto}</span>
    </li>
  );
}

/**
 * Primeiro uso (spec §12): boas-vindas, onde fica o cofre, escaneamento,
 * resumo e o primeiro backup de tudo. O app já fica pronto no meio do caminho;
 * a tela só some quando a pessoa abre a biblioteca.
 */
export function FirstRun() {
  const backend = useApp((s) => s.backend);
  const padrao = useApp((s) => s.estado?.defaultVault ?? "");
  const jogos = useApp((s) => s.biblioteca?.games ?? NENHUM);
  const aplicarEstado = useApp((s) => s.aplicarEstado);
  const concluirAssistente = useApp((s) => s.concluirAssistente);
  const escolherCofre = useApp((s) => s.escolherCofre);
  const escanear = useApp((s) => s.escanear);

  const [etapa, setEtapa] = useState<Etapa>("boasVindas");
  const [cofre, setCofre] = useState<CofreEscolhido>({ pasta: padrao, existe: false, versoes: 0 });
  const [erro, setErro] = useState<string | null>(null);
  const [base, setBase] = useState<Progresso>("esperando");
  const [saves, setSaves] = useState<Progresso>("esperando");
  const [resultado, setResultado] = useState<BackupManyView | null>(null);
  const titulo = useRef<HTMLHeadingElement>(null);

  // A cada passo, o foco vai para o título: teclado e leitor de tela acompanham.
  useEffect(() => {
    if (etapa !== "boasVindas") titulo.current?.focus({ preventScroll: true });
  }, [etapa]);

  const comSave = useMemo(() => jogos.filter((j) => !j.vaultOnly && j.status !== "semSave"), [jogos]);
  const porFonte = useMemo(() => contagens(comSave).fontes, [comSave]);

  async function irParaOCofre() {
    setEtapa("cofre");
    try {
      const sonda = await backend.inspecionarCofre(padrao);
      // Só vale se a pessoa ainda não escolheu outra pasta.
      setCofre((c) => (c.pasta === padrao ? { pasta: sonda.path, existe: sonda.exists, versoes: sonda.versions } : c));
    } catch {
      // Sem a inspeção, o cofre padrão é tratado como novo.
    }
  }

  async function escolherOutra() {
    setErro(null);
    const escolha = await escolherCofre(T.primeiroUso.escolherTitulo);
    if (escolha) setCofre(escolha);
  }

  async function procurar() {
    setErro(null);
    setEtapa("procurando");
    let novo;
    try {
      // Primeiro o cofre: a base e o escaneamento precisam do app aberto nele.
      novo = await backend.concluirPrimeiroUso(cofre.pasta);
    } catch (e) {
      setErro(ErroApp.de(e).message);
      setEtapa("cofre");
      return;
    }
    await aplicarEstado(novo);
    if (novo.phase !== "pronto") {
      await concluirAssistente();
      return;
    }
    setBase("rodando");
    try {
      const r = await backend.atualizarBase();
      setBase(r.outcome === "falhou" ? "falhou" : "ok");
    } catch {
      setBase("falhou");
    }
    setSaves("rodando");
    await escanear();
    setSaves("ok");
    setEtapa("resumo");
  }

  async function backupDeTudo() {
    setErro(null);
    setEtapa("backup");
    try {
      const ids = await backend.jogosParaBackup();
      setResultado(ids.length ? await backend.backupDeVarios(ids) : SEM_RESULTADO);
    } catch (e) {
      setErro(ErroApp.de(e).message);
      setResultado(SEM_RESULTADO);
    }
  }

  const semInternet = base === "falhou" ? <p className="nota aviso">{T.primeiroUso.semInternet}</p> : null;
  const erroNaTela = erro ? (
    <p className="erro-form" role="alert">
      {erro}
    </p>
  ) : null;

  let conteudo;
  switch (etapa) {
    case "boasVindas":
      conteudo = (
        <>
          <Logo className="logo-grande" />
          <h1>{T.primeiroUso.boasVindas}</h1>
          <p className="assistente-texto">{T.primeiroUso.boasVindasTexto}</p>
          <div className="assistente-botoes">
            <button type="button" className="btn pri" onClick={() => void irParaOCofre()}>
              {T.primeiroUso.comecar}
            </button>
          </div>
        </>
      );
      break;
    case "cofre":
      conteudo = (
        <>
          <span className="assistente-icone">
            <HardDrive className="ic" aria-hidden="true" />
          </span>
          <h1 ref={titulo} tabIndex={-1}>
            {T.primeiroUso.cofreTitulo}
          </h1>
          <p className="assistente-texto">{T.primeiroUso.cofreTexto}</p>
          <div className="pasta-cofre">
            <FolderOpen className="ic" aria-hidden="true" />
            <span className="mono">{cofre.pasta}</span>
          </div>
          {cofre.existe ? <p className="nota ok">{T.primeiroUso.cofreExiste(cofre.versoes)}</p> : null}
          {erroNaTela}
          <div className="assistente-botoes">
            <button type="button" className="btn" onClick={() => void escolherOutra()}>
              {T.primeiroUso.escolherOutra}
            </button>
            <button type="button" className="btn pri" onClick={() => void procurar()}>
              {T.primeiroUso.continuar}
            </button>
          </div>
        </>
      );
      break;
    case "procurando":
      conteudo = (
        <>
          <span className="assistente-icone">
            <LoaderCircle className="ic girando" aria-hidden="true" />
          </span>
          <h1 ref={titulo} tabIndex={-1}>
            {T.primeiroUso.procurando}
          </h1>
          <div className="barra-andando" aria-hidden="true">
            <i />
          </div>
          <ul className="passos">
            <Passo estado={base} texto={T.primeiroUso.etapaBase} />
            <Passo estado={saves} texto={T.primeiroUso.etapaSaves} />
          </ul>
          {semInternet}
        </>
      );
      break;
    case "resumo":
      conteudo = (
        <>
          <span className="assistente-icone">
            <ShieldCheck className="ic" aria-hidden="true" />
          </span>
          <h1 ref={titulo} tabIndex={-1}>
            {comSave.length ? T.primeiroUso.encontrados(comSave.length) : T.primeiroUso.nenhum}
          </h1>
          {comSave.length ? (
            <div className="chips centro">
              {FONTES.filter((f) => porFonte[f] > 0).map((f) => (
                <span key={f} className="chip">
                  <i className="cor" style={{ background: COR_FONTE[f] }} aria-hidden="true" />
                  {T.fontes[f]} <b>{porFonte[f]}</b>
                </span>
              ))}
            </div>
          ) : (
            <p className="assistente-texto">{T.primeiroUso.nenhumTexto}</p>
          )}
          <p className="nota info">{T.primeiroUso.dicaPastas}</p>
          {semInternet}
          <div className="assistente-botoes">
            {comSave.length ? (
              <>
                <button type="button" className="btn" onClick={() => void concluirAssistente()}>
                  {T.primeiroUso.agoraNao}
                </button>
                <button type="button" className="btn pri" onClick={() => void backupDeTudo()}>
                  {T.primeiroUso.backupDeTudo}
                </button>
              </>
            ) : (
              <button type="button" className="btn pri" onClick={() => void concluirAssistente()}>
                {T.primeiroUso.abrirBiblioteca}
              </button>
            )}
          </div>
        </>
      );
      break;
    case "backup": {
      const protegidos = resultado ? resultado.done.length + resultado.unchanged.length : 0;
      conteudo = (
        <>
          <span className={resultado ? "assistente-icone ok" : "assistente-icone"}>
            {resultado ? (
              <ShieldCheck className="ic" aria-hidden="true" />
            ) : (
              <LoaderCircle className="ic girando" aria-hidden="true" />
            )}
          </span>
          <h1 ref={titulo} tabIndex={-1} aria-live="polite">
            {resultado ? T.primeiroUso.pronto(protegidos) : T.primeiroUso.fazendoBackup}
          </h1>
          {resultado ? (
            <>
              {resultado.failed.length ? (
                <p className="nota aviso">{T.primeiroUso.falhas(resultado.failed.map((f) => f.title))}</p>
              ) : null}
              {erroNaTela}
              <div className="assistente-botoes">
                <button type="button" className="btn pri" onClick={() => void concluirAssistente()}>
                  {T.primeiroUso.abrirBiblioteca}
                </button>
              </div>
            </>
          ) : (
            <div className="barra-andando" aria-hidden="true">
              <i />
            </div>
          )}
        </>
      );
      break;
    }
  }

  return (
    <main className="assistente">
      <div className="assistente-caixa">
        <Pontos etapa={etapa} />
        {conteudo}
      </div>
    </main>
  );
}
