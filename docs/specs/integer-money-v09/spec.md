# Especificacao: valores monetarios inteiros e TCC v09

## Contexto e objetivo

O produto armazena valores monetarios como SQLite `REAL` e os manipula como `f64`/`number`. O painel e o relatorio avancado tambem denominam como lucro liquido um valor calculado apenas pela receita descontada menos os custos registrados nos itens das ordens finalizadas. A revisao deve tornar os calculos exatos em centavos, corrigir essa terminologia em todas as saidas e atualizar o TCC com evidencias reproduziveis.

## Cenario atual

- Banco, modelos, relatorios, CSV e PDF usam ponto flutuante para dinheiro.
- O painel e o relatorio avancado exibem `Lucro Liquido`.
- O calculo atual nao considera tributos, despesas operacionais ou financeiras.
- Bancos e backups publicados precisam continuar restauraveis.
- A v07 registra 132 testes Rust regulares, um teste real de PDF separado e 13 testes da interface.

## Requisitos funcionais

- Armazenar precos, custos, totais, receitas e resultados em centavos inteiros.
- Representar descontos em pontos-base inteiros.
- Migrar valores legados com arredondamento explicito e transacional.
- Migrar backups antigos em area temporaria antes da ativacao.
- Calcular descontos e agregacoes sem ponto flutuante monetario.
- Exibir `Lucro bruto estimado` no painel, relatorio avancado, CSV e PDF.
- Usar a mesma formula no painel e no relatorio avancado.
- Atualizar a captura do painel somente depois da correcao do produto.
- Criar a v09 a partir da v07 ativa, sem alterar versoes historicas.
- Manter o PDF da v09 em ate 20 paginas.

## Requisitos nao funcionais

- Falhar sem substituir dados ativos quando uma migracao for invalida.
- Usar aritmetica verificada em operacoes que possam exceder `i64`.
- Manter valores enviados ao frontend dentro do intervalo de inteiros seguros do JavaScript.
- Preservar IDs, relacionamentos, historico e exclusoes logicas.
- Registrar comandos, contagens e cobertura usados como evidencia da v09.

## Interfaces afetadas

- SQLite: novas colunas `*_cents` e `discount_basis_points`.
- Rust/Tauri: valores monetarios serializados como inteiros.
- Frontend: formatacao e entrada convertem diretamente entre texto BRL e centavos.
- Dashboard: `estimatedGrossProfit` substitui `netProfit`.
- Relatorio avancado: resumo, graficos, agrupamentos, CSV e PDF usam a mesma nomenclatura.

## Estrategia de testes

- Testes unitarios de arredondamento, desconto, formatacao e limites.
- Testes de migracao de esquemas e backups legados.
- Testes de repositorio com igualdade exata em centavos.
- Testes IPC para confirmar inteiros e os novos nomes do indicador.
- Testes frontend para entradas BRL, totais e descontos.
- Execucao integral de lint, tipos, build, testes, cobertura e PDF real.

## Fora de escopo

- Contabilidade completa com tributos, aluguel, folha ou despesas financeiras.
- Autenticacao, sincronizacao e portal do cliente.
- Publicacao de commit ou tag sem autorizacao explicita posterior.

## Riscos e premissas

- Colunas `REAL` legadas serao preservadas temporariamente apenas como origem de migracao; o codigo operacional nao as atualizara nem consultara.
- Bancos antigos podem conter mais de duas casas decimais e serao convertidos para centavos com arredondamento de metade para longe de zero.
- A quantidade final de testes e a cobertura somente serao inseridas no TCC depois de nova execucao.
