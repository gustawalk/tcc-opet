# Banco SQLite em pasta compartilhada na LAN — Notas de implementação (MVP)

Branch: `test/sqlite-wal`. Nenhum artefato externo: tudo dentro do repositório.

## O que foi implementado

### Config local persistida + seletor de pasta (UI)
- `StorageConfig` (`database.rs`): `{ databasePath?: string, lanShared: boolean }`, persistida como `app_config.json` no `app_data_dir`, fora do banco (o arquivo de dados pode morar num share). Escrita atômica (temp + rename), `sync_all` não exigido no MVP.
- Resolução do path (`database.rs` `get_database_path`): config local (escolha do usuário) → env `DATABASE_PATH`/`DB_PATH` → `app_data_dir/database.db`. A escolha do usuário vence o ambiente para a feature funcionar em dev mesmo com `.env`.
- Comandos novos (`settings_commands.rs`): `get_storage_config`, `update_storage_config` (aceita pasta → gera `<pasta>/database.db`, exige path absoluto), `select_database_directory` (reusa `blocking_pick_folder`). Registrados em `lib.rs`.

### Modo LAN (gate central: `database.rs::lan_shared_mode()`)
- Toggle `lanShared` no Settings (`Settings.tsx`, seção "Compartilhar na rede (LAN)") com aviso "experimental" quando ativo. Ao salvar com a rede ativada ou desativada, o app **reinicia automaticamente** (`relaunch()` do `plugin-process` + fallback de toast em caso de falha); a seção fica sempre visível (sem `details`/collapse).
- **Caminho com LAN ativa**: fixado pela pasta compartilhada — o seletor de pasta e o botão "Restabelecer" somem e um badge "Fixado pela rede" aparece; para trocar, desative a rede, mude a pasta e salve (reinicia). Com LAN desativada o usuário pode escolher pasta ("Selecionar pasta"), desfazer a escolha ("Restabelecer") e salvar.
- Salvamento que só alterna o toggle preserva o `databasePath` já configurado (envia `storageConfig.databasePath ?? null`); só muda o path quando o usuário escolheu outra pasta.
- **Instance lock pulado**: sem `<db>.lock` / `fs2` exclusivo quando LAN ativo (`init_db`).
- **WAL + controles de concorrência**: `PRAGMA journal_mode=WAL`, `busy_timeout=30000`, `synchronous=NORMAL` no `open_encrypted_database` (journal_mode via `query_row`, pois retorna linha). O mesmo par WAL/NORMAL vale para **todos** os filesystems (local e redes) — é o modo validado inclusive sobre share SMB1+unix (ver rede abaixo).
- **Guard de transporte no startup**: `probe_network_share_locking` roda **antes** de qualquer conexão ativar o WAL, com uma conexão SQLCipher própria, `busy_timeout=0` e um `CREATE + DROP` de tabela `__opets_share_lock_probe__`. Em transporte com locking quebrado (Linux CIFS + SMB2/3) falha em ~50 ms com erro legível em PT ("Esta pasta compartilhada não suporta o bloqueio de arquivos que o SQLite exige…"), em vez de um `database is locked` genérico após 30 s.
- **Permissões relaxadas**: `ensure_private_dir`/`secure_private_file` não aplicam `0700/0600` no modo LAN.

### Guard de migrations (requisito de inicialização)
- `run_migrations` roda schema + defaults dentro de `BEGIN IMMEDIATE` (`Transaction::new_unchecked(conn, Immediate)`); o segundo processo serializa via `busy_timeout` e reexecuta (idempotente).
- `migrate_integer_money` deixou de abrir transação própria (passa a rodar dentro da transação do caller), evitando transação aninhada.

### Bloqueios de segurança
- `reset_database` e `restore_backup` retornam erro no modo LAN (reescrever o arquivo com outros processos conectados corromperia estado).

### Previews de PDF
- `preview_working_dir` usa `app_data_dir()/pdf-previews` (cache local) quando LAN ativo; o share só carrega `database.db` + `-wal`/`-shm`, `.encryption.json` e `.attachments`.

