# Plano de implementacao: valores monetarios inteiros e TCC v09

## Fases

1. Registrar a baseline de testes e contratos.
2. Criar utilitarios monetarios e migracao aditiva para centavos.
3. Converter modelos, repositorios, comandos, seeds, CSV e PDF.
4. Converter contratos, formularios, paginas, painel e relatorio avancado.
5. Atualizar testes e validar bancos e backups legados.
6. Executar a verificacao completa e gerar uma nova captura.
7. Criar, revisar, renderizar e arquivar a v09.
8. Preparar a rastreabilidade para commit e tag posteriores.

## Criterios de conclusao

- Nenhum calculo monetario de producao usa `f64`.
- Painel e relatorio avancado exibem `Lucro bruto estimado`.
- Banco antigo e backup antigo sao convertidos sem perda de dados.
- Dashboard, relatorio, CSV e PDF reconciliam exatamente em centavos.
- Todos os testes, lint, tipos e builds sao aprovados.
- O TCC usa os numeros efetivamente medidos e possui ate 20 paginas A4.
