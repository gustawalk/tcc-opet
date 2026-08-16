# Recuperação de desastre

## Preparação

- Confirme periodicamente que `Última verificação` está recente.
- Confirme no cliente de nuvem ou NAS que o `.osbkp` foi sincronizado.
- Mantenha pelo menos uma cópia fora do computador principal.
- Teste a restauração em uma instalação separada antes de depender do procedimento em produção.

## Restaurar

1. Instale uma versão do OpetS igual ou mais recente que a versão que criou o backup.
2. Copie o `.osbkp` para um disco local estável se ele estiver em rede ou nuvem.
3. Abra `Configurações`.
4. Clique em `Importar Backup`.
5. Selecione o arquivo.
6. Confirme a substituição dos dados atuais.
7. Aguarde a conclusão sem fechar o aplicativo.
8. Valide empresa, clientes, estoque, ordens de serviço, anexos e relatório financeiro.
9. Em uma instalação nova, configure a pasta de backup automático. No mesmo dispositivo, confirme que a configuração local preservada continua correta.

## Falhas comuns

`Não foi possível autenticar o backup`: o arquivo está corrompido, foi alterado ou exige outra senha/chave. Tente a geração anterior.

`Destino indisponível ou substituído`: reconecte a unidade ou selecione novamente a pasta correta. O marcador impede gravar silenciosamente em outra montagem.

`Espaço insuficiente`: libere espaço no destino e no disco do aplicativo. Não apague o único backup válido.

`Arquivo em uso` no Windows: feche ferramentas externas que abriram o `.osbkp`. A conexão SQLCipher do OpetS é fechada automaticamente antes da ativação da restauração.

## Validação pós-recuperação

- Abra ordens antigas e recentes.
- Abra pelo menos um anexo.
- Confira quantidades e valores de estoque.
- Gere um relatório financeiro de período conhecido.
- Crie uma nova ordem de teste e remova-a conforme o procedimento operacional.
- Execute um novo backup e confirme `Última verificação` sem erro.

Se nenhum backup for aceito, preserve todos os arquivos e o diretório de dados original antes de qualquer tentativa manual. Não abra ou edite o banco SQLCipher com ferramentas SQLite comuns.
