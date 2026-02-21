# Diretrizes do Projeto: Sistema de Ordens de Serviço (TCC)

Este arquivo serve como o contexto mestre para o desenvolvimento do projeto. Sempre que iniciar uma tarefa, consulte estas diretrizes para manter a consistência.

## 1. Stack Tecnológica
- **Frontend:** React + TypeScript + Vite.
- **Estilização:** Tailwind CSS + shadcn/ui.
- **Gerenciamento de Estado:** TanStack Query (React Query).
- **Ícones:** Lucide React.
- **Assinatura Digital:** Canvas API.
- **Fotos:** Upload local via Tauri Path API.
- **Backend:** Tauri (Rust) com SQLite (`sqlx` ou `rusqlite`).
- **Comunicação:** Tauri IPC (`invoke`).

## 2. Convenções de Código (Frontend)
- **TanStack Query:** Usar `useQuery` para buscar dados e `useMutation` para salvar/deletar. Todas as chamadas ao Tauri devem passar pelo Query para cache e loading states.
- **Soft Deletes:** Nas consultas de Clientes e Estoque, o frontend deve enviar apenas os itens onde `deleted_at IS NULL`.
- **Dashboard Financeiro:** Exibir Faturamento Bruto (Soma das OS) e Lucro Líquido (Faturamento - Custo de Insumos).
- **Onboarding:** Detectar se a tabela `settings` está vazia e forçar o setup inicial.
- **UI:** Componentes do **shadcn/ui**.

## 3. Fluxo de Dados e Banco (Rust)
- **Migrations:** Na inicialização do Tauri (`main.rs`), executar scripts de migração do SQLite.
- **Transações:** A baixa no estoque e a conclusão da OS devem ocorrer dentro de uma `Transaction` SQL. Se um passo falhar, nada deve ser salvo.
- **Integridade:** Salvar o preço de custo no momento em que a peça é adicionada à OS, garantindo que o lucro histórico seja preservado mesmo que o preço do produto mude no estoque depois.

## 4. Prompts de Referência para Novas Funcionalidades

### Para Criar a View de Estoque com React Query:
> "Gemini, crie a View de Estoque usando TanStack Query para gerenciar o estado. Implemente o CRUD com Soft Deletes (marcar `deleted_at`) e exiba alertas visuais para itens com quantidade abaixo do mínimo."

### Para Implementar o Cálculo de Lucro no Backend:
> "Gemini, crie o comando Rust `get_financial_summary`. Ele deve calcular a soma de todas as OS concluídas e subtrair a soma dos custos de peças registradas em cada OS, retornando Faturamento, Custo e Lucro."

### Para Implementar Migrations no Rust:
> "Gemini, crie o sistema de migração no backend Tauri. Use o `PRAGMA user_version` para controlar a versão do banco e aplicar os scripts SQL iniciais de criação de tabelas e adições futuras de colunas."

## 5. Arquitetura de Pastas Sugerida
- `src/queries`: Hooks customizados do TanStack Query (ex: `useCustomers.ts`).
- `src/components`: Componentes reutilizáveis (FinancialCard, InventoryTable).
- `src/views`: Páginas principais (Dashboard, Inventory, OSDetail).
- `src/lib`: Utilitários e formatadores.
