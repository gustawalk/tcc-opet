# Fluxo de criação de uma tag de release

Este guia descreve o processo para publicar uma nova versão do OpetS.
Um merge na `main` **não** gera artefatos nem release: quem produz os
instaladores (Windows NSIS+MSI, Linux DEB e AppImage) e a GitHub Release
é o workflow `.github/workflows/release.yml`, que dispara ao **publicar
uma tag `v*`** (ou em `workflow_dispatch`).

Execute os passos na ordem, de preferência na própria máquina onde você
já valida o código.

---

## 1. Pré-checagem

Confira as condições antes de tocar em qualquer arquivo.

```bash
# Estado do repositório
git status --short                 # working tree limpo
git fetch origin
git branch --show-current          # estar em main (ou no branch de release)
git rev-parse main origin/main     # principal atualizada com a remota

# Versão atual consistente em todos os lugares
node -p "require('./package.json').version"
node -p "require('./src-tauri/tauri.conf.json').version"
sed -n 's/^version = "\(.*\)"$/\1/p' src-tauri/Cargo.toml | head -1
# Cargo.lock: conferir o bloco "name = \"tcc-opet\"" -> version
```

Critérios:

- **CI verde**: os checks do último pull request / push na `main` devem
  ter passado (`gh pr checks` / `gh run list`). O workflow `ci.yml`
  roda em qualquer `pull_request`/`push`; o `release.yml` roda por tag.
- **Consistência de versão**: `package.json`, `tauri.conf.json`,
  `Cargo.toml` e o `tcc-opet` em `Cargo.lock` devem ter **exatamente a
  mesma versão**.
- **A tag desejada ainda não existe** (ou, se existe, você sabe que não
  foi publicada). **_Nunca_ mova uma tag já publicada**: se algo ficou
  errado após o release, corrija em um novo patch (ex.: `v0.3.2`).
- **Segredos do release**: o build assinado exige os secrets
  `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` e
  `OPETS_DATA_KEY_V1` configurados no repositório. Sem eles, o build de
  artefatos falha.
- **Nota de release**: existir `docs/releases/v<N>.<N>.<N>.md` com o
  conteúdo da nova versão (criada no passo 4).

---

## 2. Validação completa (testes e builds)

Com a versão ainda na atual (ou já alterada — sempre valida com o código
do commit de release):

```bash
# Frontend
yarn typecheck
yarn lint                 # eslint src --max-warnings 0
yarn test
yarn build

# Backend (depende das libs nativas Linux listadas no CI)
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --lib -- --test-threads=1

# Dependências travadas (opcional, é o que o CI faz)
yarn install --frozen-lockfile
```

Tudo precisa terminar sem erros. Isto inclui o clippy com `--all-targets
--all-features`, que cobre testes e é o modo usado no CI (um clippy só
com `-- -D warnings` pode passar local e **falhar no GitHub**).

---

## 3. Alterar as versões (onde é necessário)

Incremente a versão nos **quatro** lugares para `X.Y.Z` (aumente o
patch, ou o minor/major conforme o tamanho da mudança).

1. `package.json` — `"version": "X.Y.Z"`
2. `src-tauri/Cargo.toml` — `version = "X.Y.Z"` no `[package]`
3. `src-tauri/Cargo.lock` — bloco `name = "tcc-opet"` → `version = "X.Y.Z"`
   (não altere outras crates que por acaso estejam na mesma versão)
4. `src-tauri/tauri.conf.json` — `"version": "X.Y.Z"`

Confira logo após:

```bash
node -p "require('./package.json').version"
node -p "require('./src-tauri/tauri.conf.json').version"
sed -n '3p' src-tauri/Cargo.toml
sed -n 's/^version = "\(.*\)"$/\1/p' src-tauri/Cargo.lock | grep -A0 -B0 "0.3.1" # conferir o bloco tcc-opet
```

Todas devem imprimir a mesma versão.

---

## 4. Notas de release (patching notes)

- **Criar** `docs/releases/vX.Y.Z.md` no mesmo formato das versões
  anteriores (`OpetS vX.Y.Z`, seções de tópicos com `-`). Esta é a nota
  usada na GitHub Release (`release.yml` lê
  `docs/releases/${GITHUB_REF_NAME}.md`).
- **Atualizar o histórico offline** em `src/lib/release-notes.ts`,
  adicionando a nova versão no topo do array com `date` no formato
  `dd/mm/aaaa`. É o que a tela de Configurações mostra desconectado.
- **Não apague** os arquivos/entradas das versões anteriores.

---

## 5. Commit e integração na `main`

Crie um commit coeso com versão + notas, integre na `main` e deixe o
principal local apontando para o commit que será a tag.

```bash
git add package.json src-tauri/Cargo.toml src-tauri/Cargo.lock \
        src-tauri/tauri.conf.json docs/releases/vX.Y.Z.md src/lib/release-notes.ts
git commit -m "chore(release): prepare vX.Y.Z"
# se não estiver em main: push, PR para main, merge e depois:
git checkout main && git pull
```

A tag deve apontar para o commit final em `main` que contém o bump.

---

## 6. Publicar a tag (gera os artefatos)

```bash
git tag -a vX.Y.Z -m "OpetS vX.Y.Z"
git push origin vX.Y.Z
```

O `release.yml` dispara e executa em sequência:

1. `verify` — confere as três versões iguais e que a tag é `v${version}`,
   roda lint/typecheck/build/testes Rust.
2. `build-windows`, `build-linux-deb`, `build-linux-appimage` — geram os
   instaladores.
3. `publish` — baixa os artefatos, gera o `updater.json`, cria/atualiza a
   GitHub Release com `docs/releases/vX.Y.Z.md` e remove os artefatos
   temporários do workflow.

---

## 7. Confirmação pós-release

```bash
gh run list --workflow=release.yml        # todos os jobs concluídos
gh release view vX.Y.Z                    # assets presentes, notas corretas
gh release list --limit 5                 # a nova versão é a "latest"
```

Caminho de verificação manual: abra o release no GitHub, confira que os
instaladores (`.exe`, `.msi`, `.deb`, `.AppImage`) e o `updater.json`
estão anexados, e que o "Verificar atualizações" do app enxerga a nova
versão.

---

## Falhas comuns

- **Tag já existe**: escolha outro número de versão; não force a tag.
- **CI clippy falha só no GitHub**: rode
  `cargo clippy --all-targets --all-features -- -D warnings` localmente.
- **Versões fora de sincronia**: o `verify` falha com
  `A tag X deve corresponder a vY.` — alinhe os quatro arquivos.
- **Nota de release ausente**: `publish` falha ao ler
  `docs/releases/vX.Y.Z.md`. Crie o arquivo antes da tag.
- **Build assinado falhando**: confira os secrets
  `TAURI_SIGNING_PRIVATE_KEY*` e `OPETS_DATA_KEY_V1` no repositório.