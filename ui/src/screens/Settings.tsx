import {
  AppWindow,
  Archive,
  Bell,
  Database,
  Eraser,
  EyeOff,
  Folder,
  FolderPlus,
  HardDrive,
  Image as ImageIcon,
  Info,
  RefreshCw,
} from "lucide-react";
import { useCallback, useEffect, useId, useRef, useState, type ReactNode } from "react";
import { ErroApp } from "../api/backend";
import type { AutoTimesView } from "../api/gerado/AutoTimesView";
import type { RetentionView } from "../api/gerado/RetentionView";
import type { SettingsView } from "../api/gerado/SettingsView";
import { Confirmacao, ConfirmarCofre } from "../components/flows/Confirmacao";
import { useApp, type CofreEscolhido } from "../store/app";
import { T } from "../texto";

function Secao({ titulo, icone, children }: { titulo: string; icone: ReactNode; children: ReactNode }) {
  const id = useId();
  return (
    <section className="cfg-secao" aria-labelledby={id}>
      <h2 id={id}>
        {icone}
        {titulo}
      </h2>
      {children}
    </section>
  );
}

function Chave({ rotulo, ligada, onMudar }: { rotulo: string; ligada: boolean; onMudar: (ligada: boolean) => void }) {
  return (
    <label className="chave-linha">
      <span>{rotulo}</span>
      <input type="checkbox" role="switch" className="chave" checked={ligada} onChange={(e) => onMudar(e.target.checked)} />
    </label>
  );
}

/** Um número inteiro digitado, ou null. */
function inteiro(s: string): number | null {
  return /^\d+$/.test(s.trim()) ? Number(s) : null;
}

/** Campo numérico curto, dentro de uma frase. */
function Numero({ rotulo, valor, onMudar }: { rotulo: string; valor: string; onMudar: (v: string) => void }) {
  return <input type="number" className="numero" aria-label={rotulo} value={valor} min={0} onChange={(e) => onMudar(e.target.value)} />;
}

/** Salva com o app; o erro (fora da faixa) aparece embaixo, como nos formulários. */
function useSalvar(onSalvo: () => void) {
  const avisar = useApp((s) => s.avisar);
  const [erro, setErro] = useState<string | null>(null);
  async function salvar(comando: () => Promise<void>, ok: string) {
    setErro(null);
    try {
      await comando();
      avisar("ok", ok);
      onSalvo();
    } catch (e) {
      setErro(ErroApp.de(e).message);
    }
  }
  return { erro, setErro, salvar };
}

/** "Guardar os últimos [10] · 1 por dia por [7] dias · 1 por semana por [4] semanas". */
function RetencaoGeral({ inicial, onSalvo }: { inicial: RetentionView; onSalvo: () => void }) {
  const backend = useApp((s) => s.backend);
  const { erro, setErro, salvar } = useSalvar(onSalvo);
  const [v, setV] = useState({
    autoRecent: String(inicial.autoRecent),
    autoDailyDays: String(inicial.autoDailyDays),
    autoWeeklyWeeks: String(inicial.autoWeeklyWeeks),
  });
  const [antes, dias, semanas, fim] = T.config.retencaoFrase;

  function enviar() {
    const r = { autoRecent: inteiro(v.autoRecent), autoDailyDays: inteiro(v.autoDailyDays), autoWeeklyWeeks: inteiro(v.autoWeeklyWeeks) };
    if (r.autoRecent === null || r.autoDailyDays === null || r.autoWeeklyWeeks === null) return setErro(T.config.numeroInvalido);
    const politica = r as RetentionView;
    void salvar(() => backend.definirRetencao(politica), T.config.retencaoSalva);
  }

  return (
    <div className="cfg-bloco">
      <b className="cfg-sub">{T.config.retencao}</b>
      <div className="cfg-frase">
        <span>{antes}</span>
        <Numero rotulo={T.config.campoUltimos} valor={v.autoRecent} onMudar={(x) => setV({ ...v, autoRecent: x })} />
        <span>{dias}</span>
        <Numero rotulo={T.config.campoDias} valor={v.autoDailyDays} onMudar={(x) => setV({ ...v, autoDailyDays: x })} />
        <span>{semanas}</span>
        <Numero rotulo={T.config.campoSemanas} valor={v.autoWeeklyWeeks} onMudar={(x) => setV({ ...v, autoWeeklyWeeks: x })} />
        <span>{fim}</span>
        <button type="button" className="btn btn-pequeno" aria-label={T.config.salvarRetencao} onClick={enviar}>
          {T.config.salvar}
        </button>
      </div>
      <p className="ajuda">{T.config.retencaoAjuda}</p>
      {erro ? (
        <p className="erro-form" role="alert">
          {erro}
        </p>
      ) : null}
    </div>
  );
}

