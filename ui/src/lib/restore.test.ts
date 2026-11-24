import type { GameView } from "../api/gerado/GameView";
import type { RestorePreviewView } from "../api/gerado/RestorePreviewView";
import { textosDaRestauracao } from "./restore";

const MB = 1024 * 1024;
const agora = new Date(2026, 10, 24, 12, 0);

const previa: RestorePreviewView = {
  snapshot: {
    fileName: "2026-11-24_10-02-00_ESTE-PC-1A2B_manual.zip",
    kind: "manual",
    label: "Antes de atualizar o recomp",
    createdAt: new Date(2026, 10, 24, 10, 2).toISOString(),
    pcName: "ESTE-PC",
    fromThisPc: true,
    totalSize: Math.round(1.2 * MB),
    fileCount: 14,
  },
  targets: [{ alias: "r0", template: "<winLocalAppData>/Reino/user/save", path: "C:\\Users\\voce\\AppData\\Local\\Reino\\user\\save" }],
  fileCount: 14,
  totalSize: Math.round(1.2 * MB),
  running: false,
  singleRoot: true,
};

const jogo = { title: "Reino Esmeralda: A Flauta do Tempo", processNames: ["Jogo.exe"] } as GameView;

describe("textos da confirmação de restaurar", () => {
  it("título, texto e linhas do mockup", () => {
    const t = textosDaRestauracao(previa, jogo, { maisAntiga: false }, agora);
    expect(t.titulo).toBe('Restaurar "Antes de atualizar o recomp"?');
    expect(t.corpo).toBe(
      "O save do A Flauta do Tempo volta a como estava hoje, 10:02. Antes disso, o save atual é guardado no cofre, então dá pra desfazer.",
    );
    expect(t.versao).toBe("hoje, 10:02 · manual");
    expect(t.arquivos).toBe("14 arquivos · 1,2 MB");
    expect(t.destinos).toEqual([{ caminho: "C:\\Users\\voce\\AppData\\Local\\Reino\\user\\save", curto: "…\\Reino\\user\\save" }]);
    expect(t.aviso).toBeNull();
  });

  it("jogo aberto: pede para fechar pelo nome do processo", () => {
    const t = textosDaRestauracao({ ...previa, running: true }, jogo, { maisAntiga: false }, agora);
    expect(t.aviso).toBe("O jogo está aberto. Feche Jogo antes de restaurar.");
    const semProcesso = textosDaRestauracao({ ...previa, running: true }, { ...jogo, processNames: [] }, { maisAntiga: false }, agora);
    expect(semProcesso.aviso).toBe("O jogo está aberto. Feche A Flauta do Tempo antes de restaurar.");
  });

  it("restauração seletiva conta só os arquivos escolhidos", () => {
    const t = textosDaRestauracao(previa, jogo, { maisAntiga: false, soArquivos: 3 }, agora);
    expect(t.arquivos).toBe("3 de 14 arquivos");
  });

  it("a versão mais antiga sem nome é o primeiro backup", () => {
    const semNome = { ...previa, snapshot: { ...previa.snapshot, label: null } };
    expect(textosDaRestauracao(semNome, jogo, { maisAntiga: true }, agora).titulo).toBe('Restaurar "Primeiro backup"?');
  });
});
