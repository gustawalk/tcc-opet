# Banco SQLite em pasta compartilhada na LAN (multi-escrita)

## 1. Metadados

- **Título**: Banco SQLite em pasta compartilhada na LAN (multi-escrita)
- **Origem**: Pedido de melhoria — texto colado no prompt (sem ticket de tracker). Slug: `database-compartilhada-em-pasta-lan`.
- **Data de análise**: 2026-08-18
- **Status**: Aprovado para planejamento — aguardando respostas do PO/PM nas perguntas da seção 4 e início de implementação
- **Profundidade da análise**: Profunda (área central de armazenamento; mais de uma interpretação técnica válida; pedido com cenário experimental em branch de testes `test/sqlite-wal`)

> Nota de investigação: o branch `test/sqlite-wal` está vazio (0 commits além de `main`). O nome indica a intenção de habilitar WAL para acesso multi-processo, mas nenhum código existe ainda (busca por `journal_mode`/`busy_timeout`/`WAL` não retorna ocorrências em `src-tauri/src/`).

## 2. Descrição e problema

Hoje o local do banco SQLite é fixo (`app_data_dir/database.db`), determinado no startup e imutável durante a execução. O pedido é: o usuário escolhe uma pasta compartilhada na LAN e outro(s) computador(es) apontando para o mesmo local conseguem **ler e escrever no mesmo arquivo**. Estamos em branch de testes — alterações estruturais são aceitas se forem necessárias.

Ao estudar o código, o problema real não é só "trocar o path"; há quatro barreiras de concorrência no mecanismo de inicialização:

1. **Trava de instância exclusiva**: `open_storage_instance_lock` cria `<db>.lock` e toma lock exclusivo de SO (`fs2::FileExt::try_lock_exclusive`) — o segundo processo que abrir o mesmo arquivo falha com `"Another application instance is already using this storage"` (`src-tauri/src/database.rs:124-140`). Com o lock, não existe acesso concorrente de forma alguma.
2. **Sem WAL e sem `busy_timeout`**: a conexão única é aberta sem `journal_mode`, logo opera em rollback journal (lock de banco inteiro); dois writers morrem com `SQLITE_BUSY` em vez de esperar (`database.rs:290` `open_encrypted_database`, `database.rs:865` `get_db`). Busca no repositório por `busy_timeout`/`journal_mode` não encontra nenhum uso.
3. **Permissões restritivas**: `ensure_private_dir` aplica `0700` na pasta e `secure_private_file` aplica `0600` no arquivo (`database.rs:336-349`) — no lado Unix/lector por outra máquina o acesso é negado.
4. **Migrations sem guard de concorrência**: `run_schema_migrations` usa `CREATE ... IF NOT EXISTS` (idempotente), mas `add_column_if_missing` executa `ALTER TABLE ... ADD COLUMN` (`database.rs:735-752`). Dois processos subindo o banco pela primeira vez ao mesmo tempo podem tentar o mesmo `ALTER` concorrentemente, com uma das duas operações abortando por lock/`SQLITE_BUSY` e quebrando o startup.

Há uma quinta barreira — estrutural e de longa duração — que independe do código do app:

5. **SQLite sobre filesystem de rede não tem garantia de locking**: SQLite documenta que seus locks de arquivo não funcionam de forma confiável em NFS/SMB/Samba. WAL mitiga, mas o `-shm` de memória compartilhada depende do filesystem. A combinação "multi-escrita simultânea em share de rede" é experimental e precisa de validação empírica real — vira critério de aceite do MVP, não algo assumido como resolvido.

## 3. Nível de definição do escopo

**Bem definido.** As decisões de escopo foram tomadas em conversa com o solicitante:

- Abordagem: **arquivo compartilhado direto** (recusada a opção de servidor embutido para já).
- Concorrência: **todos leem e escrevem** (multi-writer), com exigência explícita de robustez contra corrupção.
- Criptografia: **mesma build/instalador em todas as máquinas** → mesma chave SQLCipher embutida (`src-tauri/build.rs`, `src-tauri/src/encryption.rs:5-13`) — não é bloqueador entre PCs da mesma release.

