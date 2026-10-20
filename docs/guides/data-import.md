# Importar dados de outro sistema

Nas páginas **Clientes**, **Funcionários** e **Estoque**, clique em **Importar dados**. A importação cria novos cadastros. Ela não atualiza ou substitui registros existentes.

## Preparar o arquivo

Use CSV em UTF-8 (com ou sem BOM), separado por vírgula, ponto e vírgula ou tabulação, ou uma planilha XLSX sem senha. A primeira linha não vazia deve conter os títulos das colunas. Cada importação lê uma única planilha do XLSX. Linhas completamente vazias são ignoradas.

Os limites são 5.000 linhas de dados, 100 colunas, 4.096 caracteres por célula e 10 MiB por arquivo. O conteúdo descompactado de um XLSX não pode ultrapassar 64 MiB. Os dados associados aos campos do aplicativo não podem ultrapassar 8 MiB; divida arquivos maiores em várias importações.

Fórmulas e células com erros precisam ser substituídas por valores na revisão, ou suas linhas precisam ser excluídas. O aplicativo não calcula fórmulas nem usa seu resultado armazenado. Guarde telefones e CPF como texto no arquivo: zeros iniciais já perdidos em células numéricas não podem ser recuperados.

## Associar as colunas

1. Selecione o arquivo. Confira a planilha do XLSX ou o separador do CSV e as primeiras linhas exibidas.
2. Clique em **Mapear campos**. Para cada campo do aplicativo, escolha uma coluna, um valor fixo para todas as linhas ou deixe em branco.
3. Confira as sugestões automáticas. Se existirem duas colunas com títulos semelhantes, a associação fica para sua escolha. Os números das colunas distinguem títulos repetidos.
4. No estoque, escolha o formato dos números: brasileiro (`1.234,56`) ou internacional (`1,234.56`). Células numéricas do Excel usam seu valor numérico. Valores digitados na revisão seguem o formato escolhido.

Colunas não associadas são ignoradas. Dados como data de nascimento, contato de emergência e campos personalizados não possuem destino nesta versão. O mapeamento não é salvo para a próxima importação.

| Cadastro | Campos disponíveis |
| --- | --- |
| Clientes | Nome completo, telefone, e-mail, endereço |
| Funcionários | Nome completo, e-mail, telefone, CPF, data de admissão |
| Estoque | Nome, descrição, tipo, controle de estoque, quantidade mínima, quantidade inicial, preço de custo, preço de venda, fornecedor |

Nome e telefone são obrigatórios para clientes; nome e e-mail, para funcionários. E-mails de funcionários devem ser únicos, inclusive em relação a cadastros excluídos. Telefone e CPF opcionais dos funcionários seguem as regras do cadastro normal. Datas de admissão aceitam `DD/MM/AAAA`, `AAAA-MM-DD` ou células de data do Excel.

No estoque, nome, descrição e tipo são obrigatórios. O tipo inicia em peça (`part`); também são aceitos serviço (`service`) e item (`item`). Quantidades e preços ausentes iniciam em zero. Preços devem ter até duas casas decimais e não podem ser negativos. Peças sempre controlam estoque; serviços não controlam. Itens genéricos iniciam sem controle, mas podem usar `sim`/`true` no campo correspondente. Cadastros sem controle não podem receber quantidades positivas.

## Revisar e confirmar

Clique em **Revisar dados**. Corrija os campos na tabela e use a paginação para navegar. O número de cada linha corresponde à sua posição no arquivo. As edições permanecem ao trocar de página. Use **Somente linhas com erros** para localizar pendências.

Desmarque **Incluir** para excluir uma linha; marque novamente para restaurá-la. Todos os registros incluídos precisam estar válidos antes de continuar. Em caso de indisponibilidade do Host, **Tentar validar novamente** repete a validação sem perder a revisão.

Confira os totais e clique em **Confirmar importação**. Todos os registros incluídos são gravados juntos. Se a validação ou a gravação falhar, o lote inteiro é desfeito. Se surgir um conflito desde a revisão, o aplicativo volta aos campos que precisam de correção. Dados repetidos de clientes e estoque podem criar cadastros duplicados.

Quantidades iniciais de estoque controlado geram uma entrada com o motivo **Importação de dados**, custo unitário importado e nenhum vínculo com ordem de serviço. O custo médio inicial corresponde ao preço de custo.

Se a resposta da gravação não chegar, use **Tentar novamente**. O aplicativo mantém os mesmos dados e o identificador do lote para recuperar o resultado sem criar outros registros. Aguarde uma confirmação antes de iniciar outra importação. A revisão fica apenas na memória; reiniciar o aplicativo perde o rascunho. Uma nova importação do mesmo arquivo é um novo lote e pode criar registros repetidos.

No modo Cliente LAN, selecione um arquivo do próprio computador. A validação e a gravação acontecem no Host autenticado. A indisponibilidade do Host não faz o aplicativo gravar no banco local.

Trocar arquivo, planilha ou mapeamento e cancelar uma revisão exige confirmação de descarte. Após concluir, o aplicativo apresenta quantos registros foram criados e quantas linhas foram excluídas, e atualiza as listas e os resumos relacionados.
