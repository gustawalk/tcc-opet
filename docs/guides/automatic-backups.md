# Guia de backup automático

## Configurar

1. Abra `Configurações`.
2. Localize `Banco de Dados & Backup`.
3. Clique em `Selecionar pasta`.
4. Escolha uma pasta sincronizada, de rede ou unidade externa.
5. Defina o intervalo entre 1 e 168 horas. Recomenda-se 24 horas.
6. Marque `Backup automático`.
7. Clique em `Salvar automático`.

O primeiro arquivo será criado na próxima verificação do scheduler. `Executar agora` antecipa essa verificação, mas não duplica um backup quando os dados permanecem iguais.

## Escolher o destino

Destinos recomendados:

- Pasta do OneDrive, Google Drive para desktop ou Dropbox.
- Compartilhamento de rede ou NAS estável.
- SSD ou pendrive mantido conectado durante o expediente.

Não use:

- A pasta onde está o banco ativo.
- A pasta interna de anexos do OpetS.
- Uma unidade quase cheia ou removida com frequência.

O aplicativo produz arquivos completos e imutáveis. A confirmação de que um serviço de nuvem terminou o upload continua sendo responsabilidade do cliente de sincronização.

## Acompanhar

A tela mostra último backup, última validação, próxima data elegível, tamanho e último erro. Durante a operação aparece o modal `Backup automático em andamento, aguarde alguns instantes.`

Quando nenhum dado mudou, o arquivo anterior é validado e nenhum espaço adicional é utilizado.

## Retenção e espaço

O OpetS mantém sete pontos diários e quatro semanais. Backups manuais não entram nessa limpeza. Cada instalação usa um namespace próprio, mesmo quando duas máquinas apontam para a mesma pasta.

Para minimizar espaço e tempo:

- Mantenha o intervalo padrão de 24 horas.
- Evite clicar repetidamente em `Executar agora`; o sistema não duplica dados iguais, mas ainda precisa validar o arquivo.
- Corrija alertas de retenção ou falta de espaço assim que aparecerem.
- Não renomeie o marcador `.opets-backup-destination.json`.
