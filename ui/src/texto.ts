// Todos os textos da interface ficam aqui (spec §2: centralizados para traduzir depois).

const plural = (n: number, um: string, varios: string) => (n === 1 ? um : varios);

export const T = {
  app: {
    nome: "Save Control",
    carregando: "Abrindo o Save Control…",
  },
  janela: {
    minimizar: "Minimizar",
    maximizar: "Maximizar",
    fechar: "Fechar",
  },
  menu: {
    rotulo: "Menu",
    biblioteca: "Biblioteca",
    configuracoes: "Configurações",
    fontes: "Fontes",
  },
  fontes: {
    steam: "Steam",
    epic: "Epic Games",
    gog: "GOG",
    pcOutros: "Outros jogos de PC",
    recomp: "Recomps & Ports",
    emulador: "Emuladores",
    manual: "Adicionados à mão",
    cofre: "Só no cofre",
  },
  cofre: {
    titulo: "Cofre",
    versoes: (tam: string, n: number) => `${tam} · ${n} ${plural(n, "versão guardada", "versões guardadas")}`,
    indisponivel: "Cofre indisponível",
  },
  biblioteca: {
    busca: "Buscar jogo, emulador ou pasta…",
    atalhoBusca: "Ctrl K",
    escanear: "Escanear",
    escaneando: "Escaneando…",
    adicionar: "Adicionar jogo",
    backupDeTudo: "Backup de tudo",
    titulo: "Seus jogos",
    resumo: (n: number, p: number) =>
      `${n} ${plural(n, "encontrado", "encontrados")} · ${p} ${plural(p, "protegido", "protegidos")} · ordenados pelo último save`,
    vazia: "Nenhum jogo encontrado ainda. Escaneie de novo ou adicione um jogo à mão.",
    filtroVazio: "Nada aqui com esse filtro.",
    chips: {
      todos: "Todos",
      pc: "PC",
      recomp: "Recomps & Ports",
      emulador: "Emuladores",
      atencao: "Precisa de atenção",
    },
  },
  status: {
    protegido: "Protegido",
    saveNovo: "Save novo",
    semBackup: "Sem backup",
    semSave: "Sem save",
    soNoCofre: "Só no cofre",
  },
  cartao: {
    abrir: (titulo: string) => `Abrir ${titulo}`,
    fazerBackup: "Fazer backup",
    restaurar: "Restaurar…",
    jogandoAgora: "jogando agora",
    backup: (quando: string) => `backup ${quando}`,
    nenhumSave: "nenhum save ainda",
  },
  alerta: {
    rotulo: "Aviso",
    umSemBackup: (titulo: string) => `${titulo} ainda não tem nenhum backup`,
    variosSemBackup: (n: number) => `${n} jogos ainda não têm nenhum backup`,
    ondeFoiAchado: (caminho: string, portatil: boolean) =>
      `Encontrado agora no escaneamento, em ${caminho}${portatil ? " (modo portátil)" : ""}.`,
    lista: (titulos: string[]) =>
      titulos.length <= 2 ? `${titulos.join(" e ")}.` : `${titulos[0]}, ${titulos[1]} e mais ${titulos.length - 2}.`,
    fazerBackupAgora: "Fazer backup agora",
    cofreTitulo: "O cofre não está acessível",
    cofreTexto: (caminho: string) => `${caminho}. Se ele fica num HD externo, conecte o HD.`,
    tentarDeNovo: "Tentar de novo",
    arriscadaTitulo: (n: number) => `${n} ${plural(n, "jogo", "jogos")} com save em pasta arriscada`,
    verJogos: "Ver jogos",
  },
  backup: {
    feito: (titulo: string) => `Backup feito · ${titulo}`,
    semMudanca: (titulo: string) => `Nada mudou desde o último backup de ${titulo}`,
    semArquivos: (titulo: string) => `${titulo} não tem arquivos de save para guardar`,
    varios: (n: number) => `${n} ${plural(n, "backup feito", "backups feitos")}`,
    nadaParaFazer: "Nada para fazer: todos os jogos já estão protegidos.",
    falhas: (feitos: number, titulos: string[]) =>
      `${feitos} ${plural(feitos, "backup feito", "backups feitos")}. Não deram certo: ${titulos.join(", ")}.`,
  },
  toasts: {
    fechar: "Fechar aviso",
  },
  tempo: {
    agora: "agora",
    hoje: "hoje",
    ontem: "ontem",
    minutos: (n: number) => `há ${n} min`,
    horas: (n: number) => `há ${n} h`,
    dias: (n: number) => `há ${n} d`,
    semanas: (n: number) => `há ${n} sem`,
    meses: ["jan", "fev", "mar", "abr", "mai", "jun", "jul", "ago", "set", "out", "nov", "dez"],
  },
};
