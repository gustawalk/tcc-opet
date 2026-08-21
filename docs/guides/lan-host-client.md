# Banco compartilhado na rede local

O modo LAN permite que até cinco computadores usem o mesmo conjunto de dados dentro da rede local. Um computador Host mantém o banco SQLite/SQLCipher e os anexos; os computadores Cliente usam uma API HTTPS do aplicativo. Não há pasta de rede, acesso direto ao arquivo SQLite, serviço de nuvem ou dependência de internet.

## Modos

- **Local**: usa somente o banco deste computador. É o comportamento padrão e compatível com versões anteriores.
- **Host**: usa o banco local existente e abre o servidor do OpetS na rede local.
- **Cliente**: não abre, cria ou migra um banco de produção local. Todas as operações compartilhadas são enviadas ao Host.

Trocar de modo reinicia o aplicativo. Isso impede que uma mesma execução misture conexões locais e remotas.

## Configurar o Host

1. Abra **Configurações > Rede local** no computador que já contém os dados oficiais.
2. Escolha **Host**, mantenha a porta `8743` ou informe outra porta local livre e clique em **Salvar modo e reiniciar**.
3. Após reiniciar, confirme o estado **Ativo** e anote o endereço HTTPS mostrado.
4. Se o sistema operacional pedir autorização de firewall, permita o aplicativo somente em redes privadas.
5. Envie ao funcionário o endereço HTTPS e o código de verificação exibido. O código expira e pode ser regenerado.

Não abra ou encaminhe essa porta no roteador de internet. O Host precisa apenas aceitar conexões na LAN. Redes Wi-Fi com isolamento de clientes impedem a conexão entre os computadores e precisam ser ajustadas pelo responsável da rede.

O Host deve permanecer ligado e com o aplicativo aberto. Se ele desligar, os Clientes não conseguem ler nem alterar dados.

## Parear um Cliente

1. No computador do funcionário, abra **Configurações > Rede local** e escolha **Cliente**.
2. Informe o endereço HTTPS do Host, um nome para o computador e o código de verificação recebido.
3. Clique em **Parear e reiniciar**.
4. Após reiniciar, confirme o estado **Conectado** e a impressão digital do certificado fixado.

O código combina um segredo de uso único com a impressão digital TLS. O Cliente baixa somente o certificado público antes da validação; o código de pareamento e o token do dispositivo só são enviados depois que a impressão digital confere. Host e Cliente também devem executar exatamente a mesma versão do aplicativo.

Se o certificado do Host mudar, a versão for diferente, o token for revogado ou o Host ficar indisponível, o Cliente falha de forma fechada: não consulta nem grava no banco local anterior. Para voltar ao banco próprio, selecione **Local** e reinicie.

## Dispositivos e revogação

O Host lista os computadores pareados, versão, último acesso e estado de revogação. Use **Revogar** para remover um funcionário. A próxima chamada autenticada desse Cliente é recusada; novo acesso exige outro pareamento.

Os tokens brutos ficam somente no arquivo privado de configuração do Cliente. O Host armazena apenas a impressão BLAKE3 de cada token. A chave TLS privada permanece somente no diretório privado do Host.

## Backup e manutenção

- Host e Cliente podem exportar um backup manual.
- No Cliente, o Host cria o pacote criptografado e o transmite por HTTPS; o Cliente salva o arquivo escolhido sem acessar `database.db` diretamente.
- O Cliente pode configurar downloads automáticos locais. A criação continua ocorrendo no Host e respeita o bloqueio exclusivo de armazenamento.
- Importação, restauração, reset e configuração do backup oficial existem somente no Host.
- Um backup no Cliente é uma cópia para recuperação. Ele não transforma o Cliente em réplica gravável e não permite operação quando o Host está desligado.

## Segurança e limites

- Todo tráfego de negócio usa TLS com certificado fixado, token Bearer por dispositivo, versão exata e chaves de idempotência para alterações.
- A API aceita operações de produto conhecidas; não aceita SQL arbitrário.
- O banco, WAL e anexos nunca são compartilhados por SMB/NFS.
- A porta fica visível para a LAN. A autenticação e o TLS protegem os dados, mas a rede privada e o firewall do sistema operacional continuam fazendo parte da segurança.
- Uma transação de ordem, estoque, checklist e anexos é executada integralmente no banco do Host.

## Referência para manutenção

As tabelas centrais atuais incluem `customers`, `users`, `inventory_items`, `inventory_movements`, `service_orders`, `service_order_parts`, `service_order_checklist`, `service_order_attachments`, `service_order_events`, `settings`, `lan_devices` e `lan_idempotency_records`.

Sharding não faz parte deste modo. Possíveis unidades futuras seriam clientes/ordens por empresa ou faixas de entidades, mas hoje os seguintes contratos exigem uma visão única:

- estoque global e baixa atômica durante a criação da ordem;
- identificador sequencial de exibição;
- dashboard e relatório financeiro agregados;
- configurações únicas da empresa;
- backup consistente do banco e de todos os anexos;
- transições e cancelamentos que restauram estoque na mesma transação.

Qualquer sharding futuro precisa definir propriedade da chave, migração de partições, consultas globais, identificadores e transações entre partições. Até essas regras existirem e a carga real superar um único Host para cinco usuários, o Host continua sendo a única autoridade de escrita.
