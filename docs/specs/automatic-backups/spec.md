# Backup automático

## Objetivo

Criar pontos de recuperação autenticados sem depender de uma ação diária do usuário, com baixo consumo de CPU, memória e disco e sem colocar o banco SQLite ativo em uma pasta sincronizada.

## Comportamento

- O recurso fica desativado por padrão.
- O usuário escolhe uma pasta local, sincronizada, de rede ou unidade removível.
- O intervalo aceita valores de 1 a 168 horas e usa 24 horas por padrão.
- O backend verifica o agendamento ao abrir o aplicativo e depois a cada hora.
- Ativar o recurso ou alterar pasta/intervalo agenda o backup para a próxima verificação, sem executá-lo dentro do salvamento da configuração.
- A execução manual ignora apenas a data agendada; dados inalterados continuam sem gerar arquivo duplicado.
- A execução ocorre somente enquanto o OpetS está aberto.
- Um backup só é registrado como sucesso depois da autenticação do envelope, validação do ZIP, migrações em staging, `integrity_check`, schema e chaves estrangeiras.

## Retenção

- Manter o ponto mais recente de sete datas distintas.
- Manter um ponto por semana ISO em quatro semanas anteriores à janela diária.
- Manter sempre os dois arquivos mais recentes quando existirem.
- Proteger incondicionalmente o arquivo recém-criado durante a limpeza, inclusive se o relógio do sistema retroceder.
- Remover somente arquivos automáticos pertencentes à instalação atual.
- Não remover backups manuais, arquivos desconhecidos, links simbólicos ou backups de outra instalação.
- Executar a limpeza somente depois de criar e validar um novo backup.

## Segurança e consistência

- O formato permanece `.osbkp` versão 3.
- Backups automáticos usam a chave `OPETS_DATA_KEY_V1`; nenhuma senha é persistida.
- A pasta contém `.opets-backup-destination.json`, usado para detectar unidade ausente ou substituída.
- Cada instalação possui um `sourceId` próprio, usado no nome e na retenção dos arquivos.
- A configuração operacional fica em `database.automatic-backup.json`, fora do banco restaurável.
- A configuração é gravada com `sync_all`, arquivo temporário, versão anterior recuperável e rename.
- O destino não pode ser o diretório de anexos nem um descendente dele.
- O banco e os anexos anteriores nunca são modificados durante exportação ou validação.

## Otimizações

- Um fingerprint BLAKE3 do banco e dos anexos referenciados evita exportação quando a fonte não mudou.
- Sem mudanças, tamanho e hash do backup são conferidos em toda execução; a validação completa de banco é repetida semanalmente.
- O snapshot SQLCipher é criado durante um lock exclusivo curto.
- Anexos imutáveis e referenciados pelo snapshot são preparados por hard link; cópia é apenas fallback.
- Compressão ZIP usa `Stored`, pois páginas SQLCipher e anexos autenticados são dados de alta entropia e não comprimem de forma útil.
- XChaCha20-Poly1305 criptografa e descriptografa no próprio buffer, evitando múltiplas cópias do arquivo completo em memória.
- Temporários são removidos em todos os retornos controlados.
- O sistema verifica espaço livre antes da execução e reserva margem para staging e ativação.

## Limites

- Arquivo final: 250 MiB.
- Snapshot do banco: 100 MiB.
- Anexo armazenado: 10 MiB mais envelope criptográfico.
- Entradas de anexos: 10.000.
- A exportação rejeita antecipadamente banco ou anexo acima do limite e rejeita o arquivo final antes da ativação.

## RPO

O objetivo padrão é RPO de 24 horas. Essa garantia depende de o aplicativo permanecer aberto no momento de uma verificação e de a pasta escolhida estar disponível e com espaço livre. Intervalos acima de 24 horas exibem aviso explícito.

## Critérios de aceite

- Uma execução cria um `.osbkp` autenticado e restaurável.
- Uma segunda execução sem mudanças valida o arquivo atual e não cria outro.
- Uma mudança no banco cria novo ponto.
- Um backup retido corrompido é detectado e substituído por uma nova cópia válida.
- Falha de exportação ou validação não avança `lastSuccessAt`.
- O banco fica bloqueado somente durante fingerprint, snapshot e preparação das referências de anexos.
- Trocas repetidas de configuração funcionam no Windows.
- Duas instalações na mesma pasta não removem os arquivos uma da outra.
- O modal global impede interação durante uma execução e desaparece mesmo se a tela de Configurações não estiver aberta.
