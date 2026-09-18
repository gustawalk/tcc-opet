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
    version: "v0.5.2",
    date: "18/09/2026",
    title: "Itens de estoque, fotos e ações rápidas globais",
    sections: [
      {
        title: "Estoque e indicadores",
        items: [
          "O estoque agora permite cadastrar itens genéricos, além de peças e serviços, com controle de estoque escolhido durante o cadastro.",
          "Itens podem receber uma foto, exibida na listagem e ampliada ao clicar, e itens controlados passam a compor os indicadores e métricas de estoque.",
        ],
      },
      {
        title: "Ordens de serviço e clientes",
        items: [
          "Equipamento, descrição do problema e relato ou notas técnicas agora são opcionais nas ordens de serviço.",
          "O telefone continua obrigatório no cadastro de cliente, enquanto e-mail e endereço permanecem opcionais.",
        ],
      },
      {
        title: "Ações rápidas e navegação",
        items: [
          "A busca global ganhou ações para criar item, serviço, peça, cliente e checklist rapidamente.",
          "A criação de itens, serviços e peças, além da visualização de checklists, abre em painéis globais sem interromper a tela atual.",
        ],
      },
    ],
  },
  {
    version: "v0.5.1",
    date: "14/09/2026",
    title: "Visualização de PDFs anexados e cópia LAN padronizada",
    sections: [
      {
        title: "Anexos de ordens de serviço",
        items: [
          "PDFs anexados agora podem ser visualizados diretamente no detalhamento da ordem de serviço, sem exportação prévia.",
          "A visualização mantém o download disponível e apresenta mensagens claras ao carregar ou quando o PDF não puder ser aberto.",
        ],
      },
      {
        title: "Configurações LAN",
        items: [
          "Os botões para copiar endereço do servidor, código de pareamento e impressão digital agora têm o mesmo visual, rótulo e feedback de confirmação.",
        ],
      },
    ],
  },
  {
    version: "v0.5.0",
    date: "13/09/2026",
    title: "Busca global e migrações mais seguras",
    sections: [
      {
        title: "Busca global e navegação",
        items: [
          "Nova busca global para localizar clientes, funcionários, itens de estoque e ordens de serviço em um único atalho.",
          "A busca mantém ações recentes, navegação por teclado e identificação visual por tipo de resultado.",
          "Resultados abrem diretamente os painéis de detalhes correspondentes, sem obrigar o usuário a navegar manualmente até cada tela.",
          "O atalho de busca foi ampliado e ajustado para melhor leitura e acesso rápido.",
        ],
      },
      {
        title: "Ordens de serviço e atendimento",
        items: [
          "O cadastro de ordem agora aceita dados mínimos do cliente quando informações como e-mail e endereço ainda não estiverem disponíveis.",
          "Melhorias no formulário de criação deixam o fluxo mais flexível e evitam sugestões de cliente fora de contexto.",
        ],
      },
      {
        title: "Previsão de conclusão",
        items: [
          "O responsável pode informar uma data prevista de conclusão na ordem de serviço.",
          "Ordens abertas com previsão vencida aparecem em uma área de prioridade no painel, com acesso direto aos detalhes.",
          "A previsão também é exibida no detalhamento e nos PDFs da ordem de serviço.",
        ],
      },
      {
        title: "Migrações e desempenho",
        items: [
          "O banco agora usa migrações versionadas, com validação de integridade e compatibilidade com instalações anteriores.",
          "Antes de migrações em bancos existentes, o aplicativo cria um backup de recuperação.",
          "Consultas do relatório financeiro e a paginação de itens de checklist receberam índices para manter o desempenho em bases maiores.",
        ],
      },
    ],
  },
  {
    version: "v0.4.0",
    date: "11/09/2026",
    title: "Buscas sob demanda e novo ícone",
    sections: [
      {
        title: "Ordens de serviço",
        items: [
          "A busca de clientes passou a carregar resultados conforme o texto é digitado, sem carregar a lista completa ao abrir a tela.",
          "A busca de técnicos também é carregada sob demanda e mantém a seleção atual durante a atualização dos resultados.",
          "As sugestões aparecem somente enquanto o campo correspondente está em foco, evitando resultados fora de contexto.",
        ],
      },
      {
        title: "Identidade visual",
        items: [
          "O aplicativo, o favicon e os instaladores agora usam o novo ícone em todas as plataformas.",
        ],
      },
    ],
  },
  {
    version: "v0.3.3",
    date: "22/08/2026",
    title: "Conexão LAN mais estável e controle de dados aprimorado",
    sections: [
      {
        title: "Conexão LAN",
        items: [
          "Host e cliente agora recuperam conexões temporariamente indisponíveis com novas tentativas automáticas.",
          "O aplicativo não anuncia mais o endereço inválido 0.0.0.0; somente endereços reais da rede local são exibidos.",
          "A tela do host permite copiar o endereço completo, o código de pareamento e a impressão digital de segurança.",
          "O código de pareamento exibe sua validade, e clientes bloqueados durante a indisponibilidade do host voltam a tentar a conexão com uma mensagem clara.",
        ],
      },
      {
        title: "Banco de dados",
        items: [
          "O modo local permite alterar a pasta do banco com confirmação e reinício automático.",
          "O modo cliente mantém o banco controlado pelo host e evita operações locais incompatíveis com a conexão compartilhada.",
          "Falhas de inicialização e perda de conexão agora são apresentadas ao usuário sem encerramento abrupto do aplicativo.",
        ],
      },
      {
        title: "Documentos",
        items: [
          "A geração de ordens de serviço e relatórios financeiros agora usa 10px de espaçamento interno adicional.",
        ],
      },
    ],
  },
  {
    version: "v0.3.2",
    date: "16/08/2026",
    title: "Backup com mensagens claras e histórico expansível",
    sections: [
      {
        title: "Backup",
        items: [
          "Exportar backups e configurar o backup automático agora mostram uma mensagem clara quando a pasta escolhida não permite criar arquivos (ex.: permissão negada em raízes de unidade ou pastas protegidas pelo sistema).",
          "O guia de backup automático e a tela de Configurações passaram a recomendar evitar esses locais e sugerir pastas em nuvem como OneDrive e DropBox.",
          "Removida a nota em texto de que \"arquivos iguais não são duplicados\": o comportamento já era o esperado e não gerava duplicatas.",
        ],
      },
      {
        title: "Interface",
        items: [
          "O histórico de versões em Configurações virou um acordeão: cada versão exibe título, data e versão e expande para revelar as novidades daquela atualização.",
        ],
      },
    ],
  },
  {
    version: "v0.3.1",
    date: "16/08/2026",
    title: "Backup automático reforçado e novo visual",
    sections: [
      {
        title: "Backup automático",
        items: [
          "A configuração e os arquivos temporários passaram a ficar na pasta de dados do aplicativo, com migração automática.",
          "Cada instalação registra localmente os arquivos que criou; a retenção nunca remove backups de outras máquinas ou manuais.",
          "Novos arquivos de backup nunca sobrescrevem um arquivo existente e a configuração corrompida é recuperada da versão anterior.",
          "O agendador espera a data do próximo backup e acorda somente quando necessário; com o recurso desativado, nenhuma verificação de hora em hora roda.",
          "O progresso aparece em um diálogo fixo e as datas do último backup são exibidas de forma relativa (hoje/ontem).",
        ],
      },
      {
        title: "Aparência",
        items: [
          "Tema com as opções Claro, Escuro e Sistema, seguindo automaticamente o sistema operacional.",
          "Tamanho da fonte com as opções Pequena, Padrão e Grande, ajustando textos e interface.",
          "As preferências são salvas somente neste dispositivo.",
        ],
      },
    ],
  },
  {
    version: "v0.3.0",
    date: "12/08/2026",
    title: "Listagens mais rápidas e aplicativo mais leve",
    sections: [
      {
        title: "Desempenho",
        items: [
          "Clientes, funcionários, estoque, modelos e ordens agora usam paginação e busca no banco de dados.",
          "O aplicativo reutiliza a conexão protegida com o banco, reduzindo o tempo de abertura das telas.",
          "Relatórios e indicadores financeiros processam datas e custos com muito mais eficiência.",
        ],
      },
      {
        title: "Interface",
        items: [
          "As listagens oferecem navegação por páginas, tamanhos configuráveis e retorno automático ao início da tabela.",
          "Telas e detalhes são carregados sob demanda para reduzir o tamanho inicial do aplicativo.",
          "A pré-visualização de documentos foi simplificada sem alterar o PDF salvo no computador.",
        ],
      },
      {
        title: "Estabilidade",
        items: [
          "A paginação mantém a página selecionada enquanto os dados são carregados.",
          "O PDF temporário permanece disponível durante a visualização e é descartado somente após o fechamento.",
        ],
      },
    ],
  },
  {
    version: "v0.2.0",
    date: "09/08/2026",
    title: "Precisão financeira e estoque com alertas",
    sections: [
      {
        title: "Precisão e indicadores",
        items: [
          "Preços, custos, descontos e totais agora são calculados com precisão, sem diferenças de arredondamento.",
          "Painel e Relatório avançado mostram \"Lucro bruto estimado\" com a mesma base de cálculo em tela, CSV e PDF.",
        ],
      },
      {
        title: "Estoque e painel",
        items: [
          "Alertas para peças em estoque baixo ou esgotadas, direto no painel.",
          "Duplique itens do catálogo em um clique, sem zerar o saldo por engano.",
          "Consulte os detalhes de um modelo de checklist ao selecioná-lo.",
          "Crie o colaborador na hora da ordem de serviço e digite valores no formato brasileiro.",
        ],
      },
      {
        title: "Backup, segurança e atualizações",
        items: [
          "Mais proteção ao abrir o aplicativo: o banco não é aberto por duas instâncias ao mesmo tempo.",
          "Verificação de atualizações mais estável quando o computador está offline.",
          "Você continua podendo restaurar backups criados nas versões 0.1.x.",
        ],
      },
    ],
  },
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
