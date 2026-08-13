# Benchmark das otimizações

Medição executada em 12 de agosto de 2026 para validar o pacote de otimizações de conexão, listagens paginadas e relatório financeiro.

## Metodologia

- Build Rust `--release` com chave de dados efêmera usada apenas no processo do benchmark.
- Banco SQLCipher temporário, sem acesso ao banco real do usuário.
- 10.000 registros em cada entidade principal: clientes, funcionários, inventário, templates e ordens de serviço.
- 30.000 itens de template e 30.000 linhas de ordens de serviço.
- 5.000 ordens finalizadas no período financeiro medido.
- Página com 20 registros.
- Listagens: 3 aquecimentos e 15 amostras; conexão: 50 amostras.
- O baseline e o código otimizado são executados no mesmo processo, banco, conexão e cache.
- O tempo de criação da base sintética não faz parte das métricas.
- O payload corresponde ao JSON serializado que atravessaria o IPC.
- Para o baseline financeiro N+1 foi usada uma amostra: ela já executa 5.000 consultas adicionais e leva aproximadamente 40 segundos. O caminho otimizado teve 5 amostras.

Comando reproduzível:

```bash
OPETS_DATA_KEY_V1=<chave-hexadecimal-efemera-de-64-caracteres> \
  cargo test --release performance_benchmarks \
  -- --ignored --nocapture --test-threads=1
```

O benchmark está em `src-tauri/src/performance_benchmarks.rs` e permanece ignorado na suíte comum.

## Ambiente

- CPU: Intel Core i7-8700K, 6 núcleos / 12 threads, 3,70 GHz (turbo até 4,70 GHz).
- Sistema: Linux x86_64, kernel 6.18.22-1-lts.
- Rust/Cargo: 1.93.1.
- Dataset local criptografado com SQLCipher.

## Resultados

### Conexão

| Caminho | Mediana | p95 | Melhoria aproximada |
|---|---:|---:|---:|
| Reabrir SQLCipher por request | 4.513,3 µs | 5.065,3 µs | baseline |
| `get_db()` com conexão compartilhada | 0,8 µs | 0,8 µs | 5.642x |

A conexão compartilhada removeu aproximadamente 99,98% do custo fixo medido de aquisição.

### Listagens e telas

| Cenário | Baseline mediana | Otimizado mediana | Speedup | Payload baseline | Payload otimizado | Redução |
|---|---:|---:|---:|---:|---:|---:|
| Endpoint de clientes | 11,37 ms | 1,43 ms | 7,95x | 1.958.891 B | 3.945 B | 99,8% |
| Endpoint de funcionários | 12,67 ms | 1,55 ms | 8,19x | 2.100.001 B | 4.225 B | 99,8% |
| Endpoint de templates | 21,96 ms | 12,00 ms | 1,83x | 1.640.001 B | 3.305 B | 99,8% |
| Tela de inventário | 11,99 ms | 5,80 ms | 2,07x | 3.211.501 B | 13.034 B | 99,6% |
| Endpoint de ordens | 67,66 ms | 28,15 ms | 2,40x | 4.262.231 B | 8.370 B | 99,8% |
| Tela de ordens com selects | 88,88 ms | 52,63 ms | 1,69x | 8.321.127 B | 4.067.266 B | 51,1% |

Todos os endpoints paginados ficaram com p95 abaixo da meta de 100 ms. A redução de payload ficou entre 99,6% e 99,8%, exceto na tela completa de ordens.

### Relatório financeiro

| Cenário | Baseline | Otimizado | Speedup |
|---|---:|---:|---:|
| Filtro por data | 8,86 ms | 0,216 ms | 41,01x |
| Custo do resumo (`N+1` versus `JOIN/GROUP BY`) | 39,81 s | 78,68 ms | 506,02x |

O `EXPLAIN QUERY PLAN` confirmou a mudança:

```text
Antigo: SCAN so
Novo: SEARCH so USING COVERING INDEX idx_service_orders_finalized
      (status=? AND deleted_at=? AND finalized_date>? AND finalized_date<?)
```

Os resultados numéricos do resumo antigo e do otimizado foram comparados antes da medição e permaneceram equivalentes.

## Avaliação das metas

| Meta | Resultado |
|---|---|
| Reduzir custo fixo da conexão em mais de 90% | Atingida: aproximadamente 99,98% |
| Reduzir payload de páginas de 20 itens em aproximadamente 99% | Atingida nos endpoints: 99,6% a 99,8% |
| Reduzir mediana das listagens em pelo menos 70% | Atingida em clientes e funcionários; ganho menor em templates, inventário e ordens devido a contagens/joins auxiliares |
| p95 das listagens abaixo de 100 ms | Atingida em todos os cenários medidos |
| Eliminar crescimento N+1 do resumo financeiro | Atingida: 506x no dataset de 10 mil OS |

## Gargalos restantes

### Relatório financeiro completo

O relatório completo atual levou aproximadamente **66,61 segundos** para 10 mil ordens. A causa principal provável é a métrica de clientes recorrentes em `financial_report_repo.rs`:

```sql
EXISTS (
  SELECT 1
  FROM service_orders previous
  WHERE previous.customer_id = so.customer_id
    AND previous.deleted_at IS NULL
    AND date(previous.created_at, 'localtime') < date(?1)
)
```

Ela continua correlacionada por ordem, aplica função à coluna e não possui índice composto adequado por cliente/data. O próximo ajuste recomendado é usar `created_date` também nessa subconsulta, criar índice como `(customer_id, deleted_at, created_date)` e medir novamente o relatório completo.

### Tela de ordens

O endpoint paginado de ordens reduziu o payload em 99,8%, mas a tela completa reduziu apenas 51,1%. Os filtros ainda carregam todos os 10 mil clientes e funcionários. Recomenda-se transformar `SearchableSelect` em busca remota paginada.

### Templates

A página retorna apenas 20 templates, mas a consulta dos itens usa `WHERE template_id IN (...)` sem índice versionado para `template_items(template_id)`. Isso explica parte do ganho menor (1,83x). Recomenda-se adicionar o índice e repetir a medição.

### Busca e páginas profundas

- `LIKE '%termo%'` continua exigindo varredura e não aproveita índices B-tree comuns.
- `LIMIT/OFFSET` pode degradar em páginas muito profundas; o dataset atual ainda ficou abaixo de 100 ms.
- Se essas operações crescerem, considerar FTS para busca e keyset pagination para navegação profunda.

## Conclusão

O pacote entregue produziu a melhoria esperada para conexão, volume transferido, listagens e os hot spots financeiros que foram explicitamente otimizados. A medição também revelou que o relatório financeiro completo ainda possui uma consulta operacional correlacionada que domina seu tempo total e deve ser a próxima prioridade de performance.