## Testes

- Rust: `cargo test --lib` → **187 passando** (+6 novos para esta feature):
  - `database.rs`: `user_storage_config_precedes_every_other_resolution_source`, `default_storage_config_resolves_to_the_application_data_directory`, `missing_storage_config_defaults_to_single_user_storage`, `storage_config_round_trips_through_the_application_data_directory` (config e precedência), `shared_mode_is_off_by_default`.
  - `database.rs` (comportamento do modo LAN): `lan_mode_enables_wal_busy_timeout_and_normal_synchronous` — `open_encrypted_database_with_mode(true)` aplica `WAL`/`busy_timeout=30000`/`synchronous=NORMAL`, `false` permanece `delete`; `lan_mode_keeps_shared_storage_permissions_open` (unix) — permissões não restritas no modo LAN, `0600` mantido no single-user; `concurrent_migrations_on_the_same_database_are_serialized_and_idempotent` — 2 conexões rodando `run_migrations` no mesmo arquivo (com `busy_timeout`) terminam ambas e o schema fica íntegro (estável, validado 3x).
  - `settings_commands.rs`: `storage_config_commands_round_trip_and_reject_relative_paths` — `update_storage_config`/`get_storage_config` persistem e releem, rejeitam caminho relativo.
  - `tauri_ipc_tests.rs`: `storage_config_commands_preserve_the_ipc_contract_and_reject_relative_folders` — round-trip via IPC serializando `databasePath`/`lanShared` em camelCase e erro bilingue para pasta relativa; usa os comandos extras do macro (sem `select_database_directory`, que exige `AppHandle` e não compila com `MockRuntime`).
- `cargo clippy` e `cargo fmt --check` limpos.
- Frontend: `yarn typecheck`, `yarn lint --max-warnings 0`, `yarn test` → **68 passando** (seção LAN "Compartilhar na rede (LAN)": aviso experimental âmbar ao ligar o checkbox, caminho fixado pela rede com LAN ativa, "Restabelecer"/salvar com toggle-only mantendo o path, e `relaunch` automático mockado ao salvar).

Cobertura ainda não automatizada (depende de ambiente real de rede — critério de aceite do MVP):
- Dois processos reais do app em share SMB/Samba (a suíte roda no mesmo processo/filesystem local).
- Guards de `reset_database`/`restore_backup` no modo LAN (exigem o flag global ligado, que é `OnceCell`; coberto apenas por inspeção e lógica idêntica nos testes de permissão).
- Falha `quire_storage_instance_lock` quando uma segunda máquina sem LAN tenta abrir o mesmo arquivo (exige dois processos).
- A seção de UI não cobre o fluxo de "Selecionar pasta" (abre diálogo nativo do desktop).

## Prova de carga (multiusuário, `storage_concurrency_stress.rs`)

Suíte de estresse com os **mesmos pragmas LAN** do app (WAL, `busy_timeout=30000`, `synchronous=NORMAL`): N conexões em threads rodando o `writer_batch` — transação IMMEDIATE que espelha uma OS real (customer + item + estoque + sequência atômica `ON CONFLICT ... RETURNING` + OS + eventos + peças) — enquanto 8 leitores varrem a base. Testes marcados `#[ignore]`; rodar com `cargo test --lib -- --ignored storage_concurrency --nocapture`.

Matriz de escritas concorrentes (2.400 commits por rodada, mesma base, sequência continua sem colisão):

| workers | writes | busy | lock | other | rows | wall_s | integrity | seq OK |
|--------:|-------:|-----:|-----:|------:|-----:|-------:|-----------|:------:|
| 2  | 2400 | 0 | 0 | 0 | 2400 | 2.37 | ok | ✓ |
| 4  | 2400 | 0 | 0 | 0 | 2400 | 4.04 | ok | ✓ |
| 8  | 2400 | 0 | 0 | 0 | 2400 | 5.17 | ok | ✓ |
| 16 | 2400 | 0 | 0 | 0 | 2400 | 6.16 | ok | ✓ |
| 32 | 2400 | 0 | 0 | 0 | 2400 | 6.99 | ok | ✓ |
| 64 | 2368 | 0 | 0 | 0 | 2368 | 8.37 | ok | ✓ |

