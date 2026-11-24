// Todos os textos da interface ficam aqui (spec §2: centralizados para traduzir depois).

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
  cofre: {
    titulo: "Cofre",
    versoes: (tam: string, n: number) => `${tam} · ${n} ${n === 1 ? "versão guardada" : "versões guardadas"}`,
    indisponivel: "Cofre indisponível",
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