Ainda em aberto (seção 4): número de máquinas, sistema da rede (SMB/Samba), e administração dos backups automáticos.

## 4. Perguntas para o PO/PM

Não há dúvida de escopo de feature, mas há de ambiente/operação:

1. **Quantas máquinas simultâneas e volume de escrita real?** Determinante para o nível de confiança no modo LAN. (3+ máquinas escrevendo intensamente é o cenário mais arriscado sobre share.)
2. **Qual sistema de rede será a pasta compartilhada — Windows/SMB, Samba no Linux ou outra?** Define a matriz de testes obrigatória do MVP (critério de aceite).
3. **Quem administra os backups automáticos no modo LAN?** Sugestão técnica: apenas uma máquina mantém o backup automático ativo, para evitar N cópias do mesmo banco no mesmo local.
4. **Troca de pasta exige reinício do app** — aceitável para o MVP?

## 5. Viabilidade técnica

**Viável, com ressalvas.** No código:

- O path já é resolvível por variável de ambiente (`DATABASE_PATH`/`DB_PATH`) e por config persistida; a resolução final prioriza a **escolha explícita do usuário na UI** (config local), depois o ambiente, e por fim o default `app_data_dir/database.db` (`database.rs` `get_database_path`). A escolha do usuário vence o ambiente para que a feature seja testável em dev mesmo com o `.env` (`DATABASE_PATH=database.db`) presente.
- Já existe precedente de seletor de pasta e de config persistida no app: `select_automatic_backup_directory` usa `blocking_pick_folder` (`src-tauri/src/commands/settings_commands.rs:252-268`) e o backup automático persiste `database.automatic-backup.json` em `app_data_dir()/automatic-backup` (`src-tauri/src/automatic_backup.rs:208-249`).
- O `get_system_info` já expõe `database_path` à UI (`settings_commands.rs:132-145`), exibido em `src/views/Settings.tsx:697` — base para a nova seção "Local do banco".
- A escrita de dados já é transacional e a sequência de OS usa upsert atômico (`INSERT ... ON CONFLICT ... DO UPDATE`, `src-tauri/src/repositories/service_order_repo.rs:497-518`) — compatível com escritas concorrentes.
- Anexos são criptografados com chave derivada da mesma master (`encryption.rs:25`, `attachment_service.rs:50-75`) → mesmas builds = mesmo conteúdo legível; migram junto com o banco no share.
- **Ressalva (não confirmável sem teste real)**: robustez do locking do SQLite sobre share SMB/Samba. Não é possível afirmar em análise estática que a combinação será estável; por isso o teste com o share real é critério de aceite do MVP (seção 11).

Outras propriedades confirmadas no código:

- `database.encryption.json` (metadata de formato/chave) é co-localizada (`database.rs:240-281`) e deve viajar com o banco.
- `database.pdf-previews` é cache transitório de previews de PDF (`pdf_service.rs:389-397`) — deve ficar **local** no modo LAN, para não pagar I/O de rede com dado descartável.
- `database.attachments` é dado de negócio e precisa ser compartilhado junto.
- Restore/reset reescrevem/removem o arquivo de banco enquanto outros processos podem estar conectados (`settings_commands.rs:89-129` reset, `:200-208` restore) — operações destrutivas que precisam ser bloqueadas no modo LAN.

## 6. Projetos/repositórios impactados

Repositório único: **tcc-opet** (app Tauri 2 + Rust + React).

| Área | Arquivos | Tipo de mudança |
|---|---|---|
| Núcleo de armazenamento | `src-tauri/src/database.rs`, `src-tauri/src/encryption.rs` | Alta |
| Comandos/UI Settings | `src-tauri/src/commands/settings_commands.rs`, `src-tauri/src/lib.rs` (registro de comando) | Alta |
| Frontend | `src/views/Settings.tsx`, `src/lib/types.ts` (tipos do novo command/config) | Alta |
| Backup/restore | `src-tauri/src/automatic_backup.rs`, `backup_service.rs` | Média |
| Anexos/previews | `src-tauri/src/attachment_service.rs`, `pdf_service.rs` (previews locais no modo LAN) | Média |
| Docs/tests | `src-tauri/tests/`, `docs/` | Baixa |