Leituras paralelas não sofrem erro (`rdErr=0`) e `integrity_check` permanece `ok` em todos os níveis; o `busy_timeout` de 30s absorve o backoff em 64 workers (32 commits esperaram, nenhum falhou). Caminho degradado é coberto por `commit_backlog_exceeding_busy_timeout_fails_cleanly_without_corruption`: 64 escritores × 60 commits com `busy_timeout=400ms` → 432 BUSY limpos, zero erros inesperados, integridade ok e delta da sequência íntegro.

Validação multi-processo real (2 clients release + 16 threads simultâneos no mesmo `database.db`): 2.400/2.400 commits ok, `other=0`, integridade ok em 3.20s — prova de escrita concorrente contra o banco vivo dos clients (usa `STRESS_DB_PATH` e cria snapshot `VACUUM INTO` como safe-restore antes de escrever).

## Resultado da validação em filesystem de rede (SMB/Samba) — CRÍTICO

A justa validação foi feita com **kernel CIFS real → smbd real** (client container privilegiado montando `mount -t cifs`, não o loopback do app):

| Cliente↔servidor | Resultado SQLite (CREATE num banco novo, sem concorrência) |
|---|---|
| kernel CIFS → Samba `dperson` 4.12, SMB2/3 default | ❌ `database is locked` |
| kernel CIFS → `dockurr/samba` 4.23 (vfs `fruit`/Time Machine) | ❌ `database is locked` |
| kernel CIFS → **Samba 4.24.6 moderno, `smb3 unix extensions = yes` + mount `posix`** | ❌ `database is locked` |
| kernel CIFS → **Samba 4.24.6 + `vers=1.0,unix` (unix extensions SMB1)** | ✅ funciona |

Conclusão técnica: **SQLite não funciona sobre o client CIFS do Linux com SMB2/3 — nem o modo WAL nem o rollback-journal**, independentemente da configuração do servidor (SMB3 POSIX extensions incluídas). A primeira escrita (`CREATE TABLE`) já falha com `database is locked` porque o locking do SQLite (fcncll por ranges múltiplos no arquivo) não é satisfeito pelo transporte SMB2/3. Com SMB1 + unix extensions os locks mapeiam para fcntl POSIX no servidor e **toda a suíte de estresse passa**:

| workers | writes | busy | lock | other | rows | wall_s | integrity | seq OK |
|--------:|-------:|-----:|-----:|------:|-----:|-------:|-----------|:------:|
| 2  | 2400 | 0 | 0 | 0 | 2400 | 2.63  | ok | ✓ |
| 4  | 2400 | 0 | 0 | 0 | 2400 | 6.93  | ok | ✓ |
| 8  | 2400 | 0 | 0 | 0 | 2400 | 10.51 | ok | ✓ |
| 16 | 2400 | 0 | 0 | 0 | 2400 | 7.38  | ok | ✓ |
| 32 | 2400 | 0 | 0 | 0 | 2400 | 8.91  | ok | ✓ |
| 64 | 2368 | 0 | 0 | 0 | 2368 | 10.50 | ok | ✓ |

