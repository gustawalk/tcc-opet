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
