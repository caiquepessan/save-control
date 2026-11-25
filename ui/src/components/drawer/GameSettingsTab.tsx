import { AlertTriangle, EyeOff } from "lucide-react";
import { useEffect, useState } from "react";
import type { GameView } from "../../api/gerado/GameView";
import type { RetentionView } from "../../api/gerado/RetentionView";
import { useApp } from "../../store/app";
import { T } from "../../texto";

type Campos = Record<keyof RetentionView, string>;

const CAMPOS: (keyof RetentionView)[] = ["autoRecent", "autoDailyDays", "autoWeeklyWeeks"];
const ROTULO: Record<keyof RetentionView, string> = {
  autoRecent: T.ajustesJogo.ultimos,
  autoDailyDays: T.ajustesJogo.dias,
  autoWeeklyWeeks: T.ajustesJogo.semanas,
};

const paraCampos = (r: RetentionView): Campos => ({
  autoRecent: String(r.autoRecent),
  autoDailyDays: String(r.autoDailyDays),
  autoWeeklyWeeks: String(r.autoWeeklyWeeks),
});

/** Os 3 números, se estiverem nas faixas (últimos de 1 a 99; dias e semanas de 0 a 99). */
function lerCampos(c: Campos): RetentionView | null {
  const n = (s: string, min: number) => (/^\d{1,2}$/.test(s.trim()) && Number(s) >= min ? Number(s) : null);
  const r = { autoRecent: n(c.autoRecent, 1), autoDailyDays: n(c.autoDailyDays, 0), autoWeeklyWeeks: n(c.autoWeeklyWeeks, 0) };
  return r.autoRecent === null || r.autoDailyDays === null || r.autoWeeklyWeeks === null ? null : (r as RetentionView);
}

/** A chave do automático só deste jogo. */
function Automatico({ jogo }: { jogo: GameView }) {
  const definir = useApp((s) => s.definirAutomaticoDoJogo);
  const [ligado, setLigado] = useState(jogo.autoBackup);
  async function mudar(v: boolean) {
    setLigado(v);
    if (!(await definir(jogo.id, v))) setLigado(!v);
  }
  return (
    <section className="ajuste">
      <label className="chave-linha">
        <span className="ajuste-titulo">{T.ajustesJogo.automatico}</span>
        <input type="checkbox" role="switch" className="chave" checked={ligado} onChange={(e) => void mudar(e.target.checked)} />
      </label>
      <p className="ajuda">{T.ajustesJogo.automaticoAjuda}</p>
    </section>
  );
}

/** Retenção dos automáticos: o padrão das Configurações ou 3 números só deste jogo. */
function Retencao({ jogo, padrao }: { jogo: GameView; padrao: RetentionView | null }) {
  const definir = useApp((s) => s.definirRetencaoDoJogo);
  const avisar = useApp((s) => s.avisar);
  const [personalizado, setPersonalizado] = useState(jogo.retention !== null);
  const [campos, setCampos] = useState<Campos | null>(jogo.retention ? paraCampos(jogo.retention) : null);
  const valores = campos ?? paraCampos(padrao ?? { autoRecent: 10, autoDailyDays: 7, autoWeeklyWeeks: 4 });
  const lida = lerCampos(valores);

  async function usarPadrao() {
    setPersonalizado(false);
    if (jogo.retention !== null && !(await definir(jogo.id, null))) setPersonalizado(true);
  }

  async function salvar() {
    if (lida && (await definir(jogo.id, lida))) avisar("ok", T.ajustesJogo.retencaoSalva);
  }

  return (
    <section className="ajuste">
      <b className="ajuste-titulo" id="titulo-retencao">
        {T.ajustesJogo.retencao}
      </b>
      <div className="escolhas" role="radiogroup" aria-labelledby="titulo-retencao">
        <label className="escolha">
          <input type="radio" name="retencao" checked={!personalizado} onChange={() => void usarPadrao()} />
          {T.ajustesJogo.usarPadrao(padrao)}
        </label>
        <label className="escolha">
          <input type="radio" name="retencao" checked={personalizado} onChange={() => setPersonalizado(true)} />
          {T.ajustesJogo.personalizar}
        </label>
      </div>
      {personalizado ? (
        <div className="retencao-campos">
          {CAMPOS.map((k) => (
            <label key={k} className="campo-numero">
              <span>{ROTULO[k]}</span>
              <input
                type="number"
                min={k === "autoRecent" ? 1 : 0}
                max={99}
                value={valores[k]}
                onChange={(e) => setCampos({ ...valores, [k]: e.target.value })}
              />
            </label>
          ))}
          <button type="button" className="btn btn-pequeno" disabled={!lida} onClick={() => void salvar()}>
            {T.ajustesJogo.salvar}
          </button>
        </div>
      ) : null}
      {personalizado && !lida ? <p className="ajuda alerta">{T.ajustesJogo.faixas}</p> : <p className="ajuda">{T.ajustesJogo.retencaoAjuda}</p>}
    </section>
  );
}