/** Os 3 tempos do automático (§9), com as faixas conferidas pelo app. */
function Tempos({ inicial, onSalvo }: { inicial: AutoTimesView; onSalvo: () => void }) {
  const backend = useApp((s) => s.backend);
  const { erro, setErro, salvar } = useSalvar(onSalvo);
  const [v, setV] = useState({
    afterCloseSecs: String(inicial.afterCloseSecs),
    whileRunningMins: String(inicial.whileRunningMins),
    quietSecs: String(inicial.quietSecs),
  });
  const linhas: [keyof typeof v, readonly [string, string], string][] = [
    ["afterCloseSecs", T.config.tempoFechar, T.config.campoFechar],
    ["whileRunningMins", T.config.tempoAberto, T.config.campoAberto],
    ["quietSecs", T.config.tempoQuieto, T.config.campoQuieto],
  ];

  function enviar() {
    const t = { quietSecs: inteiro(v.quietSecs), afterCloseSecs: inteiro(v.afterCloseSecs), whileRunningMins: inteiro(v.whileRunningMins) };
    if (t.quietSecs === null || t.afterCloseSecs === null || t.whileRunningMins === null) return setErro(T.config.numeroInvalido);
    const tempos = t as AutoTimesView;
    void salvar(() => backend.definirTemposDoAutomatico(tempos), T.config.temposSalvos);
  }

  return (
    <div className="cfg-bloco">
      <b className="cfg-sub">{T.config.tempos}</b>
      {linhas.map(([k, [antes, depois], rotulo]) => (
        <div key={k} className="cfg-frase">
          <span>{antes}</span>
          <Numero rotulo={rotulo} valor={v[k]} onMudar={(x) => setV({ ...v, [k]: x })} />
          <span>{depois}</span>
        </div>
      ))}
      <div className="cfg-linha">
        <button type="button" className="btn btn-pequeno" aria-label={T.config.salvarTempos} onClick={enviar}>
          {T.config.salvar}
        </button>
      </div>
      {erro ? (
        <p className="erro-form" role="alert">
          {erro}
        </p>
      ) : null}
    </div>
  );
}

/**
 * Configurações (spec §12): cofre, pastas de jogos, base de jogos de PC, capas,
 * backup, backup automático, janela e Windows, notificações, registros do
 * Windows, jogos ignorados e créditos. O que muda o que o escaneamento acha
 * (pastas, configs, voltar a mostrar) escaneia de novo em seguida.
 */