Caminho degradado no share SMB1: 379 BUSY limpos, zero `other`, integridade ok, delta da sequência íntegro (`STRESS_WRITES` permite reduzir o volume da matriz em runs de rede: `STRESS_WRITES=300` <share>`).

**Por que não há fallback para rollback-journal:** chegou a ser implementada uma adaptação "share → `journal_mode=DELETE` + `synchronous=FULL`" (baseada em recomendação antiga dos docs do SQLite), mas a validação real mostrou que ela **trava** no único transporte de rede em que o SQLite funciona: com `DELETE`+`FULL` sobre SMB1+unix o processo ficou preso na primeira transação (arquivo `-journal` congelado, banco parado >15 min, nenhuma linha da matriz impressa), enquanto **WAL + NORMAL passa a matriz completa em segundos** no mesmo share. Conclusão: o app mantém WAL/NORMAL em todos os filesystems e usa o probe apenas para **diagnosticar** transporte inviável, nunca para mudar o modo do banco.

Leitura:
- **Clientes Windows/macOS** usam byte-range locks nativos do próprio SMB stack (SMB2 LOCK), o transporte que o SQLite precisa — é o cenário comum onde SQLite em share funciona. **Não validado aqui** (sem host Windows/mac), continua no critério de aceite real.
- **Clientes Linux com CIFS SMB2/3** (default dos sistemas atuais): **inviável** para o banco compartilhado — falha na primeira escrita, antes de qualquer dado. SMB1+unix é forçável mas é protocolo legado desligado por padrão (segurança), não recomendado.
- **NFS (Linux↔Linux)**: na teoria usa locking POSIX real, mas **não foi revalidado nesta rodada** (o mount NFS do testbed não completou após o rebuild dos containers; validações anteriores ficaram parciais). Client Linux → NFS com `lockd` é o único transporte "pasta compartilhada" viável além de SMB1+unix; revalidar com `scripts/validate-lan-shared-db.sh` antes de depender dele em produção.

### Decisão (gate)

1. Frota **toda Linux** → banco em pasta compartilhada por SMB2/3 **não funciona**; adotar a **abordagem B (servidor embutido)** da spec.
2. Frota com **clientes Windows/macOS** (ou mix em que o banco é servido por share) → validar com `scripts/validate-lan-shared-db.sh` num share real; se o probe de compatibilidade falhar, mesma conclusão da abordagem B.
3. O script `scripts/validate-lan-shared-db.sh <share>` reproduz a validação completa (probe SQLite, matriz 2–64, caminho degradado, drill de kill -9 com `integrity_check`) em qualquer share real.

## Pendente / validação obrigatória (critério de aceite do MVP)

**Atualizado após a validação em filesystem real:** o cenário exigido pela spec (máquinas Linux montando SMB2/3) é **inviável para SQLite** — a validação acima prova a falha no transporte de locks, não no código do app. Os itens de "2–3 máquinas" (1–7 abaixo) só fazem sentido num share com locking compatível:

1. 2–3 máquinas simultâneas no mesmo share com **locking compatível com SQLite** (Windows/macOS, ou SMB1+unix em Linux) — rodar `scripts/validate-lan-shared-db.sh`.
2. Leitura + escrita concorrente (OS, estoque, anexos).
3. Restart de uma máquina durante escrita de outra.
4. Queda de conexão/unmount no meio de transação (`-wal` órfão).
5. Criação concorrente de registros (sequência de OS continua única).
6. Migrations em primeiro startup simultâneo nas duas máquinas.
7. Recuperação após interrupção (`integrity_check`).

## Observações de roteiro/limitações conhecidas

- Legacy: `migrate_plaintext_database` (conversão de banco plaintext v0.1) ainda não é serializado entre processos no modo LAN; em uso normal o banco já é criptografado.
- Multi-escrita intensa em share não tem garantia de locking pelo SQLite; mitigação: WAL + `busy_timeout` + backups automáticos. A prova de carga acima mostra a degradação esperada (busy limpo, sem corrupção) até 64 escritores concorrentes no filesystem local, e a validação em SMB mostra que o **transporte SMB2/3 do client CIFS Linux não entrega os locks que o SQLite exige** (falha na primeira escrita, sem corrupção) — ver "Resultado da validação em filesystem de rede". Para multiusuário real em rede, a **abordagem B (servidor embutido)** descrita na spec é o caminho recomendado.
- `quick_check` no boot, docs de disaster recovery para rede, otimização de checkpoint e administração única de backup automático ficam para pós-MVP.