/** Incluir os arquivos de configuração: como nas Configurações, sim ou não. */
function Configs({ jogo }: { jogo: GameView }) {
  const definir = useApp((s) => s.definirIncluirConfigDoJogo);
  const [valor, setValor] = useState<boolean | null>(jogo.includeConfig);
  const opcoes: [boolean | null, string][] = [
    [null, T.ajustesJogo.configsIgual],
    [true, T.ajustesJogo.configsIncluir],
    [false, T.ajustesJogo.configsNao],
  ];
  async function escolher(v: boolean | null) {
    const antes = valor;
    setValor(v);
    if (!(await definir(jogo.id, v))) setValor(antes);
  }
  return (
    <section className="ajuste">
      <b className="ajuste-titulo" id="titulo-configs">
        {T.ajustesJogo.configs}
      </b>
      <div className="escolhas" role="radiogroup" aria-labelledby="titulo-configs">
        {opcoes.map(([v, rotulo]) => (
          <label key={String(v)} className="escolha">
            <input type="radio" name="configs" checked={valor === v} onChange={() => void escolher(v)} />
            {rotulo}
          </label>
        ))}
      </div>
    </section>
  );
}

/** Ajustes deste jogo neste PC: automático, retenção, configs, pasta do jogo, destinos lembrados e ignorar. */
export function GameSettingsTab({ jogo }: { jogo: GameView }) {
  const backend = useApp((s) => s.backend);
  const escolherPasta = useApp((s) => s.backend.escolherPasta);
  const definirPastaDoJogo = useApp((s) => s.definirPastaDoJogo);
  const esquecerDestino = useApp((s) => s.esquecerDestino);
  const ignorar = useApp((s) => s.ignorar);
  const [padrao, setPadrao] = useState<RetentionView | null>(null);

  // O padrão da retenção vem das Configurações.
  useEffect(() => {
    let vivo = true;
    backend
      .configuracoes()
      .then((c) => {
        if (vivo) setPadrao(c.retention);
      })
      .catch(() => {});
    return () => {
      vivo = false;
    };
  }, [backend]);

  const escolher = async () => {
    const pasta = await escolherPasta(T.ajustesJogo.escolherTitulo);
    if (pasta) await definirPastaDoJogo(jogo.id, pasta);
  };

  return (
    <div className="ajustes-jogo">
      <Automatico key={`auto-${jogo.id}-${jogo.autoBackup}`} jogo={jogo} />
      <Retencao key={`ret-${jogo.id}-${JSON.stringify(jogo.retention)}`} jogo={jogo} padrao={padrao} />
      <Configs key={`cfg-${jogo.id}-${String(jogo.includeConfig)}`} jogo={jogo} />

      <section className="ajuste">
        <b className="ajuste-titulo">{T.ajustesJogo.pastaDoJogo}</b>
        <p className="ajuda">{T.ajustesJogo.ajuda}</p>
        {jogo.needsGameDir ? (
          <div className="warnbox" role="note">
            <AlertTriangle className="ic" aria-hidden="true" />
            <span>{T.ajustesJogo.precisa}</span>
          </div>
        ) : null}
        <div className="pasta-escolhida">
          <span className="mono" title={jogo.gameDir ?? undefined}>
            {jogo.gameDir ?? T.ajustesJogo.nenhuma}
          </span>
          <button type="button" className="btn btn-pequeno" onClick={() => void escolher()}>
            {T.ajustesJogo.escolher}
          </button>
          {jogo.gameDir ? (
            <button type="button" className="btn btn-pequeno" onClick={() => void definirPastaDoJogo(jogo.id, null)}>
              {T.ajustesJogo.limpar}
            </button>
          ) : null}
        </div>
      </section>

      <section className="ajuste">
        <b className="ajuste-titulo">{T.ajustesJogo.destinos}</b>
        {jogo.rememberedTargets.length ? (
          jogo.rememberedTargets.map((d) => (
            <div className="destino-lembrado" key={d.template}>
              <span className="mono" title={d.template}>
                {d.template}
              </span>
              <span aria-hidden="true">→</span>
              <span className="mono" title={d.path}>
                {d.path}
              </span>
              <button type="button" className="lnk" onClick={() => void esquecerDestino(jogo.id, d.template)}>
                {T.ajustesJogo.esquecer}
              </button>
            </div>
          ))
        ) : (
          <p className="ajuda">{T.ajustesJogo.semDestinos}</p>
        )}
      </section>

      <section className="ajuste">
        <button type="button" className="btn btn-pequeno" onClick={() => void ignorar(jogo.id)}>
          <EyeOff className="ic" aria-hidden="true" />
          {T.ajustesJogo.ignorar}
        </button>
      </section>
    </div>
  );
}