export function Settings() {
  const backend = useApp((s) => s.backend);
  const avisar = useApp((s) => s.avisar);
  const escanear = useApp((s) => s.escanear);
  const escolherCofre = useApp((s) => s.escolherCofre);
  const trocarCofre = useApp((s) => s.trocarCofre);
  const [cfg, setCfg] = useState<SettingsView | null>(null);
  // O valor mais novo, para cliques seguidos não usarem uma lista velha.
  const atual = useRef<SettingsView | null>(null);
  const montado = useRef(true);
  const [atualizando, setAtualizando] = useState(false);
  const [resultadoBase, setResultadoBase] = useState<string | null>(null);
  const [escolha, setEscolha] = useState<CofreEscolhido | null>(null);
  const [removerRegistros, setRemoverRegistros] = useState(false);

  const mostrar = useCallback((novo: SettingsView) => {
    atual.current = novo;
    setCfg(novo);
  }, []);

  const recarregar = useCallback(async () => {
    try {
      const novo = await backend.configuracoes();
      if (montado.current) mostrar(novo);
    } catch (e) {
      if (montado.current) avisar("erro", ErroApp.de(e).message);
    }
  }, [backend, avisar, mostrar]);

  useEffect(() => {
    montado.current = true;
    void recarregar();
    return () => {
      montado.current = false;
    };
  }, [recarregar]);

  /** Mostra a mudança na hora e desfaz se o app recusar. */
  async function mudar(parcial: Partial<SettingsView>, comando: () => Promise<void>, depois?: () => void) {
    const antes = atual.current;
    if (!antes) return;
    mostrar({ ...antes, ...parcial });
    try {
      await comando();
      depois?.();
    } catch (e) {
      const volta = Object.fromEntries(Object.keys(parcial).map((k) => [k, antes[k as keyof SettingsView]]));
      if (atual.current) mostrar({ ...atual.current, ...volta });
      avisar("erro", ErroApp.de(e).message);
    }
  }

  const reescanear = () => void escanear();

  async function adicionarPasta() {
    let pasta: string | null;
    try {
      pasta = await backend.escolherPasta(T.config.adicionarTitulo);
    } catch (e) {
      avisar("erro", ErroApp.de(e).message);
      return;
    }
    const c = atual.current;
    if (!pasta || !c || c.userRoots.includes(pasta)) return;
    const lista = [...c.userRoots, pasta];
    await mudar({ userRoots: lista }, () => backend.definirPastasDeJogos(lista), reescanear);
  }

  async function removerPasta(pasta: string) {
    const c = atual.current;
    if (!c) return;
    const lista = c.userRoots.filter((p) => p !== pasta);
    await mudar({ userRoots: lista }, () => backend.definirPastasDeJogos(lista), reescanear);
  }

  async function voltarAMostrar(id: string) {
    const c = atual.current;
    if (!c) return;
    await mudar({ ignored: c.ignored.filter((g) => g.id !== id) }, () => backend.ignorarJogo(id, false), reescanear);
  }

  async function atualizarBase() {
    setAtualizando(true);
    setResultadoBase(null);
    try {
      const r = await backend.atualizarBase();
      if (r.outcome === "atualizada") setResultadoBase(T.config.baseAtualizada);
      else if (r.outcome === "semNovidade") setResultadoBase(T.config.baseEmDia);
      else setResultadoBase(r.message ?? T.config.baseFalhou);
      if (r.outcome !== "falhou" && atual.current) mostrar({ ...atual.current, manifestLoaded: true });
      if (r.outcome === "atualizada") reescanear();
    } catch (e) {
      setResultadoBase(ErroApp.de(e).message);
    } finally {
      setAtualizando(false);
    }
  }

  async function trocarPasta() {
    const e = await escolherCofre(T.config.trocarTitulo);
    if (e && e.pasta !== atual.current?.vaultDir) setEscolha(e);
  }

  async function confirmarTroca(e: CofreEscolhido) {
    setEscolha(null);
    const erro = await trocarCofre(e.pasta, !e.existe);
    if (erro) avisar("erro", erro);
    await recarregar();
  }

  async function confirmarRemocaoDosRegistros() {
    setRemoverRegistros(false);
    let ok = false;
    await mudar({ startWithWindows: false, notifications: false }, async () => {
      await backend.removerRegistrosDoWindows();
      ok = true;
    });
    if (ok) avisar("ok", T.config.registrosRemovidos);
  }

  const salvo = () => void recarregar();

  return (
    <div className="config">
      <div className="hd">
        <h1 className="t">{T.config.titulo}</h1>
      </div>
      <div className="config-rolagem">
        {cfg ? (
          <div className="config-coluna">
            <Secao titulo={T.config.cofre} icone={<HardDrive className="ic" aria-hidden="true" />}>
              <div className="cfg-linha">
                <span className="mono cfg-caminho" title={cfg.vaultDir}>
                  {cfg.vaultDir}
                </span>
                <button type="button" className="btn btn-pequeno" onClick={() => void trocarPasta()}>
                  {T.config.trocarPasta}
                </button>
              </div>
              <p className="ajuda">{T.config.notaCofre}</p>
            </Secao>

            <Secao titulo={T.config.pastas} icone={<Folder className="ic" aria-hidden="true" />}>
              <p className="ajuda">{T.config.pastasTexto}</p>
              {cfg.userRoots.length ? (
                <ul className="cfg-lista">
                  {cfg.userRoots.map((p) => (
                    <li key={p}>
                      <Folder className="ic" aria-hidden="true" />
                      <span className="mono" title={p}>
                        {p}
                      </span>
                      <button type="button" className="lnk-sec" aria-label={T.config.removerPasta(p)} onClick={() => void removerPasta(p)}>
                        {T.config.remover}
                      </button>
                    </li>
                  ))}
                </ul>
              ) : (
                <p className="cfg-vazio">{T.config.semPastas}</p>
              )}
              <button type="button" className="btn btn-pequeno" onClick={() => void adicionarPasta()}>
                <FolderPlus className="ic" aria-hidden="true" />
                {T.config.adicionarPasta}
              </button>
            </Secao>

            <Secao titulo={T.config.base} icone={<Database className="ic" aria-hidden="true" />}>
              <div className="cfg-linha">
                <span className={cfg.manifestLoaded ? "estado-base ok" : "estado-base"}>
                  <i aria-hidden="true" />
                  {cfg.manifestLoaded ? T.config.carregada : T.config.naoBaixada}
                </span>
                <button type="button" className="btn btn-pequeno" disabled={atualizando} onClick={() => void atualizarBase()}>
                  <RefreshCw className={atualizando ? "ic girando" : "ic"} aria-hidden="true" />
                  {atualizando ? T.config.atualizando : T.config.atualizar}
                </button>
              </div>
              {resultadoBase ? (
                <p className="cfg-resultado" role="status">
                  {resultadoBase}
                </p>
              ) : null}
            </Secao>

            <Secao titulo={T.config.capas} icone={<ImageIcon className="ic" aria-hidden="true" />}>
              <Chave
                rotulo={T.config.capasSteam}
                ligada={cfg.steamCovers}
                onMudar={(v) => void mudar({ steamCovers: v }, () => backend.definirCapasSteam(v))}
              />
            </Secao>

            <Secao titulo={T.config.backup} icone={<Archive className="ic" aria-hidden="true" />}>
              <Chave
                rotulo={T.config.incluirConfig}
                ligada={cfg.includeConfig}
                onMudar={(v) => void mudar({ includeConfig: v }, () => backend.definirIncluirConfig(v), reescanear)}
              />
            </Secao>

            <Secao titulo={T.config.automatico} icone={<RefreshCw className="ic" aria-hidden="true" />}>
              <Chave
                rotulo={T.config.fazerAutomatico}
                ligada={cfg.autoBackup}
                onMudar={(v) => void mudar({ autoBackup: v }, () => backend.definirAutomatico(v))}
              />
              <p className="ajuda">{T.config.automaticoAjuda}</p>
              <RetencaoGeral key={JSON.stringify(cfg.retention)} inicial={cfg.retention} onSalvo={salvo} />
              <Tempos key={JSON.stringify(cfg.autoTimes)} inicial={cfg.autoTimes} onSalvo={salvo} />
            </Secao>

            <Secao titulo={T.config.janela} icone={<AppWindow className="ic" aria-hidden="true" />}>
              <Chave
                rotulo={T.config.fecharParaBandeja}
                ligada={cfg.closeToTray}
                onMudar={(v) => void mudar({ closeToTray: v }, () => backend.definirFecharParaBandeja(v))}
              />
              <Chave
                rotulo={T.config.iniciarComWindows}
                ligada={cfg.startWithWindows}
                onMudar={(v) => void mudar({ startWithWindows: v }, () => backend.definirIniciarComWindows(v))}
              />
            </Secao>

            <Secao titulo={T.config.notificacoes} icone={<Bell className="ic" aria-hidden="true" />}>
              <Chave
                rotulo={T.config.mostrarNotificacoes}
                ligada={cfg.notifications}
                onMudar={(v) => void mudar({ notifications: v }, () => backend.definirNotificacoes(v))}
              />
              <p className="ajuda">{T.config.notificacoesAjuda}</p>
            </Secao>

            <Secao titulo={T.config.registros} icone={<Eraser className="ic" aria-hidden="true" />}>
              <p className="ajuda">{T.config.registrosAjuda}</p>
              <div className="cfg-linha">
                <button type="button" className="btn btn-pequeno" onClick={() => setRemoverRegistros(true)}>
                  {T.config.removerRegistros}
                </button>
              </div>
            </Secao>

            <Secao titulo={T.config.ignorados} icone={<EyeOff className="ic" aria-hidden="true" />}>
              {cfg.ignored.length ? (
                <ul className="cfg-lista">
                  {cfg.ignored.map((g) => (
                    <li key={g.id}>
                      <span className="cfg-nome">{g.title}</span>
                      <button type="button" className="lnk-sec" onClick={() => void voltarAMostrar(g.id)}>
                        {T.config.voltarAMostrar}
                      </button>
                    </li>
                  ))}
                </ul>
              ) : (
                <p className="cfg-vazio">{T.config.nenhumIgnorado}</p>
              )}
            </Secao>

            <Secao titulo={T.config.sobre} icone={<Info className="ic" aria-hidden="true" />}>
              <p className="cfg-versao">{T.config.versao(cfg.version)}</p>
              {T.config.creditos.map((c) => (
                <p key={c} className="ajuda">
                  {c}
                </p>
              ))}
            </Secao>
          </div>
        ) : (
          <p className="vazio">{T.config.carregando}</p>
        )}
      </div>
      {escolha ? (
        <ConfirmarCofre escolha={escolha} onConfirmar={() => void confirmarTroca(escolha)} onCancelar={() => setEscolha(null)} />
      ) : null}
      {removerRegistros ? (
        <Confirmacao
          titulo={T.config.removerRegistrosTitulo}
          confirmar={T.config.removerRegistrosConfirmar}
          onConfirmar={() => void confirmarRemocaoDosRegistros()}
          onCancelar={() => setRemoverRegistros(false)}
        />
      ) : null}
    </div>
  );
}
