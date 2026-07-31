export type ReleaseNote = {
  version: string;
  date: string;
  title: string;
  sections: Array<{
    title: string;
    items: string[];
  }>;
};

// Bundled with the app so the version history remains available offline.
export const releaseNotes: ReleaseNote[] = [
  {
    version: "v0.1.2",
    date: "31/07/2026",
    title: "Relatórios interativos e fluxo de OS aprimorado",
    sections: [
      {
        title: "Relatórios e gráficos",
        items: [
          "Gráficos interativos de evolução financeira, categorias, técnicos e itens mais vendidos.",
          "Filtros rápidos de período e filtro por responsável técnico.",
          "Ranking configurável por faturamento ou quantidade, entre Top 5 e Top 20.",
          "Correções de descontos, categorias históricas, contagens de OS e novas OS por data de criação.",
        ],
      },
      {
        title: "Ordens de serviço e estoque",
        items: [
          "Cancelar uma OS devolve as peças sem remover os itens do histórico.",
          "Reativar uma OS baixa as peças novamente somente quando houver saldo disponível.",
          "Crie e adicione peças ou serviços durante a edição de uma OS.",
          "Peças novas aceitam quantidade inicial em estoque.",
        ],
      },
      {
        title: "Backup e interface",
        items: [
          "A senha de backup é validada antes da restauração.",
          "Histórico offline de versões e notas exibidas após atualizar o aplicativo.",
          "Melhorias de responsividade, geração de PDF, textos e ícones da barra lateral.",
        ],
      },
    ],
  },
  {
    version: "v0.1.1",
    date: "30/07/2026",
    title: "Criação rápida e proteção de dados",
    sections: [
      {
        title: "Novidades",
        items: [
          "Crie checklists, peças e serviços sem sair da nova ordem de serviço.",
          "Consulte o histórico de ordens de serviço de um cliente pela tela de clientes ou pela lista de OS.",
          "Receba um aviso discreto quando houver uma atualização disponível.",
        ],
      },
      {
        title: "Proteção de dados",
        items: [
          "Banco de dados, anexos e novos backups passam a ser protegidos contra visualização casual de arquivos locais.",
          "A primeira abertura cria um backup de recuperação e migra os dados existentes automaticamente.",
          "Backups antigos da versão v0.1.0 continuam podendo ser importados.",
        ],
      },
    ],
  },
  {
    version: "v0.1.0",
    date: "25/07/2026",
    title: "Lançamento inicial",
    sections: [
      {
        title: "Gestão da assistência",
        items: [
          "Cadastro de clientes, usuários, estoque, fornecedores e movimentações.",
          "Ordens de serviço com checklist, peças, serviços, desconto, anexos e linha do tempo.",
          "Geração de PDF da ordem de serviço com área para assinatura do cliente.",
        ],
      },
      {
        title: "Operação",
        items: [
          "Relatórios financeiros em tela, CSV e PDF.",
          "Backup e restauração completos, incluindo anexos.",
          "Tema claro e escuro, funcionamento offline e atualização pelo aplicativo.",
        ],
      },
    ],
  },
];