Novos arquivos previstos: `app_config.json` (path persistido + toggle LAN, no `app_data_dir`), seção/local da spec sob `docs/specs/database-compartilhada-lan/`.

Backend testes: `cargo test --lib` (154 passando, 2 ignorados); frontend: `yarn typecheck`, `yarn lint`, `yarn test` (26 passando) — a rodar antes de fechar o trabalho.

## 7. Dificuldade

**Média-alta.** Justificativa qualitativa:

- A mudança toca o núcleo de inicialização (`database.rs`), área central; a remoção da trava de instância e a condicional por modo (single-user vs LAN) exigem cuidado para não enfraquecer o modo padrão.
- O guard de migrations concorrentes é delicado (envolver DDL em transação com `BEGIN IMMEDIATE` e mover `add_column_if_missing` para dentro dela), com risco de quebrar startups existentes se mal feito.
- A validação empírica (2–3 máquinas, share real, cenários de interrupção) é parte obrigatória do trabalho e não pode ser eliminada.
- Não é "alta" porque não há mudança de schema de dados, de formato de armazenamento nem reescrita de comandos IPC; a superfície de mudança de código é relativamente pequena se mantida em feature flags de modo.

## 8. Impacto em outras features

- **Backup automático**: cada máquina geraria backups `.osbkp` próprios do mesmo banco no mesmo local — duplicação. Mitigação: recomendar no Setting que apenas uma máquina mantenha o backup ativo, ou restringir por config no modo LAN.
- **Restore/reset**: ficam **bloqueados no modo LAN** (reescrever o arquivo enquanto outro processo o tem aberto corrompe estado). Impacta a UX de Settings ("Zerar dados"/"Restaurar") quando o modo compartilhado estiver ativo — o trabalho precisa avisar explicitamente.
- **Anexos**: passam a viver no share (rede mais lenta para upload/download); converte-se em mudança de onde o storage físico dos anexos reside.
- **Previews de PDF**: movidos para cache local no modo LAN; muda a semântica atual de `preview_working_dir` (`pdf_service.rs:389`) que hoje deriva o diretório do path do banco.
- **Atualizações/migrações**: o campo `databasePath` no `SystemInfo` deixa de ser somente-leitura e passa a ser gerenciável; o env `DATABASE_PATH` continua como precedência máxima.

## 9. Abordagens alternativas

Cenário avaliado com o solicitante; a abordagem A foi a escolhida, B e C ficam registradas como retorno.

### A. Arquivo compartilhado direto com WAL (escolhida)
- **Prós**: mínima mudança; mantém offline-first e o backup/restore local; reusa seletor de pasta existente.
- **Contras**: locking do SQLite sobre share de rede não tem garantia; risco residual de `SQLITE_BUSY`/corrupção em multi-escrita intensa; depende de teste empírico real.

### B. Modo servidor embutido (retorno se A falhar)
Um processo (o app do "dono" do banco) expõe as mesmas operações por um mini servidor HTTP/RPC local; os outros PCs apontam o app para `http://<ip>:porta`. O servidor é o único dono da conexão SQLite.
- **Prós**: correto por design; SQLite longe da rede; suporta N máquinas de forma robusta; reusa comandos existentes.
- **Contras**: maior esforço; precisa decidir onde o servidor fica ligado; autenticação na rede; latência extra (IPC→HTTP).

### C. Migrar para banco multi-usuário (rqlite/Postgres/etc.) — descartada
- **Prós**: robustez real; escala.
- **Contras**: quebraria offline-first, backup `.osbkp`, restauração e todo o stack SQLCipher atual — fora do propósito do app desktop. Não recomendado nesta fase.

## 10. Riscos e mitigações

