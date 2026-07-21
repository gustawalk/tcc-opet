# OpetS Manager

Aplicativo desktop para gestão de assistência técnica, desenvolvido como Trabalho de Conclusão de Curso.

## Sobre o Projeto

O OpetS Manager é um sistema completo para gerenciamento de ordens de serviço, estoque e financeiro de assistências técnicas. Ele substitui planilhas e papelada por uma interface desktop nativa, rápida e offline-first.

O público-alvo são pequenas oficinas e técnicos autônomos que precisam de:
- Controle de clientes, ordens de serviço e estoque
- Geração de PDF das OS com assinatura do cliente
- Relatórios financeiros
- Backup e restauração completos
- Funcionamento 100% offline, sem necessidade de internet

## Stack de Desenvolvimento

### Frontend

| Tecnologia | Versão |
|---|---|
| React | ^19.1 |
| TypeScript | ^5.8 |
| Tailwind CSS | ^4 |
| shadcn/ui | componentes |
| TanStack Query | ^5.90 |
| React Router DOM | ^7.13 |
| Sonner | ^2.0 |
| Lucide React | ^0.575 |
| PDF.js | ^6.1 |

### Backend / Desktop

| Tecnologia | Versão |
|---|---|
| Tauri | 2 |
| Rust | stable (edition 2021) |
| rusqlite | 0.32 (bundled) |
| Serde / Serde JSON | 1 |
| UUID | 1.4 (v4) |
| Chrono | 0.4 |
| headless_chrome | 1.0 (geração de PDF) |
| Tera | 1.20 (template HTML do PDF) |
| zip | 0.6 (backup .osbkp) |
| infer | 0.19 (validação MIME) |
| tauri-plugin-updater | 2 |
| tauri-plugin-dialog | 2 |

### Build e CI/CD

- **Build:** Vite, Docker (AppImage Debian 12)
- **CI:** GitHub Actions — validação em PRs/push, release em tags `v*`
- **Testes:** `cargo test` (92+ testes no backend Rust)

## Recursos

- Clientes, usuários, inventário, fornecedores e movimentações de estoque.
- Ordens de serviço com checklist, peças, serviços, desconto, anexos e timeline.
- PDF de ordem de serviço com assinatura manual do cliente.
- Relatórios financeiros em tela, CSV e PDF.
- Sidebar expansível/colapsável — modo compacto mantém acesso via tooltips.
- Tema claro e escuro, salvo localmente no dispositivo.
- Backup e restauração completos, incluindo anexos, pelo formato `.osbkp`.
- Auto-update via GitHub Releases com verificação de assinatura (`tauri-plugin-updater`).

## Requisitos

- Node.js 22+ e Yarn 1.x.
- Rust estável e dependências de desenvolvimento do Tauri/WebKitGTK 4.1.
- Para gerar AppImage localmente: Docker.

## Desenvolvimento

```bash
yarn install
yarn tauri dev
```

O backend carrega `src-tauri/.env`. Exemplo:

```env
DATABASE_PATH=database.db
SKIP_DB_SEED=true
```

Em builds de desenvolvimento, os dados demonstrativos são inseridos por padrão. Use `SKIP_DB_SEED=true` ou `SKIP_DB_SEED=1` para executar somente migrations. Em produção, dados demonstrativos nunca são inseridos.

## Banco e Backup

- Sem override, o banco e os anexos ficam no diretório de dados do aplicativo do sistema operacional. `DATABASE_PATH` e `DB_PATH` servem para desenvolvimento ou instalação controlada.
- Em Unix, banco, anexos e backups criados pelo app recebem permissões restritas ao usuário atual.
- O reset remove todos os dados e anexos, recria as tabelas e restaura somente registros técnicos padrão.
- O backup `.osbkp` contém um snapshot do banco e todos os anexos.
- A restauração valida manifesto, schema, integridade, chaves estrangeiras e limites de tamanho antes de substituir banco e anexos. Em caso de falha, os dados anteriores são restaurados.

## Validação

```bash
yarn lint
yarn typecheck
yarn build
cd src-tauri && cargo test   # 92+ testes
```

## Auto-update

O aplicativo usa `tauri-plugin-updater` para verificar atualizações. A cada inicialização, ele consulta o manifesto `updater.json` na raiz do repositório. Se houver uma versão mais nova, o usuário pode baixar e instalar diretamente pelo app.

Atualizações são publicadas como GitHub Releases e assinadas com chave ed25519 (minisign). A chave pública está configurada em `tauri.conf.json`.

## Distribuição

```bash
yarn build:deb
yarn build:appimage
```

`build:appimage` usa `Dockerfile.appimage` com Debian 12, evitando a incompatibilidade entre o `strip` do `linuxdeploy` e bibliotecas modernas.

Para compatibilidade com distribuições Linux mais antigas, gere o AppImage em Debian 12 ou Ubuntu 22.04.

### Releases no GitHub

- Pull requests e pushes para `main` executam a validação contínua (lint, TypeScript, build frontend e testes Rust).
- Envie uma tag `vX.Y.Z` para publicar uma GitHub Release. O pipeline de release compila para Windows e Linux (deb + AppImage).
- Execuções manuais do workflow de release geram apenas artifacts para teste, sem publicar uma release.
- O `productName` em `tauri.conf.json` define o nome do programa no menu iniciar, `Program Files` e Add/Remove Programs.

### Aviso sobre assinatura de código

Este aplicativo atualmente não possui assinatura digital porque certificados de code-signing estão financeiramente fora de alcance para este projeto. O código-fonte, o pipeline de CI e os artefatos de release são todos públicos.

**No Windows:** Se o SmartScreen aparecer ao executar o instalador, clique em **More info** → **Run anyway** após verificar que você baixou o arquivo da página oficial de Releases do GitHub.
