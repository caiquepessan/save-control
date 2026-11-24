import { Archive, Database, EyeOff, Folder, FolderPlus, HardDrive, Image as ImageIcon, Info, RefreshCw } from "lucide-react";
import { useCallback, useEffect, useId, useRef, useState, type ReactNode } from "react";
import { ErroApp } from "../api/backend";
import type { SettingsView } from "../api/gerado/SettingsView";
import { ConfirmarCofre } from "../components/flows/Confirmacao";
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

/**
 * Configurações (spec §12): cofre, pastas de jogos, base de jogos de PC, capas,
 * backup, jogos ignorados e créditos. O que muda o que o escaneamento acha
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
    </div>
  );
}
