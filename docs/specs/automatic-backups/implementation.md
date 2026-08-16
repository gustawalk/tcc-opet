# Implementação do backup automático

## Componentes

| Componente | Responsabilidade |
|---|---|
| `automatic_backup.rs` | Configuração, scheduler, destino, progresso, fingerprint, retenção e estado operacional |
| `backup_service.rs` | Snapshot, staging, ZIP, criptografia, validação e restauração |
| `settings_commands.rs` | Contratos IPC e seleção nativa de pasta |
| `AutomaticBackupProgress.tsx` | Modal global orientado por evento e recuperação de status |
| `Settings.tsx` | Ativação, destino, intervalo, execução manual e diagnóstico |

## Fluxo de execução

1. O scheduler carrega o sidecar e aguarda em uma condition variable até a data elegível, uma alteração de configuração ou o encerramento.
2. O marcador confirma que o mesmo destino continua montado.
3. O coordenador adquire o lock exclusivo do armazenamento.
4. O fingerprint BLAKE3 compara banco e anexos referenciados com o último estado.
5. Se não houve mudança, o lock é liberado, tamanho e hash do último backup são conferidos e a validação integral é repetida quando a última tiver sete dias.
6. Se houve mudança, o banco é copiado pela API de backup do SQLite e os anexos referenciados são preparados em staging.
7. O lock é liberado antes de escrever ZIP, criptografar e validar.
8. O arquivo final é ativado sem sobrescrever nomes existentes e somente após `sync_all`.
9. A validação extrai somente o banco; anexos permanecem no arquivo autenticado.
10. A retenção elimina apenas arquivos registrados como pertencentes à instalação atual e nunca infere propriedade somente pelo nome.
11. O sidecar registra sucesso, tamanho, fingerprint, próxima execução e eventual aviso de retenção.

## Eventos

Evento Tauri: `automatic-backup-progress`.

Payload:

```json
{
  "running": true,
  "percent": 30,
  "phase": "exporting",
  "message": "Empacotando banco e anexos."
}
```

Fases estimadas: `preparing`, `checking`, `snapshot`, `exporting`, `validating`, `retention`, `completed`, `unchanged` e `failed`.

## Estado persistido

O sidecar contém versão, opções e estado. Campos operacionais incluem `sourceId`, `destinationId`, `nextBackupAt`, `lastAttemptAt`, `lastSuccessAt`, `lastVerifiedAt`, `lastFullValidationAt`, `lastError`, `lastBackupPath`, `lastBackupSizeBytes`, `sourceFingerprint` e o registro dos arquivos pertencentes à instalação.

O sidecar fica em `appData/automatic-backup`, não viaja dentro do backup e é migrado automaticamente da localização legada ao lado do banco. Restaurar dados de outro computador não troca automaticamente o destino local.

Nomes de arquivo (`opets-auto-*`) usam horário local, apenas para leitura humana do diretório; os timestamps de estado (`createdAt`, `nextBackupAt` etc.) ficam em RFC3339 UTC e a propriedade nunca é inferida pelo nome.

## Falhas

- Destino ausente: registrar erro e tentar novamente após uma hora.
- Espaço insuficiente: falhar antes de criar arquivos grandes.
- Backup anterior corrompido: manter o arquivo até a nova cópia ser validada e então removê-lo.
- Falha na retenção: manter o novo backup válido e exibir o aviso no status.
- Falha na ativação: preservar ou restaurar o destino anterior.
- Encerramento abrupto: temporários têm nomes reservados e não entram na retenção.

## Validação

Comandos obrigatórios antes de release:

```bash
yarn typecheck
yarn lint
yarn test
yarn build
cd src-tauri
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --lib -- --test-threads=1
```

O job Windows deve validar repetidas gravações do sidecar, restauração e substituição de arquivos abertos.
