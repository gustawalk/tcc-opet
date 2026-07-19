# OpetS Manager

Aplicativo desktop para gestão de assistência técnica, desenvolvido com Tauri, React, TypeScript e SQLite.

## Recursos

- Clientes, usuários, inventário, fornecedores e movimentações de estoque.
- Ordens de serviço com checklist, peças, serviços, desconto, anexos e timeline.
- PDF de ordem de serviço com assinatura manual do cliente.
- Relatórios financeiros em tela, CSV e PDF.
- Backup e restauração completos, incluindo anexos, pelo formato `.osbkp`.
- Tema claro e escuro, salvo localmente no dispositivo.

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
cd src-tauri && cargo test
```

## Distribuição

```bash
yarn build:deb
yarn build:appimage
```

`build:appimage` usa `Dockerfile.appimage` com Debian 12, evitando a incompatibilidade entre o `strip` do `linuxdeploy` e bibliotecas modernas que usam a seção `.relr.dyn` em hosts rolling release.

Para compatibilidade com distribuições Linux mais antigas, gere o AppImage em Debian 12 ou Ubuntu 22.04. Builds gerados em sistemas mais novos podem exigir uma versão de glibc indisponível em distribuições antigas.

### Releases no GitHub

- Pull requests e pushes para `main` executam a validação contínua (lint, TypeScript, build frontend e testes Rust).
- Envie uma tag `vX.Y.Z` com a mesma versão de `package.json`, `Cargo.toml` e `src-tauri/tauri.conf.json` para publicar uma GitHub Release.
- A release contém o instalador NSIS (`.exe`) e MSI (`.msi`) para Windows, além de `.deb` e AppImage para Linux.
- Execuções manuais do workflow de release geram apenas artifacts para teste, sem publicar uma release.
- Os instaladores Windows não são assinados nesta etapa e podem exibir aviso do SmartScreen.