| Risco | Probabilidade | Mitigação |
|---|---|---|
| Corrupção do banco por multi-escrita em share (locking não garantido) | Alto | WAL + `busy_timeout` alto + `synchronous=NORMAL`; teste real em SMB/Samba como **critério de aceite**; aviso experimental na UI; backups automáticos como rede de segurança |
| Dois processos migram o mesmo schema ao mesmo tempo no primeiro boot | Médio/Alto | **Guard de migrations no MVP**: `BEGIN IMMEDIATE` + claim atômico envolvendo `run_schema_migrations` e `ensure_core_defaults`, `ALTER TABLE` fechado dentro da transação |
| Restore/reset de uma máquina sobrescreve o arquivo em uso por outras | Médio | Bloquear restore/reset no modo LAN com aviso |
| `SQLITE_BUSY` de thread longa em operação pesada (relatórios) | Médio | `busy_timeout=30000`; documentar recomendação de operação fora de pico |
| Dependência do `-shm` em filesystem de rede | Médio | Manter anexos/previews com política clara (attachments no share, previews locais); registar que o teste real decide |
| N máquinas criando backups automáticos duplicados | Baixo | Nova config/recomendação de "só uma máquina administra backup" |
| Perda de integridade após queda de conexão/unmount no meio de transação | Médio | Cenário no roteiro de validação obrigatória; recuperação (`-wal` órfão) testada; `integrity_check` vai para pós-MVP |

## 11. Sugestão de fasamento

**Dá para MVP? Sim**, com o escopo fechado abaixo e a validação obrigatória como critério de aceite.

### MVP
1. Config local persistida (`app_config.json` no `app_data_dir`) com path do banco + toggle "Compartilhamento LAN"; seletor de pasta no Settings (reusando `blocking_pick_folder`); reinício do app ao trocar o path. Precedência de resolução: config local (escolha do usuário) → env `DATABASE_PATH` → `app_data_dir/database.db`.
2. Remoção/condicionamento do instance lock em modo LAN (`database.rs:124-147`).
3. `PRAGMA journal_mode=WAL` + `busy_timeout=30000` + `synchronous=NORMAL` no `open_encrypted_database` (`database.rs:290`) em modo LAN.
4. Permissões relaxadas em modo LAN (`0700/0600` → acesso ao grupo/dono do share).
5. **Guard de migrations**: `run_migrations` roda o schema + defaults dentro de uma única transação `BEGIN IMMEDIATE` (`rusqlite::Transaction::new_unchecked(conn, TransactionBehavior::Immediate)`), que serializa o primeiro boot entre processos — o segundo processo aguarda via `busy_timeout` e reexecuta (idempotente). `add_column_if_missing` e `migrate_integer_money` passam a executar dentro dessa transação. Obrigatório já no MVP.
6. Bloqueio de restore/reset com aviso em modo LAN.
7. Aviso "experimental/rede não confiável" na UI + previews PDF mantidos em cache local.

### Validação obrigatória (critério de aceite do MVP)
Roteiro a executar antes de declarar o MVP pronto, com share real (SMB ou Samba conforme resposta da seção 4):
- 2–3 máquinas simultâneas no mesmo local;
- leitura + escrita concorrente (criação de OS, mutações de estoque, anexos);
- restart de uma máquina durante escrita de outra;
- queda de conexão/unmount do share no meio de transação;
- criação concorrente de registros (sequência de OS continua única — `service_order_repo.rs:497`);
- migrations em primeiro startup simultâneo nas duas máquinas;
- recuperação após interrupção (`-wal` órfão, `integrity_check`, `quick_check`).

### Pós-MVP
- `PRAGMA quick_check` no startup em modo LAN;
- documentação de disaster recovery para ambiente de rede (`docs/guides/disaster-recovery.md`);
- otimizações de checkpoint (`wal_checkpoint`) e retenção do `-wal`;
- refinamentos de backup (ex.: definir máquina administradora de backup automático no modo LAN);
- caso a validação real falhe: reavaliar abordagem B (servidor embutido).